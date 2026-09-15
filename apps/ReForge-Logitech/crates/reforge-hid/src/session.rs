use crate::{open_summary_device, response_matches};
use hidapi::HidDevice;
use reforge_core::{DeviceSummary, HidppSessionState, HidppSessionStatus};
use reforge_protocol::{HidppRequest, HidppResponse};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HidppPacketKind {
    Reply,
    Notification,
    Error,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HidppNotification {
    pub key: String,
    pub device_index: u8,
    pub feature_index: u8,
    pub function_swid: u8,
    pub params: Vec<u8>,
}

pub fn classify_response(request: Option<&HidppRequest>, response: &HidppResponse) -> HidppPacketKind {
    if response.is_hidpp_error() {
        return if request.is_some_and(|request| response_matches(request, response)) {
            HidppPacketKind::Error
        } else {
            HidppPacketKind::Notification
        };
    }
    if request.is_some_and(|request| response_matches(request, response)) {
        HidppPacketKind::Reply
    } else {
        HidppPacketKind::Notification
    }
}

pub fn classify_packet(request: Option<&HidppRequest>, bytes: &[u8]) -> HidppPacketKind {
    HidppResponse::parse(bytes)
        .map(|response| classify_response(request, &response))
        .unwrap_or(HidppPacketKind::Unknown)
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug, Clone)]
struct SessionHealth {
    state: HidppSessionState,
    reconnect_count: u32,
    last_success_unix_ms: Option<u64>,
    last_error: Option<String>,
}

impl Default for SessionHealth {
    fn default() -> Self {
        Self {
            state: HidppSessionState::Connecting,
            reconnect_count: 0,
            last_success_unix_ms: None,
            last_error: None,
        }
    }
}

impl SessionHealth {
    fn ready(&mut self) {
        self.state = HidppSessionState::Ready;
        self.last_success_unix_ms = Some(unix_ms());
        self.last_error = None;
    }

    fn failed(&mut self, message: impl Into<String>) {
        self.state = HidppSessionState::Degraded;
        self.last_error = Some(message.into());
    }

    fn reconnecting(&mut self) {
        self.state = HidppSessionState::Connecting;
        self.reconnect_count = self.reconnect_count.saturating_add(1);
        self.last_error = None;
    }
}

struct PersistentSession {
    summary: Mutex<DeviceSummary>,
    handle: Mutex<HidDevice>,
    health: Mutex<SessionHealth>,
}

impl PersistentSession {
    fn open(device: &DeviceSummary, reconnect_count: u32) -> Result<Self, String> {
        let handle = open_summary_device(device)?;
        let mut health = SessionHealth {
            reconnect_count,
            ..Default::default()
        };
        health.ready();
        Ok(Self {
            summary: Mutex::new(device.clone()),
            handle: Mutex::new(handle),
            health: Mutex::new(health),
        })
    }

    fn update_summary(&self, device: &DeviceSummary) {
        if let Ok(mut summary) = self.summary.lock() {
            *summary = device.clone();
        }
    }

    fn status(&self) -> HidppSessionStatus {
        let summary = self.summary.lock().ok().map(|value| value.clone());
        let health = self.health.lock().ok().map(|value| value.clone()).unwrap_or_default();
        HidppSessionStatus {
            key: summary.as_ref().map(|value| value.key.clone()).unwrap_or_default(),
            path: summary.as_ref().map(|value| value.path.clone()).unwrap_or_default(),
            device_index: summary.as_ref().map(|value| value.device_index).unwrap_or(0xff),
            state: health.state,
            reconnect_count: health.reconnect_count,
            last_success_unix_ms: health.last_success_unix_ms,
            last_error: health.last_error,
        }
    }
}

#[derive(Default)]
pub struct SessionManager {
    sessions: Mutex<BTreeMap<String, Arc<PersistentSession>>>,
}

impl SessionManager {
    pub fn ensure(&self, device: &DeviceSummary) -> Result<HidppSessionStatus, String> {
        if !device.hidpp {
            return Err("selected device is not a HID++ device".into());
        }
        if let Some(existing) = self
            .sessions
            .lock()
            .map_err(|_| "HID++ session registry lock is poisoned".to_owned())?
            .get(&device.key)
            .cloned()
        {
            let status = existing.status();
            if status.path == device.path && status.device_index == device.device_index {
                existing.update_summary(device);
                return Ok(existing.status());
            }
        }
        self.reconnect(device)
    }

