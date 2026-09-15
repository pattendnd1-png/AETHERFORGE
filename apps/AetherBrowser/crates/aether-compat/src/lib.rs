#![forbid(unsafe_code)]
//! Provider-aware content routing and Chromium compatibility surfaces.
//!
//! Compatibility-heavy sites run in the system Chromium build and are
//! reparented into the AetherBrowser X11 content rectangle. This keeps the
//! actual Chromium web platform (MSE/WebRTC/service workers/Google auth) while
//! preserving AetherBrowser-owned chrome, tabs and profiles.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use url::Url;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentEngine {
    Native,
    Servo,
    Compatibility,
}

#[must_use]
pub fn classify_content_engine(raw_url: &str) -> ContentEngine {
    let trimmed = raw_url.trim();
    if trimmed.starts_with("aether://") || trimmed.starts_with("data:text/html") {
        return ContentEngine::Native;
    }
    let Ok(url) = Url::parse(trimmed) else {
        return ContentEngine::Servo;
    };
    if matches!(url.scheme(), "http" | "https") {
        ContentEngine::Compatibility
    } else {
        ContentEngine::Servo
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompatibilityGeometry {
    pub x: i16,
    pub y: i16,
    pub width: u32,
    pub height: u32,
}

impl CompatibilityGeometry {
    #[must_use]
    pub const fn new(x: i16, y: i16, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Debug)]
pub enum CompatibilityError {
    UnsupportedDisplay,
    ChromiumNotFound,
    Io(String),
    X11(String),
    WindowNotFound,
}

impl fmt::Display for CompatibilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDisplay => write!(
                f,
                "compatibility surface requires an X11/XWayland parent window"
            ),
            Self::ChromiumNotFound => write!(f, "system Chromium executable was not found"),
            Self::Io(message) => write!(f, "compatibility process error: {message}"),
            Self::X11(message) => write!(f, "compatibility X11 error: {message}"),
            Self::WindowNotFound => write!(f, "Chromium compatibility window did not appear"),
        }
    }
}

impl std::error::Error for CompatibilityError {}

#[derive(Debug)]
pub struct ChromiumCompatSurface {
    window_id: u32,
    url: String,
    visible: bool,
}

impl ChromiumCompatSurface {
    #[must_use]
    pub const fn window_id(&self) -> u32 {
        self.window_id
    }
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.visible
    }
}

#[derive(Debug)]
pub struct ExternalAppSurface {
    window_id: u32,
    app_id: String,
    visible: bool,
    attached: bool,
}

