use super::{HostRect, HostStatus, WindowHostMode, window_matches_battlenet};
use std::{
    collections::VecDeque,
    process,
    time::{Duration, Instant},
};
use x11rb::{
    connection::Connection,
    protocol::xproto::{Atom, AtomEnum, ConfigureWindowAux, ConnectionExt as _, Window},
    rust_connection::RustConnection,
};

pub(super) struct X11Host {
    connection: RustConnection,
    root: Window,
    launcher_window: Option<Window>,
    battle_net_window: Option<Window>,
    utf8_string: Atom,
    net_wm_name: Atom,
    net_wm_pid: Atom,
    last_discovery: Instant,
}

impl X11Host {
    pub(super) fn connect() -> Result<Self, String> {
        let (connection, screen_num) = x11rb::connect(None).map_err(|error| error.to_string())?;
        let root = connection
            .setup()
            .roots
            .get(screen_num)
            .ok_or_else(|| "X11 screen was not available".to_string())?
            .root;
        let utf8_string = intern(&connection, "UTF8_STRING")?;
        let net_wm_name = intern(&connection, "_NET_WM_NAME")?;
        let net_wm_pid = intern(&connection, "_NET_WM_PID")?;
        Ok(Self {
            connection,
            root,
            launcher_window: None,
            battle_net_window: None,
            utf8_string,
            net_wm_name,
            net_wm_pid,
            last_discovery: Instant::now() - Duration::from_secs(1),
        })
    }

    pub(super) fn sync(&mut self, target: HostRect) -> Result<HostStatus, String> {
        let battle_window_missing = self
            .battle_net_window
            .is_none_or(|window| !self.window_exists(window));
        if (self.launcher_window.is_none() || battle_window_missing)
            && self.last_discovery.elapsed() >= Duration::from_millis(600)
        {
            if self.launcher_window.is_none() {
                self.launcher_window = self.find_launcher_window()?;
            }
            if battle_window_missing {
                self.battle_net_window = self.find_battlenet_window()?;
            }
            self.last_discovery = Instant::now();
        }

        let Some(parent) = self.launcher_window else {
            return Ok(waiting("Waiting for the OpenSanctuary X11 host window."));
        };
        let Some(child) = self.battle_net_window else {
            return Ok(waiting("Battle.net is not presenting an X11 window yet."));
        };

        let tree = self
            .connection
            .query_tree(child)
            .map_err(|error| error.to_string())?
            .reply()
            .map_err(|error| error.to_string())?;
        if tree.parent != parent {
            self.connection
                .reparent_window(child, parent, clamp_i16(target.x), clamp_i16(target.y))
                .map_err(|error| error.to_string())?;
        }
        self.connection
            .configure_window(
                child,
                &ConfigureWindowAux::new()
                    .x(target.x)
                    .y(target.y)
                    .width(target.width)
                    .height(target.height)
                    .border_width(0),
            )
            .map_err(|error| error.to_string())?;
        self.connection
            .map_window(child)
            .map_err(|error| error.to_string())?;
        self.connection.flush().map_err(|error| error.to_string())?;

        Ok(HostStatus {
            mode: WindowHostMode::X11Embedded,
            embedded: true,
            detail: "Battle.net is embedded in the OpenSanctuary X11 host region.".into(),
        })
    }

    pub(super) fn release(&mut self) -> Result<(), String> {
        let Some(child) = self.battle_net_window.take() else {
            return Ok(());
        };
        if self.window_exists(child) {
            self.connection
                .reparent_window(child, self.root, 32, 32)
                .map_err(|error| error.to_string())?;
            self.connection
                .map_window(child)
                .map_err(|error| error.to_string())?;
            self.connection.flush().map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn find_launcher_window(&self) -> Result<Option<Window>, String> {
        let pid = process::id();
        for window in self.descendants(self.root) {
            if self.window_pid(window)? == Some(pid) {
                let title = self.window_title(window)?;
                if title.to_ascii_lowercase().contains("opensanctuary") {
                    return Ok(Some(window));
                }
            }
        }
        Ok(None)
    }

    fn find_battlenet_window(&self) -> Result<Option<Window>, String> {
        for window in self.descendants(self.root) {
            let title = self.window_title(window)?;
            let class = self.window_class(window)?;
            if window_matches_battlenet(&title, &class) {
                return Ok(Some(window));
            }
        }
        Ok(None)
    }

    fn descendants(&self, root: Window) -> Vec<Window> {
        let mut output = Vec::new();
        let mut queue = VecDeque::from([root]);
        let mut visited = 0usize;
        while let Some(window) = queue.pop_front() {
            if visited >= 4096 {
                break;
            }
            visited += 1;
            let Ok(cookie) = self.connection.query_tree(window) else {
                continue;
            };
            let Ok(reply) = cookie.reply() else {
                continue;
            };
            for child in reply.children {
                output.push(child);
                queue.push_back(child);
            }
        }
        output
    }

    fn window_title(&self, window: Window) -> Result<String, String> {
        if let Some(value) = self.property_string(window, self.net_wm_name, self.utf8_string)? {
            return Ok(value);
        }
        Ok(self
            .property_string(window, AtomEnum::WM_NAME.into(), AtomEnum::STRING.into())?
            .unwrap_or_default())
    }

    fn window_class(&self, window: Window) -> Result<String, String> {
        Ok(self
            .property_string(window, AtomEnum::WM_CLASS.into(), AtomEnum::STRING.into())?
            .unwrap_or_default()
            .replace('\0', " "))
    }

    fn window_pid(&self, window: Window) -> Result<Option<u32>, String> {
        let reply = self
            .connection
            .get_property(false, window, self.net_wm_pid, AtomEnum::CARDINAL, 0, 1)
            .map_err(|error| error.to_string())?
            .reply()
            .map_err(|error| error.to_string())?;
        Ok(reply.value32().and_then(|mut values| values.next()))
    }

    fn property_string(
        &self,
        window: Window,
        property: Atom,
        property_type: Atom,
    ) -> Result<Option<String>, String> {
        let reply = self
            .connection
            .get_property(false, window, property, property_type, 0, 4096)
            .map_err(|error| error.to_string())?
            .reply()
            .map_err(|error| error.to_string())?;
        if reply.value.is_empty() {
            return Ok(None);
        }
        Ok(Some(String::from_utf8_lossy(&reply.value).into_owned()))
    }

    fn window_exists(&self, window: Window) -> bool {
        window != 0
            && self
                .connection
                .get_window_attributes(window)
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .is_some()
    }
}

fn intern(connection: &RustConnection, name: &str) -> Result<Atom, String> {
    connection
        .intern_atom(false, name.as_bytes())
        .map_err(|error| error.to_string())?
        .reply()
        .map(|reply| reply.atom)
        .map_err(|error| error.to_string())
}

fn clamp_i16(value: i32) -> i16 {
    value.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

fn waiting(detail: &str) -> HostStatus {
    HostStatus {
        mode: WindowHostMode::X11Embedded,
        embedded: false,
        detail: detail.into(),
    }
}