    pub fn reconnect(&self, device: &DeviceSummary) -> Result<HidppSessionStatus, String> {
        let (old_count, existed) = {
            let sessions = self
                .sessions
                .lock()
                .map_err(|_| "HID++ session registry lock is poisoned".to_owned())?;
            match sessions.get(&device.key) {
                Some(session) => (session.status().reconnect_count, true),
                None => (0, false),
            }
        };
        let reconnect_count = old_count.saturating_add(u32::from(existed));
        let session = Arc::new(PersistentSession::open(device, reconnect_count)?);
        let status = session.status();
        self.sessions
            .lock()
            .map_err(|_| "HID++ session registry lock is poisoned".to_owned())?
            .insert(device.key.clone(), session);
        Ok(status)
    }

    pub fn remove(&self, key: &str) {
        if let Ok(mut sessions) = self.sessions.lock() {
            sessions.remove(key);
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        self.sessions
            .lock()
            .map(|sessions| sessions.contains_key(key))
            .unwrap_or(false)
    }

    pub fn status(&self, key: &str) -> Option<HidppSessionStatus> {
        self.sessions
            .lock()
            .ok()
            .and_then(|sessions| sessions.get(key).cloned())
            .map(|session| session.status())
    }

    pub fn status_all(&self) -> Vec<HidppSessionStatus> {
        self.sessions
            .lock()
            .map(|sessions| sessions.values().map(|session| session.status()).collect())
            .unwrap_or_default()
    }

    pub fn with_device<T>(
        &self,
        key: &str,
        operation: impl FnOnce(&HidDevice) -> Result<T, String>,
    ) -> Result<T, String> {
        let session = self
            .sessions
            .lock()
            .map_err(|_| "HID++ session registry lock is poisoned".to_owned())?
            .get(key)
            .cloned()
            .ok_or_else(|| format!("no persistent HID++ session for {key}"))?;
        let handle = session
            .handle
            .lock()
            .map_err(|_| format!("HID++ session lock is poisoned for {key}"))?;
        match operation(&handle) {
            Ok(value) => {
                if let Ok(mut health) = session.health.lock() {
                    health.ready();
                }
                Ok(value)
            }
            Err(error) => {
                if let Ok(mut health) = session.health.lock() {
                    health.failed(error.clone());
                }
                Err(error)
            }
        }
    }

    pub fn poll_notifications(&self) -> Vec<HidppNotification> {
        let sessions = self
            .sessions
            .lock()
            .map(|sessions| sessions.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        let mut notifications = Vec::new();
        for session in sessions {
            let Ok(handle) = session.handle.try_lock() else {
                continue;
            };
            let summary = session.summary.lock().ok().map(|value| value.clone());
            let Some(summary) = summary else {
                continue;
            };
            for _ in 0..8 {
                let mut buffer = [0_u8; 64];
                let read = match handle.read_timeout(&mut buffer, 0) {
                    Ok(read) => read,
                    Err(error) => {
                        if let Ok(mut health) = session.health.lock() {
                            health.failed(format!("HID++ notification read failed: {error}"));
                        }
                        break;
                    }
                };
                if read == 0 {
                    break;
                }
                let Ok(response) = HidppResponse::parse(&buffer[..read]) else {
                    continue;
                };
                if classify_response(None, &response) != HidppPacketKind::Notification {
                    continue;
                }
                if let Ok(mut health) = session.health.lock() {
                    health.ready();
                }
                notifications.push(HidppNotification {
                    key: summary.key.clone(),
                    device_index: response.device_index,
                    feature_index: response.feature_index,
                    function_swid: response.function_swid,
                    params: response.params,
                });
            }
        }
        notifications
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_state_tracks_failure_and_reconnect() {
        let mut health = SessionHealth::default();
        health.ready();
        assert_eq!(health.state, HidppSessionState::Ready);
        health.failed("offline");
        assert_eq!(health.state, HidppSessionState::Degraded);
        health.reconnecting();
        assert_eq!(health.state, HidppSessionState::Connecting);
        assert_eq!(health.reconnect_count, 1);
    }
}
