use reforge_core::{DevicePresence, DeviceRuntimeInfo, DeviceSummary};
use std::collections::{BTreeMap, HashSet};

pub const OFFLINE_RETENTION_MS: u64 = 10 * 60 * 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryTransitionKind {
    Added,
    Refreshed,
    Offline,
    Reconnecting,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryTransition {
    pub key: String,
    pub kind: RegistryTransitionKind,
}

#[derive(Debug, Clone)]
struct RegistryEntry {
    info: DeviceRuntimeInfo,
    offline_since_unix_ms: Option<u64>,
}

#[derive(Debug, Default)]
pub struct DeviceRegistry {
    entries: BTreeMap<String, RegistryEntry>,
}

impl DeviceRegistry {
    pub fn merge_scan(&mut self, now_ms: u64, devices: Vec<DeviceSummary>) -> Vec<RegistryTransition> {
        let mut transitions = Vec::new();
        let seen: HashSet<String> = devices.iter().map(|device| device.key.clone()).collect();

        for device in devices {
            let key = device.key.clone();
            match self.entries.get_mut(&key) {
                Some(entry) => {
                    let reconnecting = entry.info.presence == DevicePresence::Offline;
                    entry.info.summary = device;
                    entry.info.last_seen_unix_ms = now_ms;
                    entry.offline_since_unix_ms = None;
                    if reconnecting {
                        entry.info.presence = DevicePresence::Reconnecting;
                        entry.info.reconnect_count = entry.info.reconnect_count.saturating_add(1);
                        transitions.push(RegistryTransition { key, kind: RegistryTransitionKind::Reconnecting });
                    } else {
                        entry.info.presence = DevicePresence::Online;
                        transitions.push(RegistryTransition { key, kind: RegistryTransitionKind::Refreshed });
                    }
                }
                None => {
                    self.entries.insert(
                        key.clone(),
                        RegistryEntry {
                            info: DeviceRuntimeInfo {
                                summary: device,
                                presence: DevicePresence::Online,
                                first_seen_unix_ms: now_ms,
                                last_seen_unix_ms: now_ms,
                                reconnect_count: 0,
                            },
                            offline_since_unix_ms: None,
                        },
                    );
                    transitions.push(RegistryTransition { key, kind: RegistryTransitionKind::Added });
                }
            }
        }

        for (key, entry) in &mut self.entries {
            if seen.contains(key) {
                continue;
            }
            if entry.info.presence != DevicePresence::Offline {
                entry.info.presence = DevicePresence::Offline;
                entry.offline_since_unix_ms = Some(now_ms);
                transitions.push(RegistryTransition { key: key.clone(), kind: RegistryTransitionKind::Offline });
            }
        }

        let expired = self
            .entries
            .iter()
            .filter_map(|(key, entry)| {
                let offline_since = entry.offline_since_unix_ms?;
                (now_ms.saturating_sub(offline_since) >= OFFLINE_RETENTION_MS).then(|| key.clone())
            })
            .collect::<Vec<_>>();
        for key in expired {
            self.entries.remove(&key);
            transitions.push(RegistryTransition { key, kind: RegistryTransitionKind::Removed });
        }

        transitions
    }

    pub fn mark_online(&mut self, key: &str) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.info.presence = DevicePresence::Online;
            entry.offline_since_unix_ms = None;
        }
    }

    pub fn mark_offline(&mut self, key: &str, now_ms: u64) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.info.presence = DevicePresence::Offline;
            entry.offline_since_unix_ms.get_or_insert(now_ms);
        }
    }

    pub fn get_summary(&self, key: &str) -> Option<DeviceSummary> {
        self.entries.get(key).map(|entry| entry.info.summary.clone())
    }

    pub fn get_info(&self, key: &str) -> Option<DeviceRuntimeInfo> {
        self.entries.get(key).map(|entry| entry.info.clone())
    }

    pub fn update_summary(&mut self, summary: DeviceSummary) {
        if let Some(entry) = self.entries.get_mut(&summary.key) {
            entry.info.summary = summary;
        }
    }

    pub fn runtime_list(&self) -> Vec<DeviceRuntimeInfo> {
        self.entries.values().map(|entry| entry.info.clone()).collect()
    }

    pub fn online_summaries(&self) -> Vec<DeviceSummary> {
        self.entries
            .values()
            .filter(|entry| entry.info.presence == DevicePresence::Online)
            .map(|entry| entry.info.summary.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reforge_core::{DeviceClass, TransportKind};

    fn device(key: &str) -> DeviceSummary {
        DeviceSummary {
            key: key.into(), path: format!("/dev/{key}"), device_index: 0xff, vendor_id: 0x046d,
            product_id: 1, product: key.into(), serial: None, hardware_id: None, interface_number: 0,
            usage_page: 0, usage: 0, transport: TransportKind::Usb, hidpp: true,
            device_class: DeviceClass::Keyboard, providers: vec![], controls: vec![], features: vec![], dpi: None,
            lighting: None, error: None,
        }
    }

    #[test]
    fn preserves_stable_identity_across_disconnect_and_reconnect() {
        let mut registry = DeviceRegistry::default();
        registry.merge_scan(100, vec![device("g515")]);
        registry.merge_scan(200, vec![]);
        assert_eq!(registry.get_info("g515").unwrap().presence, DevicePresence::Offline);
        let transitions = registry.merge_scan(300, vec![device("g515")]);
        assert!(transitions.iter().any(|item| item.kind == RegistryTransitionKind::Reconnecting));
        assert_eq!(registry.get_info("g515").unwrap().reconnect_count, 1);
        registry.mark_online("g515");
        assert_eq!(registry.get_info("g515").unwrap().presence, DevicePresence::Online);
    }

    #[test]
    fn removes_offline_entries_after_retention() {
        let mut registry = DeviceRegistry::default();
        registry.merge_scan(0, vec![device("old")]);
        registry.merge_scan(1, vec![]);
        registry.merge_scan(OFFLINE_RETENTION_MS + 2, vec![]);
        assert!(registry.get_info("old").is_none());
    }

    #[test]
    fn runtime_list_is_sorted_by_stable_key() {
        let mut registry = DeviceRegistry::default();
        registry.merge_scan(0, vec![device("z"), device("a")]);
        let keys = registry.runtime_list().into_iter().map(|item| item.summary.key).collect::<Vec<_>>();
        assert_eq!(keys, vec!["a", "z"]);
    }
}
