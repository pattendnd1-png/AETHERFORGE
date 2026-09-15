use super::{HostRect, HostStatus, WindowHostMode};

pub(super) struct WaylandHost;

impl WaylandHost {
    pub(super) fn new() -> Self {
        Self
    }

    pub(super) fn sync(&mut self, _target: HostRect) -> HostStatus {
        HostStatus {
            mode: WindowHostMode::WaylandCompanion,
            embedded: false,
            detail: "Battle.net is open as a managed companion surface on Wayland.".into(),
        }
    }
}
