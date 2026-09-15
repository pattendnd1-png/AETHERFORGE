use aether_terminal_core::{
    SessionConfig, SessionSignal, TerminalSession, broker_socket_path, parity_capability_names,
};
use aether_terminal_ui::{MonitorState, PaneLayout, SearchOptions, SplitAxis, find_matches};
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub const AETHER_TERMINAL_DONOR_VERSION: &str = "10.2.21";
pub const AETHER_TERMINAL_PROTOCOL: &str = "unix-socket-bincode-v1";

#[derive(Debug)]
pub struct TerminalTab {
    pub title: String,
    pub session_id: String,
    pub session: TerminalSession,
}

#[derive(Debug)]
pub struct TerminalWorkspace {
    tabs: Vec<TerminalTab>,
    active: usize,
    panes: PaneLayout,
    // Task 4 renderer consumes this staged Terminal parity state.
    #[allow(dead_code)]
    search: SearchOptions,
    // Task 4 renderer consumes this staged Terminal parity state.
    #[allow(dead_code)]
    monitor: MonitorState,
    last_error: Option<String>,
}

impl Default for TerminalWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalWorkspace {
    #[must_use]
    pub fn new() -> Self {
        let _terminal_contract = (AETHER_TERMINAL_DONOR_VERSION, AETHER_TERMINAL_PROTOCOL);
        Self {
            tabs: Vec::new(),
            active: 0,
            panes: PaneLayout::default(),
            search: SearchOptions::default(),
            monitor: MonitorState::new(Duration::from_secs(10)),
            last_error: None,
        }
    }

    #[must_use]
    pub fn capability_names() -> Vec<&'static str> {
        parity_capability_names()
    }

    #[must_use]
    pub fn capability_count() -> usize {
        Self::capability_names().len()
    }

    #[must_use]
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    #[must_use]
    pub fn active_session_id(&self) -> Option<&str> {
        self.tabs
            .get(self.active)
            .map(|tab| tab.session_id.as_str())
    }

    #[must_use]
    pub fn pane_count(&self) -> usize {
        self.panes.pane_count()
    }

    #[must_use]
    // Task 4 renderer wires this staged Aether Terminal parity action.
    #[allow(dead_code)]
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn ensure_session(&mut self) -> Result<(), String> {
        if self.tabs.is_empty() {
            self.new_tab()?;
        }
        Ok(())
    }

    pub fn new_tab(&mut self) -> Result<(), String> {
        ensure_external_aether_terminal_daemon()?;
        let config = SessionConfig::login_shell().map_err(|error| error.to_string())?;
        let session = TerminalSession::spawn(config).map_err(|error| error.to_string())?;
        let session_id = session.session_id().to_string();
        let title = format!("Terminal {}", self.tabs.len() + 1);
        self.tabs.push(TerminalTab {
            title,
            session_id,
            session,
        });
        self.active = self.tabs.len().saturating_sub(1);
        self.last_error = None;
        Ok(())
    }

    // Task 4 renderer wires this staged Aether Terminal parity action.
    #[allow(dead_code)]
    pub fn reattach(&mut self, session_id: impl Into<String>) -> Result<(), String> {
        ensure_external_aether_terminal_daemon()?;
        let session_id = session_id.into();
        let session =
            TerminalSession::attach(session_id.clone()).map_err(|error| error.to_string())?;
        self.tabs.push(TerminalTab {
            title: "Reattached Terminal".into(),
            session_id,
            session,
        });
        self.active = self.tabs.len().saturating_sub(1);
        self.last_error = None;
        Ok(())
    }

    // Task 4 renderer wires this staged Aether Terminal parity action.
    #[allow(dead_code)]
    pub fn close_active_tab(&mut self) -> Result<(), String> {
        if self.tabs.is_empty() {
            return Ok(());
        }
        let mut tab = self.tabs.remove(self.active);
        tab.session.terminate().map_err(|error| error.to_string())?;
        if self.active >= self.tabs.len() && self.active > 0 {
            self.active -= 1;
        }
        Ok(())
    }

    // Task 4 renderer wires this staged Aether Terminal parity action.
    #[allow(dead_code)]
    pub fn activate_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
        }
    }

    // Task 4 renderer wires this staged Aether Terminal parity action.
    #[allow(dead_code)]
    pub fn split_left_right(&mut self) {
        self.panes.split_active(SplitAxis::Horizontal);
    }

    // Task 4 renderer wires this staged Aether Terminal parity action.
    #[allow(dead_code)]
    pub fn split_top_bottom(&mut self) {
        self.panes.split_active(SplitAxis::Vertical);
    }

    // Task 4 renderer wires this staged Aether Terminal parity action.
    #[allow(dead_code)]
    pub fn equalize_panes(&mut self) {
        self.panes.equalize();
    }

    // Task 4 renderer wires this staged Aether Terminal parity action.
    #[allow(dead_code)]
    pub fn toggle_maximize_pane(&mut self) {
        self.panes.toggle_maximize();
    }

    pub fn write_text(&mut self, text: &str) -> Result<(), String> {
        self.ensure_session()?;
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return Err("terminal tab unavailable".into());
        };
        tab.session
            .write_input(text.as_bytes())
            .map_err(|error| error.to_string())
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        self.ensure_session()?;
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return Err("terminal tab unavailable".into());
        };
        tab.session
            .write_input(bytes)
            .map_err(|error| error.to_string())
    }

    // Task 4 renderer wires this staged Aether Terminal parity action.
    #[allow(dead_code)]
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), String> {
        self.ensure_session()?;
        let Some(tab) = self.tabs.get_mut(self.active) else {
            return Err("terminal tab unavailable".into());
        };
        tab.session
            .resize(cols.max(2), rows.max(2))
            .map_err(|error| error.to_string())
    }

    // Task 4 renderer wires this staged Aether Terminal parity action.
    #[allow(dead_code)]
    pub fn scroll(&mut self, rows: i32) {
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.session.scroll_scrollback(rows);
        }
    }

    // Task 4 renderer wires this staged Aether Terminal parity action.
    #[allow(dead_code)]
    pub fn scroll_to_bottom(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.session.scroll_to_bottom();
        }
    }

    // Task 4 renderer wires this staged Aether Terminal parity action.
    #[allow(dead_code)]
    pub fn clear_scrollback(&mut self) {
        if let Some(tab) = self.tabs.get_mut(self.active) {
            tab.session.clear_scrollback();
        }
    }

    // Task 4 renderer wires this staged Aether Terminal parity action.
    #[allow(dead_code)]
    pub fn send_signal(&self, signal: SessionSignal) -> Result<(), String> {
        let Some(tab) = self.tabs.get(self.active) else {
            return Err("terminal tab unavailable".into());
        };
        tab.session
            .send_signal(signal)
            .map_err(|error| error.to_string())
    }

    #[must_use]
    pub fn snapshot_text(&self) -> String {
        self.tabs
            .get(self.active)
            .map(|tab| tab.session.snapshot().text)
            .unwrap_or_else(|| "Aether Terminal — no active session".into())
    }

    #[must_use]
    pub fn current_title(&self) -> String {
        self.tabs
            .get(self.active)
            .map(|tab| {
                let snapshot = tab.session.snapshot();
                if snapshot.title.trim().is_empty() {
                    tab.title.clone()
                } else {
                    snapshot.title
                }
            })
            .unwrap_or_else(|| "Aether Terminal".into())
    }

    #[must_use]
    pub fn current_cwd(&self) -> Option<String> {
        self.tabs
            .get(self.active)
            .and_then(|tab| tab.session.working_directory())
            .map(|path| path.to_string_lossy().into_owned())
    }

    // Task 4 renderer wires this staged Aether Terminal parity action.
    #[allow(dead_code)]
    pub fn set_search(&mut self, query: impl Into<String>, case_sensitive: bool, regex: bool) {
        self.search.query = query.into();
        self.search.case_sensitive = case_sensitive;
        self.search.regex = regex;
    }

    #[must_use]
    // Task 4 renderer wires this staged Aether Terminal parity action.
    #[allow(dead_code)]
    pub fn search_matches(&self) -> Vec<(usize, usize)> {
        find_matches(&self.snapshot_text(), &self.search)
    }

    #[must_use]
    // Task 4 renderer wires this staged Aether Terminal parity action.
    #[allow(dead_code)]
    pub fn observe_monitoring(&mut self) -> Vec<aether_terminal_ui::MonitorMode> {
        let Some(tab) = self.tabs.get(self.active) else {
            return Vec::new();
        };
        let revision = tab.session.revision();
        let prompt_generation = tab.session.semantic_state().prompt_generation;
        self.monitor
            .observe(revision, prompt_generation, Instant::now())
    }
}