impl ExternalAppSurface {
    #[must_use]
    pub const fn window_id(&self) -> u32 {
        self.window_id
    }
    #[must_use]
    pub fn app_id(&self) -> &str {
        &self.app_id
    }
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.visible
    }
    #[must_use]
    pub const fn attached(&self) -> bool {
        self.attached
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompatCookie {
    pub name: String,
    pub value: String,
    /// HTTPS origin used for host-only cookie import.
    pub url: String,
    pub domain: Option<String>,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    pub expires: Option<f64>,
    pub same_site: Option<String>,
}

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use serde_json::{Value, json};
    use tungstenite::{Message, connect};
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{
        AtomEnum, ClientMessageData, ClientMessageEvent, ConfigureWindowAux, ConnectionExt,
        EventMask, MapState,
    };
    use x11rb::rust_connection::RustConnection;

    pub struct ChromiumCompatHost {
        conn: RustConnection,
        root: u32,
        parent: u32,
        profile_root: PathBuf,
        chromium: PathBuf,
    }

    impl fmt::Debug for ChromiumCompatHost {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("ChromiumCompatHost")
                .field("root", &self.root)
                .field("parent", &self.parent)
                .field("profile_root", &self.profile_root)
                .field("chromium", &self.chromium)
                .finish()
        }
    }

    impl ChromiumCompatHost {
        pub fn new(
            parent: u32,
            profile_root: impl Into<PathBuf>,
        ) -> Result<Self, CompatibilityError> {
            if parent == 0 {
                return Err(CompatibilityError::UnsupportedDisplay);
            }
            let chromium = chromium_executable().ok_or(CompatibilityError::ChromiumNotFound)?;
            let profile_root = profile_root.into();
            std::fs::create_dir_all(&profile_root)
                .map_err(|e| CompatibilityError::Io(e.to_string()))?;
            let (conn, screen) =
                x11rb::connect(None).map_err(|e| CompatibilityError::X11(e.to_string()))?;
            let root = conn.setup().roots[screen].root;
            Ok(Self {
                conn,
                root,
                parent,
                profile_root,
                chromium,
            })
        }

        #[must_use]
        pub fn profile_root(&self) -> &Path {
            &self.profile_root
        }

        fn top_level_windows(&self) -> Result<BTreeSet<u32>, CompatibilityError> {
            let reply = self
                .conn
                .query_tree(self.root)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .reply()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            Ok(reply.children.into_iter().collect())
        }

        fn is_chromium_window(&self, window: u32) -> bool {
            let Ok(cookie) = self.conn.intern_atom(false, b"WM_CLASS") else {
                return false;
            };
            let Ok(atom) = cookie.reply() else {
                return false;
            };
            let Ok(cookie) =
                self.conn
                    .get_property(false, window, atom.atom, AtomEnum::STRING, 0, 1024)
            else {
                return false;
            };
            let Ok(reply) = cookie.reply() else {
                return false;
            };
            let class = String::from_utf8_lossy(&reply.value).to_ascii_lowercase();
            class.contains("chromium") || class.contains("aetherbrowsercompat")
        }

        /// One-time migration of authenticated provider cookies from Servo into
        /// the persistent Chromium compatibility profile. Cookie values are sent
        /// only over Chromium's loopback DevTools socket and are never logged or
        /// written into AetherBrowser diagnostics.
        pub fn import_servo_cookies(
            &mut self,
            cookies: &[CompatCookie],
        ) -> Result<usize, CompatibilityError> {
            let marker = self.profile_root.join(".aether-servo-cookie-migration-v1");
            if marker.is_file() {
                return Ok(0);
            }
            if cookies.is_empty() {
                return Ok(0);
            }

            let devtools_port_file = self.profile_root.join("DevToolsActivePort");
            let _ = std::fs::remove_file(&devtools_port_file);
            let mut child = Command::new(&self.chromium)
                .arg(format!("--user-data-dir={}", self.profile_root.display()))
                .arg("--headless=new")
                .arg("--remote-debugging-port=0")
                .arg("--no-first-run")
                .arg("--no-default-browser-check")
                .arg("about:blank")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| CompatibilityError::Io(e.to_string()))?;

            let deadline = Instant::now() + Duration::from_secs(12);
            let (port, browser_path) = loop {
                if let Ok(contents) = std::fs::read_to_string(&devtools_port_file) {
                    let mut lines = contents.lines();
                    if let (Some(port), Some(path)) = (lines.next(), lines.next())
                        && !port.trim().is_empty()
                        && !path.trim().is_empty()
                    {
                        break (port.trim().to_owned(), path.trim().to_owned());
                    }
                }
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(CompatibilityError::Io(
                        "Chromium DevTools endpoint did not become ready".to_owned(),
                    ));
                }
                thread::sleep(Duration::from_millis(50));
            };

            let ws_url = format!("ws://127.0.0.1:{port}{browser_path}");
            let (mut socket, _) = connect(ws_url.as_str()).map_err(|e| {
                CompatibilityError::Io(format!("Chromium DevTools connection failed: {e}"))
            })?;
            let cookie_values: Vec<Value> = cookies
                .iter()
                .map(|cookie| {
                    let mut value = json!({
                        "name": cookie.name,
                        "value": cookie.value,
                        "url": cookie.url,
                        "path": cookie.path,
                        "secure": cookie.secure,
                        "httpOnly": cookie.http_only,
                    });
                    if let Some(domain) = &cookie.domain {
                        value["domain"] = Value::String(domain.clone());
                    }
                    if let Some(expires) = cookie.expires {
                        value["expires"] = json!(expires);
                    }
                    if let Some(same_site) = &cookie.same_site {
                        value["sameSite"] = Value::String(same_site.clone());
                    }
                    value
                })
                .collect();
            let request = json!({
                "id": 1,
                "method": "Storage.setCookies",
                "params": {"cookies": cookie_values},
            });
            socket
                .send(Message::text(request.to_string()))
                .map_err(|e| {
                    CompatibilityError::Io(format!("Chromium cookie migration send failed: {e}"))
                })?;

            loop {
                let message = socket.read().map_err(|e| {
                    CompatibilityError::Io(format!(
                        "Chromium cookie migration response failed: {e}"
                    ))
                })?;
                let Message::Text(text) = message else {
                    continue;
                };
                let response: Value = serde_json::from_str(text.as_str()).map_err(|e| {
                    CompatibilityError::Io(format!("Chromium cookie migration JSON failed: {e}"))
                })?;
                if response.get("id").and_then(Value::as_u64) != Some(1) {
                    continue;
                }
                if let Some(error) = response.get("error") {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(CompatibilityError::Io(format!(
                        "Chromium rejected cookie migration: {error}"
                    )));
                }
                break;
            }

            let _ = socket.send(Message::text(
                json!({"id": 2, "method": "Browser.close"}).to_string(),
            ));
            let _ = child.wait();
            std::fs::write(&marker, b"servo-provider-cookie-migration-v1\n")
                .map_err(|e| CompatibilityError::Io(e.to_string()))?;
            Ok(cookies.len())
        }

        pub fn create_surface(
            &mut self,
            raw_url: &str,
            geometry: CompatibilityGeometry,
        ) -> Result<ChromiumCompatSurface, CompatibilityError> {
            let before = self.top_level_windows()?;
            let mut command = Command::new(&self.chromium);
            command
                .arg(format!("--user-data-dir={}", self.profile_root.display()))
                .arg(format!("--app={raw_url}"))
                // The AetherBrowser parent is forced to X11/XWayland. Chromium must
                // use the same windowing protocol or X11 reparenting cannot work.
                .arg("--ozone-platform=x11")
                .arg("--new-window")
                .arg("--no-first-run")
                .arg("--no-default-browser-check")
                .arg("--disable-session-crashed-bubble")
                .arg("--disable-backgrounding-occluded-windows")
                .arg("--disable-renderer-backgrounding")
                .arg("--disable-background-timer-throttling")
                .arg("--class=AetherBrowserCompat")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            command
                .spawn()
                .map_err(|e| CompatibilityError::Io(e.to_string()))?;

            let deadline = Instant::now() + Duration::from_secs(12);
            let window = loop {
                let now = self.top_level_windows()?;
                if let Some(window) = now
                    .difference(&before)
                    .copied()
                    .find(|id| self.is_chromium_window(*id))
                {
                    break window;
                }
                if Instant::now() >= deadline {
                    return Err(CompatibilityError::WindowNotFound);
                }
                thread::sleep(Duration::from_millis(75));
            };

            self.conn
                .reparent_window(window, self.parent, geometry.x, geometry.y)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .configure_window(
                    window,
                    &ConfigureWindowAux::new()
                        .x(i32::from(geometry.x))
                        .y(i32::from(geometry.y))
                        .width(geometry.width)
                        .height(geometry.height),
                )
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .map_window(window)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .flush()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            Ok(ChromiumCompatSurface {
                window_id: window,
                url: raw_url.to_owned(),
                visible: true,
            })
        }

        pub fn set_visible(
            &self,
            surface: &mut ChromiumCompatSurface,
            visible: bool,
        ) -> Result<(), CompatibilityError> {
            if visible {
                if surface.visible {
                    return Ok(());
                }
                self.conn
                    .map_window(surface.window_id)
                    .map_err(|e| CompatibilityError::X11(e.to_string()))?
                    .check()
                    .map_err(|e| CompatibilityError::X11(e.to_string()))?;
                self.conn
                    .flush()
                    .map_err(|e| CompatibilityError::X11(e.to_string()))?;
                surface.visible = true;
                return Ok(());
            }

            // X11 child windows must be decisively disengaged when their tab is
            // inactive. Move the child outside any visible parent geometry first,
            // then unmap and verify X11 actually reports it as unmapped. This
            // prevents a Chromium child (notably YouTube Music) from remaining
            // visually/focus-active above a sibling tab when an unmap races with
            // the compositor/window manager.
            self.conn
                .configure_window(
                    surface.window_id,
                    &ConfigureWindowAux::new()
                        .x(-32768)
                        .y(-32768)
                        .width(1)
                        .height(1),
                )
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .unmap_window(surface.window_id)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .flush()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            let attributes = self
                .conn
                .get_window_attributes(surface.window_id)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .reply()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            if attributes.map_state != MapState::UNMAPPED {
                return Err(CompatibilityError::X11(format!(
                    "Chromium child {} did not disengage; map_state={:?}",
                    surface.window_id, attributes.map_state
                )));
            }
            surface.visible = false;
            Ok(())
        }

        pub fn resize(
            &self,
            surface: &ChromiumCompatSurface,
            geometry: CompatibilityGeometry,
        ) -> Result<(), CompatibilityError> {
            self.conn
                .configure_window(
                    surface.window_id,
                    &ConfigureWindowAux::new()
                        .x(i32::from(geometry.x))
                        .y(i32::from(geometry.y))
                        .width(geometry.width)
                        .height(geometry.height),
                )
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .flush()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            Ok(())
        }

        pub fn close(&self, surface: &mut ChromiumCompatSurface) -> Result<(), CompatibilityError> {
            // Ask Chromium to close its app window through the normal WM protocol.
            // That gives Chromium a normal shutdown boundary to flush cookies,
            // IndexedDB, service-worker state, and session data into the persistent
            // --user-data-dir. Window close is explicitly not a profile cleanup.
            let wm_protocols = self
                .conn
                .intern_atom(false, b"WM_PROTOCOLS")
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .reply()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .atom;
            let wm_delete_window = self
                .conn
                .intern_atom(false, b"WM_DELETE_WINDOW")
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .reply()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .atom;
            let event = ClientMessageEvent::new(
                32,
                surface.window_id,
                wm_protocols,
                ClientMessageData::from([wm_delete_window, 0, 0, 0, 0]),
            );
            self.conn
                .send_event(false, surface.window_id, EventMask::NO_EVENT, event)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .flush()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            surface.visible = false;
            Ok(())
        }
    }

    pub struct ExternalAppHost {
        conn: RustConnection,
        root: u32,
        parent: u32,
    }

    impl fmt::Debug for ExternalAppHost {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("ExternalAppHost")
                .field("root", &self.root)
                .field("parent", &self.parent)
                .finish()
        }
    }

    impl ExternalAppHost {
        pub fn new(parent: u32) -> Result<Self, CompatibilityError> {
            if parent == 0 {
                return Err(CompatibilityError::UnsupportedDisplay);
            }
            let (conn, screen) =
                x11rb::connect(None).map_err(|e| CompatibilityError::X11(e.to_string()))?;
            let root = conn.setup().roots[screen].root;
            Ok(Self { conn, root, parent })
        }

        fn top_level_windows(&self) -> Result<BTreeSet<u32>, CompatibilityError> {
            let reply = self
                .conn
                .query_tree(self.root)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .reply()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            Ok(reply.children.into_iter().collect())
        }

        fn window_pid(&self, window: u32) -> Option<u32> {
            let atom = self
                .conn
                .intern_atom(false, b"_NET_WM_PID")
                .ok()?
                .reply()
                .ok()?
                .atom;
            let reply = self
                .conn
                .get_property(false, window, atom, AtomEnum::CARDINAL, 0, 1)
                .ok()?
                .reply()
                .ok()?;
            reply.value32()?.next()
        }

        fn process_matches_program(pid: u32, program: &str) -> bool {
            let Ok(executable) = fs::read_link(format!("/proc/{pid}/exe")) else {
                return false;
            };
            let expected = Path::new(program).file_name();
            executable.file_name() == expected
        }

        fn adopt_window(
            &self,
            app_id: &str,
            window: u32,
            geometry: CompatibilityGeometry,
        ) -> Result<ExternalAppSurface, CompatibilityError> {
            self.conn
                .unmap_window(window)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .reparent_window(window, self.parent, geometry.x, geometry.y)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .configure_window(
                    window,
                    &ConfigureWindowAux::new()
                        .x(i32::from(geometry.x))
                        .y(i32::from(geometry.y))
                        .width(geometry.width)
                        .height(geometry.height),
                )
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .map_window(window)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .flush()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            Ok(ExternalAppSurface {
                window_id: window,
                app_id: app_id.to_owned(),
                visible: true,
                attached: true,
            })
        }

        pub fn adopt_surface(
            &self,
            app_id: &str,
            program: &str,
            geometry: CompatibilityGeometry,
        ) -> Result<Option<ExternalAppSurface>, CompatibilityError> {
            for window in self.top_level_windows()? {
                let Some(pid) = self.window_pid(window) else {
                    continue;
                };
                if Self::process_matches_program(pid, program) {
                    return self.adopt_window(app_id, window, geometry).map(Some);
                }
            }
            Ok(None)
        }

        pub fn adopt_surfaces(
            &self,
            app_id: &str,
            program: &str,
            geometry: CompatibilityGeometry,
        ) -> Result<Vec<ExternalAppSurface>, CompatibilityError> {
            let mut surfaces = Vec::new();
            for window in self.top_level_windows()? {
                let Some(pid) = self.window_pid(window) else {
                    continue;
                };
                if Self::process_matches_program(pid, program) {
                    surfaces.push(self.adopt_window(app_id, window, geometry)?);
                }
            }
            Ok(surfaces)
        }

        pub fn create_surface(
            &mut self,
            app_id: &str,
            program: &str,
            args: &[String],
            env: &[(String, String)],
            geometry: CompatibilityGeometry,
        ) -> Result<ExternalAppSurface, CompatibilityError> {
            let before = self.top_level_windows()?;
            let mut command = Command::new(program);
            command
                .args(args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            for (key, value) in env {
                command.env(key, value);
            }
            let child = command
                .spawn()
                .map_err(|e| CompatibilityError::Io(e.to_string()))?;
            let pid = child.id();

            let deadline = Instant::now() + Duration::from_secs(12);
            let window = loop {
                let now = self.top_level_windows()?;
                if let Some(window) = now
                    .difference(&before)
                    .copied()
                    .find(|window| self.window_pid(*window) == Some(pid))
                {
                    break window;
                }
                if Instant::now() >= deadline {
                    return Err(CompatibilityError::WindowNotFound);
                }
                thread::sleep(Duration::from_millis(50));
            };

            self.conn
                .reparent_window(window, self.parent, geometry.x, geometry.y)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .configure_window(
                    window,
                    &ConfigureWindowAux::new()
                        .x(i32::from(geometry.x))
                        .y(i32::from(geometry.y))
                        .width(geometry.width)
                        .height(geometry.height),
                )
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .map_window(window)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .flush()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;

            Ok(ExternalAppSurface {
                window_id: window,
                app_id: app_id.to_owned(),
                visible: true,
                attached: true,
            })
        }

        pub fn set_visible(
            &self,
            surface: &mut ExternalAppSurface,
            visible: bool,
        ) -> Result<(), CompatibilityError> {
            if !surface.attached {
                return Ok(());
            }
            if visible == surface.visible {
                return Ok(());
            }
            if visible {
                self.conn
                    .map_window(surface.window_id)
                    .map_err(|e| CompatibilityError::X11(e.to_string()))?
                    .check()
                    .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            } else {
                self.conn
                    .unmap_window(surface.window_id)
                    .map_err(|e| CompatibilityError::X11(e.to_string()))?
                    .check()
                    .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            }
            self.conn
                .flush()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            surface.visible = visible;
            Ok(())
        }

        pub fn resize(
            &self,
            surface: &ExternalAppSurface,
            geometry: CompatibilityGeometry,
        ) -> Result<(), CompatibilityError> {
            if !surface.attached {
                return Ok(());
            }
            self.conn
                .configure_window(
                    surface.window_id,
                    &ConfigureWindowAux::new()
                        .x(i32::from(geometry.x))
                        .y(i32::from(geometry.y))
                        .width(geometry.width)
                        .height(geometry.height),
                )
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .flush()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            Ok(())
        }

        pub fn detach(&self, surface: &mut ExternalAppSurface) -> Result<(), CompatibilityError> {
            if !surface.attached {
                return Ok(());
            }
            self.conn
                .unmap_window(surface.window_id)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .reparent_window(surface.window_id, self.root, 96, 96)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .map_window(surface.window_id)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .flush()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            surface.attached = false;
            surface.visible = true;
            Ok(())
        }

        pub fn reattach(
            &self,
            surface: &mut ExternalAppSurface,
            geometry: CompatibilityGeometry,
        ) -> Result<(), CompatibilityError> {
            if surface.attached {
                return Ok(());
            }
            self.conn
                .unmap_window(surface.window_id)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .reparent_window(surface.window_id, self.parent, geometry.x, geometry.y)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .configure_window(
                    surface.window_id,
                    &ConfigureWindowAux::new()
                        .x(i32::from(geometry.x))
                        .y(i32::from(geometry.y))
                        .width(geometry.width)
                        .height(geometry.height),
                )
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .map_window(surface.window_id)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .flush()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            surface.attached = true;
            surface.visible = true;
            Ok(())
        }

        pub fn close(&self, surface: &mut ExternalAppSurface) -> Result<(), CompatibilityError> {
            let wm_protocols = self
                .conn
                .intern_atom(false, b"WM_PROTOCOLS")
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .reply()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .atom;
            let wm_delete_window = self
                .conn
                .intern_atom(false, b"WM_DELETE_WINDOW")
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .reply()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .atom;
            let event = ClientMessageEvent::new(
                32,
                surface.window_id,
                wm_protocols,
                ClientMessageData::from([wm_delete_window, 0, 0, 0, 0]),
            );
            self.conn
                .send_event(false, surface.window_id, EventMask::NO_EVENT, event)
                .map_err(|e| CompatibilityError::X11(e.to_string()))?
                .check()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            self.conn
                .flush()
                .map_err(|e| CompatibilityError::X11(e.to_string()))?;
            surface.visible = false;
            Ok(())
        }
    }

    fn chromium_executable() -> Option<PathBuf> {
        if let Some(explicit) = std::env::var_os("AETHER_BROWSER_CHROMIUM") {
            let path = PathBuf::from(explicit);
            if path.is_file() {
                return Some(path);
            }
        }
        ["/usr/bin/chromium", "/usr/bin/chromium-browser"]
            .into_iter()
            .map(PathBuf::from)
            .find(|path| path.is_file())
    }

    pub use ChromiumCompatHost as Host;
    pub use ExternalAppHost as AppHost;
}

#[cfg(target_os = "linux")]
pub use linux::{AppHost as ExternalAppHost, Host as ChromiumCompatHost};

#[cfg(not(target_os = "linux"))]
#[derive(Debug)]
pub struct ChromiumCompatHost;

#[cfg(not(target_os = "linux"))]
impl ChromiumCompatHost {
    pub fn new(
        _parent: u32,
        _profile_root: impl Into<PathBuf>,
    ) -> Result<Self, CompatibilityError> {
        Err(CompatibilityError::UnsupportedDisplay)
    }
}

#[cfg(not(target_os = "linux"))]
#[derive(Debug)]
pub struct ExternalAppHost;

#[cfg(not(target_os = "linux"))]
impl ExternalAppHost {
    pub fn new(_parent: u32) -> Result<Self, CompatibilityError> {
        Err(CompatibilityError::UnsupportedDisplay)
    }
}
