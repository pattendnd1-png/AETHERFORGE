use reforge_core::{ServiceKind, ServiceState, ServiceStatus};
use std::collections::BTreeMap;

fn sanitize_message(message: impl Into<String>) -> String {
    let mut value = message.into().replace(['\r', '\n', '\t'], " ");
    while value.contains("  ") {
        value = value.replace("  ", " ");
    }
    value.truncate(240);
    value
}

#[derive(Debug)]
pub struct ServiceRegistry {
    items: BTreeMap<ServiceKind, ServiceStatus>,
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        let mut items = BTreeMap::new();
        for (kind, backend) in [
            (ServiceKind::Hidpp, "hidapi/hidraw"),
            (ServiceKind::Udev, "udevadm"),
            (ServiceKind::Fwupd, "fwupdmgr/LVFS"),
            (ServiceKind::V4l2, "v4l2-ctl"),
            (ServiceKind::Pipewire, "wpctl"),
        ] {
            items.insert(
                kind,
                ServiceStatus::unavailable(kind, backend, "service has not been probed yet"),
            );
        }
        Self { items }
    }
}

impl ServiceRegistry {
    pub fn set(
        &mut self,
        kind: ServiceKind,
        state: ServiceState,
        backend: impl Into<String>,
        version: Option<String>,
        message: Option<String>,
        last_success_unix_ms: Option<u64>,
    ) {
        self.items.insert(
            kind,
            ServiceStatus {
                kind,
                state,
                backend: backend.into(),
                version,
                message: message.map(sanitize_message),
                last_success_unix_ms,
            },
        );
    }

    pub fn ready(
        &mut self,
        kind: ServiceKind,
        backend: impl Into<String>,
        version: Option<String>,
        message: impl Into<String>,
        now_ms: u64,
    ) {
        self.set(
            kind,
            ServiceState::Ready,
            backend,
            version,
            Some(message.into()),
            Some(now_ms),
        );
    }

    pub fn degraded(&mut self, kind: ServiceKind, backend: impl Into<String>, message: impl Into<String>) {
        let previous = self.items.get(&kind).cloned();
        self.set(
            kind,
            ServiceState::Degraded,
            backend,
            previous.as_ref().and_then(|item| item.version.clone()),
            Some(message.into()),
            previous.and_then(|item| item.last_success_unix_ms),
        );
    }

    pub fn unavailable(&mut self, kind: ServiceKind, backend: impl Into<String>, message: impl Into<String>) {
        self.set(
            kind,
            ServiceState::Unavailable,
            backend,
            None,
            Some(message.into()),
            None,
        );
    }

    pub fn list(&self) -> Vec<ServiceStatus> {
        self.items.values().cloned().collect()
    }

    pub fn get(&self, kind: ServiceKind) -> Option<ServiceStatus> {
        self.items.get(&kind).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_can_recover_from_degraded_state() {
        let mut services = ServiceRegistry::default();
        services.degraded(ServiceKind::Hidpp, "hidraw", "temporary failure\nwith newline");
        assert_eq!(services.get(ServiceKind::Hidpp).unwrap().state, ServiceState::Degraded);
        services.ready(ServiceKind::Hidpp, "hidraw", Some("2.6.6".into()), "connected", 42);
        let status = services.get(ServiceKind::Hidpp).unwrap();
        assert_eq!(status.state, ServiceState::Ready);
        assert_eq!(status.last_success_unix_ms, Some(42));
    }

    #[test]
    fn messages_are_sanitized() {
        let mut services = ServiceRegistry::default();
        services.degraded(ServiceKind::Fwupd, "fwupdmgr", "line 1\nline 2\tsecret");
        let message = services.get(ServiceKind::Fwupd).unwrap().message.unwrap();
        assert!(!message.contains('\n'));
        assert!(!message.contains('\t'));
    }
}