fn ensure_external_aether_terminal_daemon() -> Result<(), String> {
    let socket = broker_socket_path().map_err(|error| error.to_string())?;
    if UnixStream::connect(&socket).is_ok() {
        return Ok(());
    }

    let candidates = ["/usr/bin/aether-terminal", "/usr/local/bin/aether-terminal"];
    let executable = candidates
        .iter()
        .find(|candidate| std::path::Path::new(candidate).is_file())
        .ok_or_else(|| "AetherForge Terminal executable is not installed".to_string())?;

    Command::new(executable)
        .arg("--session-daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("failed to start Aether Terminal session daemon: {error}"))?;

    for _ in 0..100 {
        if UnixStream::connect(&socket).is_ok() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }

    Err("Aether Terminal session daemon did not become ready".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_capability_registry_matches_current_terminal_contract() {
        let names = TerminalWorkspace::capability_names();
        assert_eq!(names.len(), 28);
        for required in [
            "vt-xterm",
            "truecolor",
            "alternate-screen",
            "bracketed-paste",
            "application-cursor",
            "kitty-keyboard",
            "mouse-tracking",
            "osc8-hyperlinks",
            "osc133-semantic-shell",
            "complex-text",
            "tabs",
            "split-views",
            "saved-layouts",
            "search",
            "selection-mode",
            "profiles",
            "bookmarks",
            "broadcast-input",
            "process-signals",
            "activity-monitoring",
            "silence-monitoring",
            "prompt-monitoring",
            "save-output",
            "print-screen",
            "zmodem-upload",
            "drag-drop-hotspots",
            "scripting-broker",
            "cli-compatibility",
        ] {
            assert!(
                names.contains(&required),
                "missing shared terminal capability: {required}"
            );
        }
    }

    #[test]
    fn shared_pane_layout_is_used_directly() {
        let mut terminal = TerminalWorkspace::new();
        assert_eq!(terminal.pane_count(), 1);
        terminal.split_left_right();
        terminal.split_top_bottom();
        assert_eq!(terminal.pane_count(), 3);
        terminal.equalize_panes();
    }

    #[test]
    fn terminal_integration_declares_existing_daemon_protocol() {
        assert_eq!(AETHER_TERMINAL_DONOR_VERSION, "10.2.21");
        assert_eq!(AETHER_TERMINAL_PROTOCOL, "unix-socket-bincode-v1");
    }
}
