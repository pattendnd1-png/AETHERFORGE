use std::{env, ffi::OsStr};

mod wayland;
mod x11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowHostMode {
    X11Embedded,
    WaylandCompanion,
    Companion,
}

impl WindowHostMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::X11Embedded => "X11 embedded",
            Self::WaylandCompanion => "Wayland companion",
            Self::Companion => "Managed companion",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl HostRect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Option<Self> {
        (width >= 64 && height >= 64).then_some(Self {
            x,
            y,
            width,
            height,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostStatus {
    pub mode: WindowHostMode,
    pub embedded: bool,
    pub detail: String,
}

impl HostStatus {
    fn companion(mode: WindowHostMode, detail: impl Into<String>) -> Self {
        Self {
            mode,
            embedded: false,
            detail: detail.into(),
        }
    }
}

enum HostBackend {
    X11(Box<x11::X11Host>),
    Wayland(wayland::WaylandHost),
    Companion,
}

pub struct BattleNetWindowHost {
    backend: HostBackend,
    status: HostStatus,
}

impl BattleNetWindowHost {
    pub fn detect() -> Self {
        let mode = detect_window_host_mode(
            env::var_os("XDG_SESSION_TYPE").as_deref(),
            env::var_os("WAYLAND_DISPLAY").as_deref(),
            env::var_os("DISPLAY").as_deref(),
        );
        match mode {
            WindowHostMode::X11Embedded => match x11::X11Host::connect() {
                Ok(host) => Self {
                    backend: HostBackend::X11(Box::new(host)),
                    status: HostStatus::companion(mode, "Waiting for the Battle.net X11 window."),
                },
                Err(error) => Self {
                    backend: HostBackend::Companion,
                    status: HostStatus::companion(
                        WindowHostMode::Companion,
                        format!("X11 host unavailable; using companion mode: {error}"),
                    ),
                },
            },
            WindowHostMode::WaylandCompanion => Self {
                backend: HostBackend::Wayland(wayland::WaylandHost::new()),
                status: HostStatus::companion(
                    mode,
                    "Battle.net uses a compositor-managed companion surface on Wayland.",
                ),
            },
            WindowHostMode::Companion => Self {
                backend: HostBackend::Companion,
                status: HostStatus::companion(
                    mode,
                    "No embeddable X11 launcher surface was detected; using companion mode.",
                ),
            },
        }
    }

    pub fn mode(&self) -> WindowHostMode {
        self.status.mode
    }

    pub fn status(&self) -> &HostStatus {
        &self.status
    }

    pub fn sync(&mut self, target: HostRect) -> HostStatus {
        let result = match &mut self.backend {
            HostBackend::X11(host) => host.sync(target),
            HostBackend::Wayland(host) => Ok(host.sync(target)),
            HostBackend::Companion => Ok(HostStatus::companion(
                WindowHostMode::Companion,
                "Battle.net is managed as a companion surface.",
            )),
        };
        self.status = match result {
            Ok(status) => status,
            Err(error) => {
                if let HostBackend::X11(host) = &mut self.backend {
                    let _ = host.release();
                }
                self.backend = HostBackend::Companion;
                HostStatus::companion(
                    WindowHostMode::Companion,
                    format!("Battle.net embedding failed; using companion mode: {error}"),
                )
            }
        };
        self.status.clone()
    }

    pub fn release(&mut self) {
        if let HostBackend::X11(host) = &mut self.backend {
            let _ = host.release();
        }
        self.status.embedded = false;
    }
}

pub fn detect_window_host_mode(
    session_type: Option<&OsStr>,
    wayland_display: Option<&OsStr>,
    display: Option<&OsStr>,
) -> WindowHostMode {
    let wayland_session = session_type
        .and_then(OsStr::to_str)
        .is_some_and(|value| value.eq_ignore_ascii_case("wayland"));
    if wayland_session || wayland_display.is_some() {
        return WindowHostMode::WaylandCompanion;
    }
    if display.is_some() {
        return WindowHostMode::X11Embedded;
    }
    WindowHostMode::Companion
}

pub fn window_matches_battlenet(title: &str, class: &str) -> bool {
    let title = title.to_ascii_lowercase();
    let class = class.to_ascii_lowercase();
    title.contains("battle.net")
        || title.contains("battlenet")
        || class.contains("battle.net")
        || class.contains("battlenet")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wayland_never_claims_arbitrary_embedding() {
        assert_eq!(
            detect_window_host_mode(
                Some(OsStr::new("wayland")),
                Some(OsStr::new("wayland-0")),
                Some(OsStr::new(":0")),
            ),
            WindowHostMode::WaylandCompanion
        );
    }

    #[test]
    fn x11_session_selects_embedded_host() {
        assert_eq!(
            detect_window_host_mode(Some(OsStr::new("x11")), None, Some(OsStr::new(":0")),),
            WindowHostMode::X11Embedded
        );
    }

    #[test]
    fn battle_net_window_matching_is_specific() {
        assert!(window_matches_battlenet("Battle.net", "battle.net.exe"));
        assert!(window_matches_battlenet("Battle.net Login", "wine"));
        assert!(!window_matches_battlenet("OpenSanctuary", "opensanctuary"));
    }

    #[test]
    fn host_rect_rejects_tiny_surfaces() {
        assert!(HostRect::new(0, 0, 100, 100).is_some());
        assert!(HostRect::new(0, 0, 32, 32).is_none());
    }
}
