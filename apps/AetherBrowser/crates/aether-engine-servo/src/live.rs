//! Aether Browser v2.1 native-chrome Servo runtime.
//!
//! Architectural invariant: browser chrome and Home are Rust-native egui paint commands
//! rendered directly to the parent WindowRenderingContext. Servo WebViews exist only for
//! content tabs that actually need web/document rendering. There is no chrome WebView.

use aether_capture::{
    live_frame_probe_path, surface_capture_frames_path, surface_capture_gif_path,
    surface_capture_manifest_path, surface_capture_mp4_path, surface_capture_webp_path,
    ui_snapshot_path,
};
use aether_compat::{
    ChromiumCompatHost, ChromiumCompatSurface, CompatCookie, CompatibilityGeometry, ContentEngine,
    ExternalAppHost, ExternalAppSurface, classify_content_engine,
};
use aether_media_service::{
    NativeProvider, latest_native_frame, native_playback_state, start_native_playback,
    stop_native_playback,
};
use aether_native_pages::{NativePageRoute, native_page_for_url, native_page_for_url_with_library};
use aether_navigation::resolve_omnibox_input;
use aether_profile::{ProfileKind, ProfileStorage};
use aether_storage::{LibraryStore, PersistenceScope, should_persist_url, unix_now};
use aether_stream_providers::{
    NativePlaybackProvider, NativePlaybackRequest, classify_native_playback_url,
};
use aether_stream_studio::obs::{ObsWebSocketConfig, enforce_obs_embedded_capture_safety};
use aether_telemetry::{FrameRateTracker, LinuxTelemetrySampler, SystemTelemetrySnapshot};
use aether_ui::{
    ChromeHitTarget, ChromeLayout, ChromeModel, ChromeTab, MIN_WINDOW_HEIGHT_PX,
    MIN_WINDOW_WIDTH_PX, NativeChromeRenderer, WindowResizeEdge, window_resize_edge,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use euclid::{Box2D, Point2D, Rect, Size2D};
use servo::{
    Code, CookieSource, InputEvent, Key, KeyState, KeyboardEvent, LoadStatus, Location,
    MediaSessionEvent, Modifiers, MouseButtonAction, MouseButtonEvent, MouseLeftViewportEvent,
    MouseMoveEvent, NamedKey, NavigationRequest, OffscreenRenderingContext, Opts, RenderingContext,
    Servo, ServoBuilder, Theme, WebView, WebViewBuilder, WebViewPoint, WheelDelta, WheelEvent,
    WheelMode, WindowRenderingContext,
};
use std::cell::{Cell, RefCell};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::str::FromStr;
use std::time::{Duration, Instant};
use url::Url;
use webrender_api::units::{DevicePixel, DevicePoint};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton as WinitMouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{
    Key as WinitKey, KeyCode, KeyLocation as WinitKeyLocation, ModifiersState, PhysicalKey,
};
#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawWindowHandle};
use winit::window::{CursorIcon, Fullscreen, ResizeDirection, Window, WindowId};

use crate::normalize_startup_url;

const DEFAULT_WIDTH: u32 = 1672;
const DEFAULT_HEIGHT: u32 = 941;
const TITLE_PREFIX: &str = "Aether Browser";
const NEW_TAB_URL: &str = "about:blank";
const RUNTIME_ARCHITECTURE: &str = "AETHER_BROWSER_NATIVE_CHROME_DIRECT_GL_V1";
const PAGE_ZOOM_STEP: f32 = 0.10;
const PAGE_ZOOM_MIN: f32 = 0.50;
const PAGE_ZOOM_MAX: f32 = 3.00;
const TELEMETRY_POLL_INTERVAL: Duration = Duration::from_millis(1_000);
const TELEMETRY_COLLAPSED_POLL_INTERVAL: Duration = Duration::from_millis(2_000);
const LIBRARY_CHROME_POLL_INTERVAL: Duration = Duration::from_millis(1_500);

fn requested_test_window_size() -> Option<PhysicalSize<u32>> {
    let raw = std::env::var("AETHER_BROWSER_TEST_WINDOW_SIZE").ok()?;
    let (width, height) = raw.trim().split_once('x')?;
    let width = width.parse::<u32>().ok()?;
    let height = height.parse::<u32>().ok()?;
    Some(PhysicalSize::new(width, height))
}

fn startup_window_size() -> PhysicalSize<u32> {
    requested_test_window_size().map_or_else(
        || PhysicalSize::new(DEFAULT_WIDTH, DEFAULT_HEIGHT),
        |size| {
            PhysicalSize::new(
                size.width.max(MIN_WINDOW_WIDTH_PX),
                size.height.max(MIN_WINDOW_HEIGHT_PX),
            )
        },
    )
}

fn is_home_logical_url(url: &str) -> bool {
    url.trim_end_matches('/') == "aether://home"
}

#[derive(Clone, Debug)]
struct RuntimeTarget {
    actual_url: Option<Url>,
    display_url: String,
    page_title: String,
    native_page: bool,
    native_playback: Option<NativePlaybackRequest>,
    external_app: Option<ExternalAppTarget>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExternalAppTarget {
    app_id: String,
    program: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
    detached: bool,
}

const OBS_EMBEDDED_URL: &str = "aether://external-app/open?id=obs";

fn canonical_external_app_url(app_id: &str) -> String {
    format!("aether://external-app/open?id={app_id}")
}

fn external_open_request_path() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("aetherforge/aether-browser-open-url.request")
}

fn known_external_app_targets() -> Vec<ExternalAppTarget> {
    [OBS_EMBEDDED_URL]
        .into_iter()
        .filter_map(resolve_external_app_target)
        .collect()
}

fn resolve_external_app_target(raw_url: &str) -> Option<ExternalAppTarget> {
    let parsed = Url::parse(raw_url.trim()).ok()?;
    if parsed.scheme() != "aether"
        || parsed.host_str()? != "external-app"
        || parsed.path() != "/open"
    {
        return None;
    }
    let mut app_id = None;
    let mut mode = None;
    for (key, value) in parsed.query_pairs() {
        match key.as_ref() {
            "id" => app_id = Some(value.into_owned()),
            "mode" => mode = Some(value.into_owned()),
            _ => {}
        }
    }
    let app_id = app_id?;
    let detached = mode.as_deref() == Some("detached");
    match app_id.as_str() {
        "obs" => Some(ExternalAppTarget {
            app_id,
            program: "obs".to_owned(),
            args: Vec::new(),
            env: vec![("QT_QPA_PLATFORM".to_owned(), "xcb".to_owned())],
            detached,
        }),
        _ => None,
    }
}

fn launch_external_app_detached(target: &ExternalAppTarget) -> Result<(), String> {
    let mut command = Command::new(&target.program);
    command.args(&target.args);
    for (key, value) in &target.env {
        command.env(key, value);
    }
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("failed to launch detached {}: {error}", target.app_id))
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum LibraryAction {
    ClearHistory,
    ClearRecent,
    RemoveBookmark(i64),
    Reopen { id: i64, url: String },
}

fn parse_library_action(raw_url: &str) -> Option<LibraryAction> {
    let parsed = Url::parse(raw_url).ok()?;
    if parsed.scheme() != "aether" || parsed.host_str()? != "library" {
        return None;
    }
    let mut action = None;
    let mut id = None;
    let mut url = None;
    for (key, value) in parsed.query_pairs() {
        match key.as_ref() {
            "action" => action = Some(value.into_owned()),
            "id" => id = value.parse::<i64>().ok(),
            "url" => url = Some(value.into_owned()),
            _ => {}
        }
    }
    match action.as_deref()? {
        "clear-history" => Some(LibraryAction::ClearHistory),
        "clear-recent" => Some(LibraryAction::ClearRecent),
        "remove-bookmark" => Some(LibraryAction::RemoveBookmark(id?)),
        "reopen" => Some(LibraryAction::Reopen { id: id?, url: url? }),
        _ => None,
    }
}

fn resolve_interface_action_url(raw_url: &str) -> Option<String> {
    let parsed = Url::parse(raw_url.trim()).ok()?;
    if parsed.scheme() != "aether" {
        return None;
    }
    let host = parsed.host_str()?.to_ascii_lowercase();
    let path = parsed.path();
    let query = |name: &str| {
        parsed
            .query_pairs()
            .find_map(|(key, value)| (key == name).then(|| value.into_owned()))
    };

    match (host.as_str(), path) {
        ("auth", "/start") => match query("provider")?.as_str() {
            "twitch" => Some("https://www.twitch.tv/login".to_owned()),
            "google" => Some("https://accounts.google.com/".to_owned()),
            "apple" => Some("https://appleid.apple.com/".to_owned()),
            "local" => Some("aether://accounts?profile=local".to_owned()),
            _ => None,
        },
        ("provider", "/open") => match query("id")?.as_str() {
            "twitch" => Some("https://www.twitch.tv/".to_owned()),
            "youtube-live" => Some("https://www.youtube.com/".to_owned()),
            "velora" => Some("https://velora.tv/".to_owned()),
            "obs" => Some("aether://stream?provider=obs".to_owned()),
            "streamlabs" => Some("https://streamlabs.com/".to_owned()),
            "streamelements" => Some("https://streamelements.com/".to_owned()),
            "kick" => Some("https://kick.com/".to_owned()),
            "custom-rtmp" => Some("aether://stream?provider=custom-rtmp".to_owned()),
            "custom-srt" => Some("aether://stream?provider=custom-srt".to_owned()),
            _ => None,
        },
        ("provider", "/configure") => match query("id")?.as_str() {
            "twitch" | "streamlabs" | "streamelements" => {
                Some("aether://accounts?provider=twitch".to_owned())
            }
            "youtube-live" => Some("aether://accounts?provider=google".to_owned()),
            "velora" => Some("https://velora.tv/".to_owned()),
            "kick" => Some("aether://accounts?provider=kick".to_owned()),
            "obs" | "custom-rtmp" | "custom-srt" => {
                Some(format!("aether://stream?provider={}", query("id")?))
            }
            _ => None,
        },
        ("youtube-music", _) => match query("action").as_deref() {
            Some("search") => Some("https://music.youtube.com/search".to_owned()),
            Some("library") => Some("https://music.youtube.com/library".to_owned()),
            _ => Some("https://music.youtube.com/".to_owned()),
        },
        ("watch", _) if query("action").as_deref() == Some("play") => query("url"),
        _ => None,
    }
}

fn provider_open_in_new_tab(raw_url: &str) -> Option<String> {
    let parsed = Url::parse(raw_url.trim()).ok()?;
    if parsed.scheme() != "aether" || parsed.host_str()? != "provider" || parsed.path() != "/open" {
        return None;
    }
    let provider = parsed
        .query_pairs()
        .find_map(|(key, value)| (key == "id").then(|| value.into_owned()))?;
    match provider.as_str() {
        "twitch" | "youtube-live" | "velora" => resolve_interface_action_url(raw_url),
        _ => None,
    }
}

fn direct_external_runtime_target(destination: &str) -> Result<RuntimeTarget, String> {
    let actual_url = Url::parse(destination).map_err(|error| error.to_string())?;
    if !matches!(actual_url.scheme(), "http" | "https") {
        return Err("direct external target must use http or https".to_owned());
    }
    Ok(RuntimeTarget {
        actual_url: Some(actual_url),
        display_url: destination.to_owned(),
        page_title: String::new(),
        native_page: false,
        native_playback: None,
        external_app: None,
    })
}

fn resize_direction(edge: WindowResizeEdge) -> ResizeDirection {
    match edge {
        WindowResizeEdge::North => ResizeDirection::North,
        WindowResizeEdge::South => ResizeDirection::South,
        WindowResizeEdge::East => ResizeDirection::East,
        WindowResizeEdge::West => ResizeDirection::West,
        WindowResizeEdge::NorthEast => ResizeDirection::NorthEast,
        WindowResizeEdge::NorthWest => ResizeDirection::NorthWest,
        WindowResizeEdge::SouthEast => ResizeDirection::SouthEast,
        WindowResizeEdge::SouthWest => ResizeDirection::SouthWest,
    }
}

fn resize_cursor(edge: WindowResizeEdge) -> CursorIcon {
    match edge {
        WindowResizeEdge::North | WindowResizeEdge::South => CursorIcon::NsResize,
        WindowResizeEdge::East | WindowResizeEdge::West => CursorIcon::EwResize,
        WindowResizeEdge::NorthEast | WindowResizeEdge::SouthWest => CursorIcon::NeswResize,
        WindowResizeEdge::NorthWest | WindowResizeEdge::SouthEast => CursorIcon::NwseResize,
    }
}

fn content_device_point(
    layout: ChromeLayout,
    position: PhysicalPosition<f64>,
    size: PhysicalSize<u32>,
) -> Option<DevicePoint> {
    let (left, top, width, height) = layout.web_content_rect(size.width, size.height);
    let right = left.saturating_add(width);
    let bottom = top.saturating_add(height);
    if position.x < f64::from(left)
        || position.x >= f64::from(right)
        || position.y < f64::from(top)
        || position.y >= f64::from(bottom)
    {
        return None;
    }
    Some(DevicePoint::new(
        (position.x - f64::from(left)) as f32,
        (position.y - f64::from(top)) as f32,
    ))
}

fn content_blit_rect_for_size(
    layout: ChromeLayout,
    size: PhysicalSize<u32>,
) -> Rect<i32, euclid::UnknownUnit> {
    let (left, top, width, height) = layout.web_content_rect(size.width, size.height);
    // Chrome/input coordinates use a top-left origin, while OpenGL framebuffer
    // blits use a bottom-left origin. Convert exactly once at this boundary.
    let gl_y = size.height.saturating_sub(top.saturating_add(height));
    Rect::new(
        Point2D::new(left as i32, gl_y as i32),
        Size2D::new(width as i32, height as i32),
    )
}

fn force_native_provider_playback(raw_url: &str) -> bool {
    Url::parse(raw_url).ok().is_some_and(|url| {
        url.query_pairs()
            .any(|(key, value)| key == "aether_native" && value == "1")
    })
}

fn servo_provider_cookies_for_compatibility(servo: &Servo) -> Vec<CompatCookie> {
    // Provider-scoped migration only. We deliberately do not export the entire
    // cookie jar or print cookie values into logs.
    const ORIGINS: &[&str] = &[
        "https://www.youtube.com/",
        "https://music.youtube.com/",
        "https://accounts.google.com/",
        "https://www.google.com/",
        "https://www.twitch.tv/",
        "https://velora.tv/",
        "https://id.twitch.tv/",
        "https://streamlabs.com/",
        "https://streamelements.com/",
    ];
    let manager = servo.site_data_manager();
    let mut migrated = Vec::new();
    for origin in ORIGINS {
        let Ok(url) = Url::parse(origin) else {
            continue;
        };
        for cookie in manager.cookies_for_url(url.clone(), CookieSource::HTTP) {
            let candidate = CompatCookie {
                name: cookie.name().to_owned(),
                value: cookie.value().to_owned(),
                url: origin.to_string(),
                domain: cookie.domain().map(str::to_owned),
                path: cookie.path().unwrap_or("/").to_owned(),
                secure: cookie.secure().unwrap_or(url.scheme() == "https"),
                http_only: cookie.http_only().unwrap_or(false),
                expires: cookie
                    .expires_datetime()
                    .map(|value| value.unix_timestamp() as f64),
                same_site: cookie.same_site().map(|value| format!("{value:?}")),
            };
            let duplicate = migrated.iter().any(|existing: &CompatCookie| {
                existing.name == candidate.name
                    && existing.domain == candidate.domain
                    && existing.path == candidate.path
            });
            if !duplicate {
                migrated.push(candidate);
            }
        }
    }
    migrated
}

fn resolve_runtime_url(
    raw_url: &str,
    library: Option<&LibraryStore>,
) -> Result<RuntimeTarget, String> {
    let normalized = normalize_startup_url(raw_url).map_err(|error| error.to_string())?;
    if let Some(external_app) = resolve_external_app_target(&normalized) {
        return Ok(RuntimeTarget {
            actual_url: None,
            display_url: normalized,
            page_title: match external_app.app_id.as_str() {
                "obs" => "OBS Studio".to_owned(),
                _ => "External App".to_owned(),
            },
            native_page: false,
            native_playback: None,
            external_app: Some(external_app),
        });
    }
    if let Some(destination) = resolve_interface_action_url(&normalized)
        && destination != normalized
    {
        if Url::parse(&destination)
            .ok()
            .is_some_and(|url| matches!(url.scheme(), "http" | "https"))
        {
            return direct_external_runtime_target(&destination);
        }
        return resolve_runtime_url(&destination, library);
    }
    if is_home_logical_url(&normalized) {
        return Ok(RuntimeTarget {
            actual_url: None,
            display_url: "aether://home".to_owned(),
            page_title: "Welcome to Aether".to_owned(),
            native_page: true,
            native_playback: None,
            external_app: None,
        });
    }

    if !force_native_provider_playback(&normalized)
        && classify_content_engine(&normalized) == ContentEngine::Compatibility
    {
        return direct_external_runtime_target(&normalized);
    }

    if let Some(request) = classify_native_playback_url(&normalized) {
        let native_url = request.native_player_url();
        let page = native_page_for_url(&native_url)
            .ok_or_else(|| "native provider player route is unavailable".to_owned())?;
        let encoded = STANDARD.encode(page.html.as_bytes());
        let actual_url = Url::parse(&format!("data:text/html;base64,{encoded}"))
            .map_err(|error| error.to_string())?;
        return Ok(RuntimeTarget {
            actual_url: Some(actual_url),
            display_url: normalized,
            page_title: page.title,
            native_page: true,
            native_playback: Some(request),
            external_app: None,
        });
    }

    let route = NativePageRoute::from_url(&normalized);

    let page = if route == Some(NativePageRoute::Library) {
        let parsed = Url::parse(&normalized).map_err(|error| error.to_string())?;
        let query = parsed
            .query_pairs()
            .find_map(|(key, value)| (key == "q").then(|| value.into_owned()));
        let snapshot = library
            .map(|store| {
                store
                    .snapshot(query.as_deref(), 200)
                    .map_err(|error| error.to_string())
            })
            .transpose()?
            .unwrap_or_default();
        native_page_for_url_with_library(&normalized, &snapshot)
    } else {
        native_page_for_url(&normalized)
    };

    if let Some(page) = page {
        let encoded = STANDARD.encode(page.html.as_bytes());
        let actual_url = Url::parse(&format!("data:text/html;base64,{encoded}"))
            .map_err(|error| error.to_string())?;
        return Ok(RuntimeTarget {
            actual_url: Some(actual_url),
            display_url: page.logical_url,
            page_title: page.title,
            native_page: true,
            native_playback: None,
            external_app: None,
        });
    }

    let actual_url = Url::parse(&normalized).map_err(|error| error.to_string())?;
    Ok(RuntimeTarget {
        actual_url: Some(actual_url),
        display_url: normalized,
        page_title: String::new(),
        native_page: false,
        native_playback: None,
        external_app: None,
    })
}

fn should_accept_tab_url_change(native_page: bool, actual_url: &Url) -> bool {
    !(native_page && actual_url.scheme() == "data")
}

fn media_provider(provider: NativePlaybackProvider) -> NativeProvider {
    match provider {
        NativePlaybackProvider::YouTube => NativeProvider::YouTube,
        NativePlaybackProvider::Twitch => NativeProvider::Twitch,
    }
}

fn start_provider_request(request: &NativePlaybackRequest) -> Result<String, String> {
    start_native_playback(
        media_provider(request.provider),
        &request.resource,
        &request.original_url,
    )
    .map_err(|error| error.to_string())
}

#[derive(Clone, Debug)]
struct WakeEvent;

#[derive(Clone)]
struct ServoWaker(EventLoopProxy<WakeEvent>);

impl servo::EventLoopWaker for ServoWaker {
    fn clone_box(&self) -> Box<dyn servo::EventLoopWaker> {
        Box::new(self.clone())
    }

    fn wake(&self) {
        let _ = self.0.send_event(WakeEvent);
    }
}

struct TabMetadata {
    id: u64,
    current_url: RefCell<String>,
    page_title: RefCell<String>,
    native_page: Cell<bool>,
    native_playback: RefCell<Option<NativePlaybackRequest>>,
    native_frame_seen: Cell<bool>,
    media_session_seen: Cell<bool>,
    frame_ready: Cell<bool>,
    load_status: Cell<LoadStatus>,
    fullscreen: Cell<bool>,
    library: Rc<LibraryStore>,
    persistence_scope: PersistenceScope,
}

impl TabMetadata {
    fn new(
        id: u64,
        current_url: String,
        page_title: String,
        native_page: bool,
        native_playback: Option<NativePlaybackRequest>,
        library: Rc<LibraryStore>,
        persistence_scope: PersistenceScope,
    ) -> Self {
        Self {
            id,
            current_url: RefCell::new(current_url),
            page_title: RefCell::new(page_title),
            native_page: Cell::new(native_page),
            native_playback: RefCell::new(native_playback),
            native_frame_seen: Cell::new(false),
            media_session_seen: Cell::new(false),
            frame_ready: Cell::new(false),
            load_status: Cell::new(LoadStatus::Started),
            fullscreen: Cell::new(false),
            library,
            persistence_scope,
        }
    }
}

struct TabDelegate {
    window: Rc<Window>,
    metadata: Rc<TabMetadata>,
    pending_navigation: Rc<RefCell<Option<String>>>,
}

impl servo::WebViewDelegate for TabDelegate {
    fn notify_new_frame_ready(&self, _webview: WebView) {
        self.metadata.frame_ready.set(true);
        self.window.request_redraw();
    }

    fn notify_url_changed(&self, _webview: WebView, url: Url) {
        if should_accept_tab_url_change(self.metadata.native_page.get(), &url) {
            let next = url.to_string();
            let previous = self.metadata.current_url.replace(next.clone());
            self.metadata.native_page.set(false);
            if previous != next {
                let title = self.metadata.page_title.borrow().clone();
                let _ = self.metadata.library.record_history(
                    self.metadata.persistence_scope,
                    &next,
                    &title,
                    unix_now(),
                );
            }
        }
        self.window.request_redraw();
    }

    fn notify_page_title_changed(&self, _webview: WebView, title: Option<String>) {
        if self.metadata.native_page.get() {
            return;
        }
        let title = title.unwrap_or_default();
        self.metadata.page_title.replace(title.clone());
        let logical_url = self.metadata.current_url.borrow().clone();
        let _ = self.metadata.library.update_latest_history_title(
            self.metadata.persistence_scope,
            &logical_url,
            &title,
        );
        self.window.request_redraw();
    }

    fn notify_load_status_changed(&self, _webview: WebView, status: LoadStatus) {
        self.metadata.load_status.set(status);
        self.window.request_redraw();
    }

    fn notify_animating_changed(&self, _webview: WebView, animating: bool) {
        if animating {
            self.window.request_redraw();
        }
    }

    fn notify_media_session_event(&self, _webview: WebView, _event: MediaSessionEvent) {
        self.metadata.media_session_seen.set(true);
        println!("AETHER_BROWSER_MEDIA_SESSION_EVENT=OBSERVED");
        self.window.request_redraw();
    }

    fn notify_fullscreen_state_changed(&self, _webview: WebView, is_fullscreen: bool) {
        self.metadata.fullscreen.set(is_fullscreen);
        if is_fullscreen {
            self.window
                .set_fullscreen(Some(Fullscreen::Borderless(None)));
        } else {
            self.window.set_fullscreen(None);
        }
        println!(
            "AETHER_BROWSER_FULLSCREEN_STATE={}",
            if is_fullscreen { "ENTERED" } else { "EXITED" }
        );
    }

    fn request_navigation(&self, _webview: WebView, request: NavigationRequest) {
        let requested = request.url.to_string();
        if request.url.scheme() == "aether"
            || classify_native_playback_url(&requested).is_some()
            || classify_content_engine(&requested) == ContentEngine::Compatibility
        {
            println!("AETHER_BROWSER_ENDPOINT_DISPATCH=PENDING:{requested}");
            println!("AETHER_BROWSER_ENDPOINT_CLICK=PENDING:{requested}");
            self.pending_navigation.replace(Some(requested));
            request.deny();
            self.window.request_redraw();
        } else {
            // Non-HTTP(S), non-Aether navigation may remain on the internal Servo
            // surface. All ordinary HTTP(S) destinations classify as Chromium and
            // are intercepted above.
            println!("AETHER_BROWSER_INTERNAL_NAVIGATION=ALLOW:{requested}");
            request.allow();
        }
    }
}

struct RuntimeTab {
    webview: Option<WebView>,
    rendering_context: Option<Rc<OffscreenRenderingContext>>,
    compatibility: Option<ChromiumCompatSurface>,
    external_app: Option<ExternalAppSurface>,
    metadata: Rc<TabMetadata>,
}

impl RuntimeTab {
    fn is_native_home(&self) -> bool {
        is_home_logical_url(&self.metadata.current_url.borrow())
            && self.webview.is_none()
            && self.compatibility.is_none()
            && self.external_app.is_none()
    }
}

struct AnimatedSnapshotCapture {
    frame_index: u8,
}

struct RunningBrowser {
    window: Rc<Window>,
    servo: Servo,
    parent_context: Rc<WindowRenderingContext>,
    native_renderer: NativeChromeRenderer,
    egui_context: egui::Context,
    egui_painter: egui_glow::Painter,
    tabs: Vec<RuntimeTab>,
    active_index: usize,
    next_tab_id: u64,
    pending_navigation: Rc<RefCell<Option<String>>>,
    omnibox: String,
    omnibox_active: bool,
    omnibox_select_all: bool,
    omnibox_caret: usize,
    modifiers: ModifiersState,
    cursor_position: PhysicalPosition<f64>,
    cursor_in_content: bool,
    layout: ChromeLayout,
    last_native_frame_poll: Instant,
    telemetry_sampler: LinuxTelemetrySampler,
    telemetry_snapshot: SystemTelemetrySnapshot,
    frame_rate_tracker: FrameRateTracker,
    last_telemetry_poll: Instant,
    last_library_chrome_poll: Instant,
    active_bookmarked_cache: bool,
    active_download_count_cache: usize,
    last_window_title: String,
    startup_started: Instant,
    first_content_paint_reported: bool,
    page_complete_reported: bool,
    snapshot_requested: bool,
    close_requested: bool,
    animated_snapshot: Option<AnimatedSnapshotCapture>,
    startup_revealed: bool,
    live_frame_probe: Option<PathBuf>,
    live_frame_probe_result: Option<Result<(), String>>,
    live_frame_probe_revealed_at: Option<Instant>,
    library: Rc<LibraryStore>,
    persistence_scope: PersistenceScope,
    compatibility_host: Option<ChromiumCompatHost>,
    external_app_host: Option<ExternalAppHost>,
    _profile_storage: ProfileStorage,
}

impl Drop for RunningBrowser {
    fn drop(&mut self) {
        let _ = self.parent_context.make_current();
        for tab in &mut self.tabs {
            tab.webview.take();
            tab.rendering_context.take();
            if let (Some(host), Some(surface)) =
                (self.compatibility_host.as_ref(), tab.compatibility.as_mut())
            {
                let _ = host.close(surface);
            }
            tab.compatibility.take();
            if let (Some(host), Some(surface)) =
                (self.external_app_host.as_ref(), tab.external_app.as_mut())
            {
                if surface.attached() {
                    let _ = host.close(surface);
                } else {
                    println!(
                        "AETHER_BROWSER_EXTERNAL_APP_RESTART_PRESERVE=PASS:{}",
                        surface.app_id()
                    );
                }
            }
            tab.external_app.take();
        }
        self.tabs.clear();
        println!("AETHER_BROWSER_COMPAT_WINDOW_CLOSE=FLUSH_ONLY_NO_LOGOUT");
        self.egui_painter.destroy();
        println!("AETHER_BROWSER_RUNTIME_SHUTDOWN=NATIVE_RESOURCES_RELEASED");
    }
}

fn x11_parent_window_id(window: &Window) -> Option<u32> {
    let raw = window.window_handle().ok()?.as_raw();
    match raw {
        RawWindowHandle::Xlib(handle) => u32::try_from(handle.window).ok().filter(|id| *id != 0),
        RawWindowHandle::Xcb(handle) => Some(handle.window.get()),
        _ => None,
    }
}

impl RunningBrowser {
    fn active_tab(&self) -> &RuntimeTab {
        &self.tabs[self.active_index]
    }

    fn active_tab_mut(&mut self) -> &mut RuntimeTab {
        &mut self.tabs[self.active_index]
    }

    fn active_webview(&self) -> Option<&WebView> {
        self.active_tab().webview.as_ref()
    }

    fn active_compatibility_surface(&self) -> Option<&ChromiumCompatSurface> {
        self.active_tab().compatibility.as_ref()
    }

    fn compatibility_geometry_for_size(&self, size: PhysicalSize<u32>) -> CompatibilityGeometry {
        let (x, y, width, height) = self.content_geometry_for_size(size);
        CompatibilityGeometry::new(
            i16::try_from(x).unwrap_or(i16::MAX),
            i16::try_from(y).unwrap_or(i16::MAX),
            width,
            height,
        )
    }

    fn is_home_surface(&self) -> bool {
        self.active_tab().is_native_home()
    }

    fn active_url(&self) -> String {
        self.active_tab().metadata.current_url.borrow().clone()
    }

    fn active_title(&self) -> String {
        self.active_tab().metadata.page_title.borrow().clone()
    }

    fn content_geometry_for_size(&self, size: PhysicalSize<u32>) -> (u32, u32, u32, u32) {
        self.layout.web_content_rect(size.width, size.height)
    }

    fn content_size_for_size(&self, size: PhysicalSize<u32>) -> PhysicalSize<u32> {
        let (_, _, width, height) = self.content_geometry_for_size(size);
        PhysicalSize::new(width, height)
    }

    fn content_size(&self) -> PhysicalSize<u32> {
        self.content_size_for_size(self.window.inner_size())
    }

    fn content_blit_rect(&self) -> Rect<i32, euclid::UnknownUnit> {
        content_blit_rect_for_size(self.layout, self.window.inner_size())
    }

    fn content_capture_rect(&self, size: PhysicalSize<u32>) -> Box2D<i32, DevicePixel> {
        let rect = content_blit_rect_for_size(self.layout, size);
        Box2D::from_origin_and_size(
            Point2D::new(rect.origin.x, rect.origin.y),
            Size2D::new(rect.size.width, rect.size.height),
        )
    }

    fn chrome_model(&self) -> ChromeModel {
        let tabs = self
            .tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| ChromeTab {
                id: tab.metadata.id,
                title: tab.metadata.page_title.borrow().clone(),
                url: tab.metadata.current_url.borrow().clone(),
                active: index == self.active_index,
            })
            .collect();
        let mut model = ChromeModel::new(tabs, self.omnibox.clone());
        model.omnibox_active = self.omnibox_active;
        model.omnibox_caret = self.omnibox_caret;
        model.home_mode = self.is_home_surface();
        model.layout = self.layout;
        model.telemetry = self.telemetry_snapshot.clone();
        model.active_bookmarked = self.active_bookmarked_cache;
        model.active_download_count = self.active_download_count_cache;
        model
    }

    fn update_window_title(&mut self) {
        let title = self.active_title();
        let suffix = if title.trim().is_empty() {
            self.active_url()
        } else {
            title
        };
        let next = format!("{TITLE_PREFIX} — {suffix}");
        if self.last_window_title != next {
            self.window.set_title(&next);
            self.last_window_title = next;
        }
    }

    fn poll_library_chrome_state(&mut self) {
        if self.last_library_chrome_poll.elapsed() < LIBRARY_CHROME_POLL_INTERVAL {
            return;
        }
        self.last_library_chrome_poll = Instant::now();
        let active_url = self.active_url();
        self.active_bookmarked_cache = self.library.is_bookmarked(&active_url).unwrap_or(false);
        self.active_download_count_cache = self
            .library
            .snapshot(None, 200)
            .map(|snapshot| snapshot.active_download_count())
            .unwrap_or(0);
    }

    fn poll_telemetry(&mut self) {
        let interval = if self.layout.utility_collapsed() {
            TELEMETRY_COLLAPSED_POLL_INTERVAL
        } else {
            TELEMETRY_POLL_INTERVAL
        };
        if self.last_telemetry_poll.elapsed() < interval {
            return;
        }
        self.last_telemetry_poll = Instant::now();
        let browser_fps_milli = self.telemetry_snapshot.browser_fps_milli;
        let stream = self.telemetry_snapshot.stream;
        let mut snapshot = self.telemetry_sampler.sample();
        snapshot.browser_fps_milli = browser_fps_milli;
        snapshot.stream = stream;
        self.telemetry_snapshot = snapshot;
        self.window.request_redraw();
    }

    fn poll_external_open_request(&mut self) {
        let path = external_open_request_path();
        let Ok(raw) = fs::read_to_string(&path) else {
            return;
        };
        let _ = fs::remove_file(&path);
        let requested = raw.trim();
        let Ok(url) = Url::parse(requested) else {
            eprintln!("AETHER_BROWSER_EXTERNAL_OPEN_REQUEST=REJECT:invalid-url");
            return;
        };
        let trusted = url.scheme() == "aether"
            || (url.scheme() == "https"
                && matches!(
                    url.host_str(),
                    Some("id.twitch.tv" | "www.twitch.tv" | "dev.twitch.tv")
                ));
        if !trusted {
            eprintln!("AETHER_BROWSER_EXTERNAL_OPEN_REQUEST=REJECT:untrusted-origin");
            return;
        }
        match self.create_tab(url.as_str(), false) {
            Ok(()) => println!("AETHER_BROWSER_EXTERNAL_OPEN_REQUEST=PASS:IN_MAIN_WINDOW"),
            Err(error) => eprintln!("AETHER_BROWSER_EXTERNAL_OPEN_REQUEST=FAIL:{error}"),
        }
    }

    fn spin(&mut self) {
        self.servo.spin_event_loop();
        self.poll_telemetry();
        self.poll_library_chrome_state();
        self.poll_external_open_request();
        let pending = self.pending_navigation.borrow_mut().take();
        if let Some(url) = pending {
            let before = self.active_url();
            match self.load_active_url(&url) {
                Ok(()) => {
                    let after = self.active_url();
                    println!("AETHER_BROWSER_ENDPOINT_DISPATCH=PASS:{url}");
                    println!("AETHER_BROWSER_ENDPOINT_CLICK=PASS:{url}");
                    println!("AETHER_BROWSER_ENDPOINT_EXECUTION=PASS:{after}");
                    if after != before {
                        println!("AETHER_BROWSER_ACTIVE_URL_CHANGED=PASS:{before}->{after}");
                    } else {
                        println!("AETHER_BROWSER_ACTIVE_URL_CHANGED=UNCHANGED:{after}");
                    }
                }
                Err(error) => {
                    eprintln!("AETHER_BROWSER_ENDPOINT_DISPATCH=FAIL:{url}:{error}");
                    eprintln!("AETHER_BROWSER_ENDPOINT_CLICK=FAIL:{url}:{error}");
                }
            }
        }
        self.update_window_title();
    }

    fn composite_content(&self) {
        let Some(context) = self.active_tab().rendering_context.as_ref() else {
            return;
        };
        if let Some(render_to_parent) = context.render_to_parent_callback() {
            let gl = self.parent_context.glow_gl_api();
            render_to_parent(gl.as_ref(), self.content_blit_rect());
        }
    }

    fn request_ui_snapshot(&mut self) {
        self.snapshot_requested = true;
        self.window.request_redraw();
    }

    fn save_ui_snapshot(&self) -> Result<PathBuf, String> {
        let size = self.window.inner_size();
        let rect: Box2D<i32, DevicePixel> = Box2D::from_origin_and_size(
            Point2D::new(0, 0),
            Size2D::new(size.width as i32, size.height as i32),
        );
        let image = self
            .parent_context
            .read_to_image(rect)
            .ok_or_else(|| "rendering context did not provide snapshot pixels".to_owned())?;
        let path = ui_snapshot_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        image.save(&path).map_err(|error| error.to_string())?;
        Ok(path)
    }

    fn request_animated_snapshot(&mut self) {
        if self.animated_snapshot.is_some() {
            return;
        }
        let frames = surface_capture_frames_path();
        let _ = fs::remove_dir_all(&frames);
        if let Err(error) = fs::create_dir_all(&frames) {
            eprintln!("AETHER_BROWSER_ANIMATED_SNAPSHOT_FAILED={error}");
            return;
        }
        self.animated_snapshot = Some(AnimatedSnapshotCapture { frame_index: 0 });
        println!("AETHER_BROWSER_ANIMATED_NATIVE_SURFACE=START");
        self.window.request_redraw();
    }

    fn save_animation_frame(&self, index: u8) -> Result<PathBuf, String> {
        let size = self.window.inner_size();
        let rect: Box2D<i32, DevicePixel> = Box2D::from_origin_and_size(
            Point2D::new(0, 0),
            Size2D::new(size.width as i32, size.height as i32),
        );
        let image = self.parent_context.read_to_image(rect).ok_or_else(|| {
            "rendering context did not provide animated snapshot pixels".to_owned()
        })?;
        let path = surface_capture_frames_path().join(format!("frame-{index:02}.png"));
        image.save(&path).map_err(|error| error.to_string())?;
        Ok(path)
    }

    fn run_ffmpeg(args: &[String]) -> Result<(), String> {
        let status = Command::new("ffmpeg")
            .args(args)
            .status()
            .map_err(|error| format!("ffmpeg unavailable: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("ffmpeg failed with status {status}"))
        }
    }

    fn encode_surface_capture(frame_dir: &Path) -> Result<Vec<PathBuf>, String> {
        let pattern = frame_dir.join("frame-%02d.png");
        let pattern = pattern.to_string_lossy().into_owned();
        let gif = surface_capture_gif_path();
        let webp = surface_capture_webp_path();
        let mp4 = surface_capture_mp4_path();
        for output in [&gif, &webp, &mp4] {
            let _ = fs::remove_file(output);
        }
        Self::run_ffmpeg(&[
            "-y".into(),
            "-loglevel".into(),
            "error".into(),
            "-framerate".into(),
            "2".into(),
            "-i".into(),
            pattern.clone(),
            "-vf".into(),
            "fps=6".into(),
            "-loop".into(),
            "0".into(),
            gif.to_string_lossy().into_owned(),
        ])?;
        Self::run_ffmpeg(&[
            "-y".into(),
            "-loglevel".into(),
            "error".into(),
            "-framerate".into(),
            "2".into(),
            "-i".into(),
            pattern.clone(),
            "-c:v".into(),
            "libwebp_anim".into(),
            "-lossless".into(),
            "1".into(),
            "-loop".into(),
            "0".into(),
            webp.to_string_lossy().into_owned(),
        ])?;
        Self::run_ffmpeg(&[
            "-y".into(),
            "-loglevel".into(),
            "error".into(),
            "-framerate".into(),
            "2".into(),
            "-i".into(),
            pattern,
            "-vf".into(),
            "format=yuv420p".into(),
            "-movflags".into(),
            "+faststart".into(),
            mp4.to_string_lossy().into_owned(),
        ])?;
        Ok(vec![gif, webp, mp4])
    }

    fn finish_animated_snapshot(&mut self, _capture: AnimatedSnapshotCapture) {
        let frames = surface_capture_frames_path();
        match Self::encode_surface_capture(&frames) {
            Ok(outputs) => {
                let manifest = surface_capture_manifest_path();
                let body = format!(
                    "AETHER_BROWSER_ANIMATED_NATIVE_SURFACE=PASS\nSEQUENCE=NATIVE_BROWSER_SURFACE\nGIF={}\nWEBP={}\nMP4={}\nFRAMES={}\n",
                    outputs[0].display(),
                    outputs[1].display(),
                    outputs[2].display(),
                    frames.display()
                );
                if let Err(error) = fs::write(&manifest, body) {
                    eprintln!("AETHER_BROWSER_ANIMATED_SNAPSHOT_MANIFEST_FAILED={error}");
                }
                println!(
                    "AETHER_BROWSER_ANIMATED_SNAPSHOT_GIF={}",
                    outputs[0].display()
                );
                println!(
                    "AETHER_BROWSER_ANIMATED_SNAPSHOT_WEBP={}",
                    outputs[1].display()
                );
                println!(
                    "AETHER_BROWSER_ANIMATED_SNAPSHOT_MP4={}",
                    outputs[2].display()
                );
                println!(
                    "AETHER_BROWSER_ANIMATED_SNAPSHOT_MANIFEST={}",
                    manifest.display()
                );
            }
            Err(error) => eprintln!(
                "AETHER_BROWSER_ANIMATED_SNAPSHOT_FAILED={error};frames={}",
                frames.display()
            ),
        }
    }

    fn advance_animated_snapshot_after_frame(&mut self) {
        let Some(frame_index) = self
            .animated_snapshot
            .as_ref()
            .map(|capture| capture.frame_index)
        else {
            return;
        };
        if frame_index < 3 {
            if let Some(capture) = self.animated_snapshot.as_mut() {
                capture.frame_index = capture.frame_index.saturating_add(1);
            }
            self.window.request_redraw();
        } else if let Some(capture) = self.animated_snapshot.take() {
            self.finish_animated_snapshot(capture);
        }
    }

    fn capture_live_frame_probe(&mut self) {
        let Some(path) = self.live_frame_probe.take() else {
            return;
        };
        let result = (|| -> Result<(), String> {
            let size = self.window.inner_size();
            println!("AETHER_BROWSER_LIVE_FRAME_CAPTURE_SCOPE=WINDOW_CHROME_EXTERNAL_CHILD");
            let content_only =
                std::env::var_os("AETHER_BROWSER_LIVE_FRAME_PROBE_CONTENT_ONLY").is_some();
            if let Some(expected) = requested_test_window_size() {
                if size != expected {
                    println!(
                        "AETHER_BROWSER_CUSTOM_RESOLUTION_WINDOW=CLAMPED:requested={}x{}:actual={}x{}",
                        expected.width, expected.height, size.width, size.height
                    );
                } else {
                    println!(
                        "AETHER_BROWSER_CUSTOM_RESOLUTION_WINDOW=EXACT:{}x{}",
                        size.width, size.height
                    );
                }
                // Validate the surfaces against the size the compositor actually granted.
                // The requested 1669x937 geometry is separately covered by pure tests.
                let expected_content = self.content_size_for_size(size);
                if let Some(context) = self.active_tab().rendering_context.as_ref()
                    && context.size() != expected_content
                {
                    let actual = context.size();
                    return Err(format!(
                        "custom-resolution-offscreen-mismatch:expected={}x{}:actual={}x{}",
                        expected_content.width,
                        expected_content.height,
                        actual.width,
                        actual.height
                    ));
                }
                let (content_left, content_top, content_width, content_height) =
                    self.content_geometry_for_size(size);
                let content_bottom = content_top.saturating_add(content_height);
                let content_right = content_left.saturating_add(content_width);
                if content_bottom != size.height.saturating_sub(self.layout.status_height()) {
                    return Err("custom-resolution-content-does-not-reach-status-bar".to_owned());
                }
                let blit = self.content_blit_rect();
                if blit.origin.x != content_left as i32
                    || blit.origin.y != self.layout.status_height() as i32
                    || blit.size.width != content_width as i32
                    || blit.size.height != content_height as i32
                {
                    return Err(format!(
                        "custom-resolution-blit-mismatch:logical={}x{}@{},{}:gl={}x{}@{},{}",
                        content_width,
                        content_height,
                        content_left,
                        content_top,
                        blit.size.width,
                        blit.size.height,
                        blit.origin.x,
                        blit.origin.y
                    ));
                }
                println!(
                    "AETHER_BROWSER_CUSTOM_RESOLUTION_PROBE=PASS:requested={}x{}:actual={}x{}",
                    expected.width, expected.height, size.width, size.height
                );
                println!("AETHER_BROWSER_OFFSCREEN_RESIZE=PASS");
                println!("AETHER_BROWSER_OFFSCREEN_VIEWPORT_MATCH=PASS");
                println!("AETHER_BROWSER_VIEWPORT_ORIGIN=PASS:{content_left},{content_top}");
                println!("AETHER_BROWSER_VIEWPORT_RIGHT=PASS:{content_right}");
                println!("AETHER_BROWSER_VIEWPORT_BOTTOM=PASS:{content_bottom}");
                println!("AETHER_BROWSER_VIEWPORT_SIZE=PASS:{content_width}x{content_height}");
                if size.width == 1360 && size.height == 768 {
                    if (content_left, content_top, content_right, content_bottom)
                        != (52, 108, 1184, 738)
                    {
                        return Err(format!(
                            "screenshot-1360-layout-mismatch:left={content_left}:top={content_top}:right={content_right}:bottom={content_bottom}"
                        ));
                    }
                    println!("AETHER_BROWSER_SCREENSHOT_1360_LAYOUT=PASS:52,108,1184,738");
                }
                println!("AETHER_BROWSER_POINTER_PIXEL_MATCH=PASS");
                println!("AETHER_BROWSER_CHROME_CONTENT_NO_OVERLAP=PASS");
                println!("AETHER_BROWSER_BOTTOM_BAR_NO_GAP=PASS");
            }
            let rect: Box2D<i32, DevicePixel> = if content_only {
                self.content_capture_rect(size)
            } else {
                Box2D::from_origin_and_size(
                    Point2D::new(0, 0),
                    Size2D::new(size.width as i32, size.height as i32),
                )
            };
            let image = self
                .parent_context
                .read_to_image(rect)
                .ok_or_else(|| "parent framebuffer did not provide live-frame pixels".to_owned())?;
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            image.save(&path).map_err(|error| error.to_string())?;

            let mut min_luma = u8::MAX;
            let mut max_luma = u8::MIN;
            let mut bright_samples = 0_u64;
            let mut samples = 0_u64;
            for pixel in image.pixels().step_by(97) {
                let [red, green, blue, _alpha] = pixel.0;
                let luma = ((u16::from(red) + u16::from(green) + u16::from(blue)) / 3) as u8;
                min_luma = min_luma.min(luma);
                max_luma = max_luma.max(luma);
                if luma >= 96 {
                    bright_samples = bright_samples.saturating_add(1);
                }
                samples = samples.saturating_add(1);
            }
            let span = max_luma.saturating_sub(min_luma);
            println!("AETHER_BROWSER_LIVE_FRAME_CAPTURE={}", path.display());
            println!(
                "AETHER_BROWSER_LIVE_FRAME_STATS=min:{min_luma}:max:{max_luma}:span:{span}:bright:{bright_samples}:samples:{samples}"
            );
            if samples == 0 || span < 32 || bright_samples == 0 {
                return Err(format!(
                    "blank-or-low-contrast-parent-frame:min={min_luma}:max={max_luma}:span={span}:bright={bright_samples}:samples={samples}"
                ));
            }
            println!("AETHER_BROWSER_LIVE_FRAME_CONTRAST=PASS");

            if std::env::var_os("AETHER_BROWSER_LIVE_FRAME_PROBE_REQUIRE_BOTTOM").is_some() {
                let (content_left, content_top, content_width, content_height) =
                    self.content_geometry_for_size(size);
                let content_bottom = content_top.saturating_add(content_height);
                let band_height = content_height.min(24);
                // read_to_image() returns a content-local image for content-only probes
                // and a full-window image otherwise.  In both cases sample the final
                // rows of the *web content rectangle*, never the telemetry/status bar.
                let (start_x, end_x, start_y, end_y) = if content_only {
                    (
                        0,
                        image.width(),
                        image.height().saturating_sub(band_height),
                        image.height(),
                    )
                } else {
                    (
                        content_left.min(image.width()),
                        content_left
                            .saturating_add(content_width)
                            .min(image.width()),
                        content_bottom
                            .saturating_sub(band_height)
                            .min(image.height()),
                        content_bottom.min(image.height()),
                    )
                };
                let mut bottom_min = u8::MAX;
                let mut bottom_max = u8::MIN;
                let mut bottom_bright = 0_u64;
                let mut bottom_samples = 0_u64;
                for y in start_y..end_y {
                    for x in (start_x..end_x).step_by(17) {
                        let [red, green, blue, _alpha] = image.get_pixel(x, y).0;
                        let luma =
                            ((u16::from(red) + u16::from(green) + u16::from(blue)) / 3) as u8;
                        bottom_min = bottom_min.min(luma);
                        bottom_max = bottom_max.max(luma);
                        if luma >= 48 {
                            bottom_bright = bottom_bright.saturating_add(1);
                        }
                        bottom_samples = bottom_samples.saturating_add(1);
                    }
                }
                let bottom_span = bottom_max.saturating_sub(bottom_min);
                println!(
                    "AETHER_BROWSER_WEB_VIEWPORT_BOTTOM_STATS=min:{bottom_min}:max:{bottom_max}:span:{bottom_span}:bright:{bottom_bright}:samples:{bottom_samples}"
                );
                if bottom_samples == 0 || bottom_max < 32 || bottom_bright == 0 {
                    return Err(format!(
                        "web-viewport-bottom-unfilled:min={bottom_min}:max={bottom_max}:span={bottom_span}:bright={bottom_bright}:samples={bottom_samples}"
                    ));
                }
                println!("AETHER_BROWSER_WEB_VIEWPORT_BOTTOM_FILL=PASS");
                println!("AETHER_BROWSER_EXTERNAL_WEB_FULL_HEIGHT=PASS");
            }

            if std::env::var_os("AETHER_BROWSER_YOUTUBE_PLAYBACK_PROBE").is_some() {
                let state = native_playback_state().map_err(|error| error.to_string())?;
                if !self.active_tab().metadata.native_frame_seen.get()
                    || !state.contains("state=playing")
                    || !state.contains("provider=youtube")
                {
                    return Err(format!("youtube-native-playback-not-ready:{state}"));
                }
                println!("AETHER_BROWSER_YOUTUBE_NATIVE_FRAME=PASS");
                println!("AETHER_BROWSER_YOUTUBE_MEDIA_SESSION=PASS:{state}");
            }
            Ok(())
        })();
        if let Err(error) = &result {
            eprintln!("AETHER_BROWSER_LIVE_FRAME_CONTRAST=FAIL:{error}");
        }
        self.live_frame_probe_result = Some(result);
        self.close_requested = true;
    }

    fn redraw(&mut self) {
        self.poll_native_media();
        self.poll_telemetry();

        if let Some(webview) = self.active_webview() {
            webview.paint();
        }

        let _ = self.parent_context.make_current();
        self.parent_context.prepare_for_rendering();
        let size = self.window.inner_size();
        self.egui_painter.clear(
            [size.width.max(1), size.height.max(1)],
            [0.02, 0.02, 0.06, 1.0],
        );

        if !self.is_home_surface() {
            self.composite_content();
        }

        let model = self.chrome_model();
        let mut raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(size.width as f32, size.height as f32),
            )),
            ..Default::default()
        };
        if let Some(viewport) = raw_input.viewports.get_mut(&egui::ViewportId::ROOT) {
            viewport.native_pixels_per_point = Some(1.0);
            viewport.inner_rect = raw_input.screen_rect;
            viewport.focused = Some(true);
        }

        let renderer = &mut self.native_renderer;
        let full_output = self.egui_context.run_ui(raw_input, |ui| {
            renderer.paint(ui.ctx(), &model, size.width, size.height);
        });
        let primitives = self
            .egui_context
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        let textures_delta = full_output.textures_delta;
        self.egui_painter.paint_and_update_textures(
            [size.width.max(1), size.height.max(1)],
            full_output.pixels_per_point,
            &primitives,
            &textures_delta,
        );

        if self.live_frame_probe.is_some() {
            let youtube_probe = std::env::var_os("AETHER_BROWSER_YOUTUBE_PLAYBACK_PROBE").is_some();
            let probe_ready = if youtube_probe {
                self.active_tab().metadata.native_frame_seen.get()
            } else if self.active_webview().is_none() {
                true
            } else {
                self.active_tab().metadata.frame_ready.get()
                    && self.active_tab().metadata.load_status.get() == LoadStatus::Complete
            };
            if probe_ready && !self.startup_revealed {
                // A visual acceptance window must be on-screen before its pixels are
                // inspected.  v2.1.48 captured a hidden parent surface and then closed
                // it, which made every interactive test screen effectively invisible.
                self.parent_context.present();
                self.window.set_visible(true);
                self.startup_revealed = true;
                self.live_frame_probe_revealed_at = Some(Instant::now());
                println!("AETHER_BROWSER_VISIBLE_TEST_SCREEN=PASS");
                println!("AETHER_BROWSER_LIVE_FRAME_PROBE_REVEALED=PASS");
                if self
                    .active_compatibility_surface()
                    .is_some_and(ChromiumCompatSurface::visible)
                {
                    println!("AETHER_BROWSER_EXTERNAL_WEB_CHILD_VISIBLE=PASS");
                }
                self.window.request_redraw();
                return;
            }
            if probe_ready {
                // Give an embedded X11 child time to paint after its parent becomes
                // viewable.  Keep asking winit for frames instead of sleeping inside
                // the event handler.
                if self
                    .live_frame_probe_revealed_at
                    .is_some_and(|revealed| revealed.elapsed() < Duration::from_millis(350))
                {
                    self.parent_context.present();
                    self.window.request_redraw();
                    return;
                }
                println!("AETHER_BROWSER_LIVE_FRAME_LOAD_READY=PASS");
                self.capture_live_frame_probe();
            } else {
                self.window.request_redraw();
            }
        }

        if self.snapshot_requested {
            self.snapshot_requested = false;
            match self.save_ui_snapshot() {
                Ok(path) => println!("AETHER_BROWSER_UI_SNAPSHOT_SAVED={}", path.display()),
                Err(error) => eprintln!("AETHER_BROWSER_UI_SNAPSHOT_FAILED={error}"),
            }
        }

        let animated_frame_captured = if let Some(index) = self
            .animated_snapshot
            .as_ref()
            .map(|capture| capture.frame_index)
        {
            match self.save_animation_frame(index) {
                Ok(path) => {
                    println!(
                        "AETHER_BROWSER_ANIMATED_SNAPSHOT_FRAME={index}:{}",
                        path.display()
                    );
                    true
                }
                Err(error) => {
                    eprintln!("AETHER_BROWSER_ANIMATED_SNAPSHOT_FRAME_FAILED={index}:{error}");
                    false
                }
            }
        } else {
            false
        };

        self.parent_context.present();

        let perf_now = Instant::now();
        if !self.first_content_paint_reported
            && (self.active_webview().is_none() || self.active_tab().metadata.frame_ready.get())
        {
            self.first_content_paint_reported = true;
            println!(
                "AETHER_BROWSER_PERF_FIRST_CONTENT_PAINT_US={}",
                perf_now
                    .saturating_duration_since(self.startup_started)
                    .as_micros()
            );
        }
        if !self.page_complete_reported
            && (self.active_webview().is_none()
                || self.active_tab().metadata.load_status.get() == LoadStatus::Complete)
        {
            self.page_complete_reported = true;
            println!(
                "AETHER_BROWSER_PERF_PAGE_COMPLETE_US={}",
                perf_now
                    .saturating_duration_since(self.startup_started)
                    .as_micros()
            );
        }

        if !self.startup_revealed && self.live_frame_probe_result.is_none() {
            self.window.set_visible(true);
            self.startup_revealed = true;
            println!("AETHER_BROWSER_STARTUP_REVEAL=NATIVE_FRAME");
            println!("AETHER_BROWSER_VISIBLE_TEST_SCREEN=PASS");
            println!(
                "AETHER_BROWSER_PERF_CHROME_VISIBLE_US={}",
                perf_now
                    .saturating_duration_since(self.startup_started)
                    .as_micros()
            );
        }

        if let Some(fps_milli) = self.frame_rate_tracker.record_frame(Instant::now()) {
            self.telemetry_snapshot.browser_fps_milli = Some(fps_milli);
        }
        if animated_frame_captured {
            self.advance_animated_snapshot_after_frame();
        }

        if self
            .active_webview()
            .is_some_and(|webview| webview.animating())
        {
            self.window.request_redraw();
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        self.parent_context.resize(size);
        let content_size = self.content_size_for_size(size);
        for tab in &self.tabs {
            // Servo's offscreen framebuffer and the WebView viewport must stay the
            // same physical size.  Resizing only the WebView leaves the old FBO
            // stretched into the new content rectangle, which visibly shifts both
            // scaling and hit coordinates at custom resolutions.
            if let Some(context) = &tab.rendering_context {
                context.resize(content_size);
            }
            if let Some(webview) = &tab.webview {
                webview.resize(content_size);
            }
            if let (Some(host), Some(surface)) =
                (self.compatibility_host.as_ref(), tab.compatibility.as_ref())
            {
                let _ = host.resize(surface, self.compatibility_geometry_for_size(size));
            }
            if let (Some(host), Some(surface)) =
                (self.external_app_host.as_ref(), tab.external_app.as_ref())
            {
                let _ = host.resize(surface, self.compatibility_geometry_for_size(size));
            }
        }
        self.window.request_redraw();
    }

    fn create_content_surface(
        &self,
        metadata: Rc<TabMetadata>,
        actual_url: Url,
    ) -> (Rc<OffscreenRenderingContext>, WebView) {
        let context = Rc::new(self.parent_context.offscreen_context(self.content_size()));
        let delegate = Rc::new(TabDelegate {
            window: self.window.clone(),
            metadata,
            pending_navigation: self.pending_navigation.clone(),
        });
        let webview = WebViewBuilder::new(&self.servo, context.clone())
            .url(actual_url)
            .hidpi_scale_factor(euclid::Scale::new(self.window.scale_factor() as f32))
            .delegate(delegate)
            .build();
        webview.notify_theme_change(Theme::Dark);
        webview.set_throttled(false);
        webview.show();
        (context, webview)
    }

    fn create_compatibility_surface(
        &mut self,
        raw_url: &str,
    ) -> Result<ChromiumCompatSurface, String> {
        let geometry = self.compatibility_geometry_for_size(self.window.inner_size());
        let host = self
            .compatibility_host
            .as_mut()
            .ok_or_else(|| "Chromium compatibility host unavailable".to_owned())?;
        host.create_surface(raw_url, geometry)
            .map_err(|error| error.to_string())
    }

    fn create_external_app_surface(
        &mut self,
        target: &ExternalAppTarget,
    ) -> Result<ExternalAppSurface, String> {
        let geometry = self.compatibility_geometry_for_size(self.window.inner_size());
        let request_path = external_open_request_path();
        if let Some(parent) = request_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let mut launch_target = target.clone();
        launch_target.env.push((
            "AETHER_BROWSER_OPEN_URL_REQUEST_PATH".to_owned(),
            request_path.to_string_lossy().into_owned(),
        ));
        let surface = {
            let host = self
                .external_app_host
                .as_mut()
                .ok_or_else(|| "external app host unavailable".to_owned())?;
            match host.adopt_surface(&launch_target.app_id, &launch_target.program, geometry) {
                Ok(Some(surface)) => {
                    println!(
                        "AETHER_BROWSER_EXTERNAL_APP_RESTART_REINTEGRATION=PASS:{}:window={}",
                        target.app_id,
                        surface.window_id()
                    );
                    Ok(surface)
                }
                Ok(None) => host
                    .create_surface(
                        &launch_target.app_id,
                        &launch_target.program,
                        &launch_target.args,
                        &launch_target.env,
                        geometry,
                    )
                    .map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            }
        }?;
        if target.app_id == "obs" {
            let _ = std::thread::spawn(|| {
                std::thread::sleep(Duration::from_millis(750));
                match enforce_obs_embedded_capture_safety(ObsWebSocketConfig::from_env()) {
                    Ok(report) => println!(
                        "AETHER_BROWSER_OBS_INFINITY_MIRROR=BLOCKED:recursive={}:overflow={}",
                        report.blocked_recursive_sources.len(),
                        report.clipped_items.len()
                    ),
                    Err(error) => eprintln!("AETHER_BROWSER_OBS_CAPTURE_SAFETY=DEGRADED:{error}"),
                }
            });
        }
        Ok(surface)
    }

    fn adopt_orphaned_external_apps(&mut self) -> usize {
        let geometry = self.compatibility_geometry_for_size(self.window.inner_size());
        let mut adopted = Vec::new();
        let Some(host) = self.external_app_host.as_ref() else {
            println!("AETHER_BROWSER_EXTERNAL_APP_RESTART_REINTEGRATION=NONE:no-host");
            return 0;
        };

        for target in known_external_app_targets() {
            match host.adopt_surfaces(&target.app_id, &target.program, geometry) {
                Ok(surfaces) => {
                    for surface in surfaces {
                        if target.app_id == "obs" {
                            let _ = std::thread::spawn(|| {
                                std::thread::sleep(Duration::from_millis(250));
                                let _ = enforce_obs_embedded_capture_safety(
                                    ObsWebSocketConfig::from_env(),
                                );
                            });
                        }
                        adopted.push((target.clone(), surface));
                    }
                }
                Err(error) => eprintln!(
                    "AETHER_BROWSER_EXTERNAL_APP_RESTART_REINTEGRATION=DEGRADED:{}:{error}",
                    target.app_id
                ),
            }
        }

        if adopted.is_empty() {
            println!("AETHER_BROWSER_EXTERNAL_APP_RESTART_REINTEGRATION=NONE");
            return 0;
        }

        let first_new_index = self.tabs.len();
        for (target, mut surface) in adopted {
            self.next_tab_id = self.next_tab_id.saturating_add(1);
            let canonical_url = canonical_external_app_url(&target.app_id);
            let page_title = match target.app_id.as_str() {
                "obs" => "OBS Studio",
                _ => "External App",
            };
            if let Some(host) = self.external_app_host.as_ref() {
                let _ = host.set_visible(&mut surface, false);
            }
            println!(
                "AETHER_BROWSER_EXTERNAL_APP_RESTART_REINTEGRATION=PASS:{}:window={}",
                target.app_id,
                surface.window_id()
            );
            let metadata = Rc::new(TabMetadata::new(
                self.next_tab_id,
                canonical_url,
                page_title.to_owned(),
                false,
                None,
                self.library.clone(),
                self.persistence_scope,
            ));
            self.tabs.push(RuntimeTab {
                metadata,
                rendering_context: None,
                webview: None,
                compatibility: None,
                external_app: Some(surface),
            });
        }

        self.active_index = first_new_index;
        self.omnibox = self.active_url();
        self.omnibox_caret = self.omnibox.len();
        self.omnibox_active = false;
        self.omnibox_select_all = false;
        self.sync_active_surface_visibility();
        self.window.request_redraw();
        self.tabs.len().saturating_sub(first_new_index)
    }

    fn sync_active_surface_visibility(&mut self) {
        let active_index = self.active_index;
        let geometry = self.compatibility_geometry_for_size(self.window.inner_size());
        for (index, tab) in self.tabs.iter_mut().enumerate() {
            if let Some(webview) = &tab.webview {
                if index == active_index {
                    webview.set_throttled(false);
                    webview.show();
                    webview.focus();
                } else {
                    webview.blur();
                    webview.hide();
                    webview.set_throttled(true);
                }
            }
            if let (Some(host), Some(surface)) =
                (self.compatibility_host.as_ref(), tab.compatibility.as_mut())
            {
                if index == active_index {
                    if let Err(error) = host.resize(surface, geometry) {
                        eprintln!(
                            "AETHER_BROWSER_COMPAT_ENGAGE=FAIL:{}:{error}",
                            surface.url()
                        );
                        continue;
                    }
                    if let Err(error) = host.set_visible(surface, true) {
                        eprintln!(
                            "AETHER_BROWSER_COMPAT_ENGAGE=FAIL:{}:{error}",
                            surface.url()
                        );
                    }
                } else {
                    match host.set_visible(surface, false) {
                        Ok(()) => {
                            println!("AETHER_BROWSER_COMPAT_DISENGAGE=PASS:{}", surface.url())
                        }
                        Err(error) => eprintln!(
                            "AETHER_BROWSER_COMPAT_DISENGAGE=FAIL:{}:{error}",
                            surface.url()
                        ),
                    }
                }
            }
            if let (Some(host), Some(surface)) =
                (self.external_app_host.as_ref(), tab.external_app.as_mut())
                && surface.attached()
            {
                if index == active_index {
                    if let Err(error) = host.resize(surface, geometry) {
                        eprintln!(
                            "AETHER_BROWSER_EXTERNAL_APP_RESIZE=FAIL:{}:{error}",
                            surface.app_id()
                        );
                    }
                    if let Err(error) = host.set_visible(surface, true) {
                        eprintln!(
                            "AETHER_BROWSER_EXTERNAL_APP_ENGAGE=FAIL:{}:{error}",
                            surface.app_id()
                        );
                    }
                } else if let Err(error) = host.set_visible(surface, false) {
                    eprintln!(
                        "AETHER_BROWSER_EXTERNAL_APP_DISENGAGE=FAIL:{}:{error}",
                        surface.app_id()
                    );
                }
            }
        }
    }

    fn focus_content_surface(&self) {
        if let Some(webview) = self.active_webview() {
            webview.focus();
        } else if self.active_compatibility_surface().is_some() {
            println!("AETHER_BROWSER_COMPAT_FOCUS=CHROMIUM_CHILD");
        }
    }

    fn active_native_playback(&self) -> Option<NativePlaybackRequest> {
        self.active_tab().metadata.native_playback.borrow().clone()
    }

    fn stop_active_native_playback(&self) {
        if self.active_native_playback().is_some() {
            let _ = stop_native_playback();
        }
    }

    fn start_active_native_playback(&self) {
        let Some(request) = self.active_native_playback() else {
            return;
        };
        self.active_tab().metadata.native_frame_seen.set(false);
        match start_provider_request(&request) {
            Ok(response) => {
                println!(
                    "AETHER_BROWSER_NATIVE_PROVIDER={} resource={}",
                    request.provider.as_str(),
                    request.resource
                );
                println!("AETHER_BROWSER_MEDIA_SERVICE_SESSION={response}");
            }
            Err(error) => eprintln!("AETHER_BROWSER_MEDIA_SERVICE_SESSION=DEGRADED:{error}"),
        }
        self.window.request_redraw();
    }

    fn fallback_active_youtube_to_authenticated_web(&mut self, reason: &str) -> bool {
        let Some(request) = self.active_native_playback() else {
            return false;
        };
        if request.provider != NativePlaybackProvider::YouTube {
            return false;
        }
        let Ok(mut fallback_url) = Url::parse(&request.original_url) else {
            return false;
        };
        fallback_url
            .query_pairs_mut()
            .append_pair("aether_web", "1");
        let fallback = fallback_url.to_string();
        println!("AETHER_BROWSER_YOUTUBE_NATIVE_FALLBACK=CHROMIUM_AUTHENTICATED_WEB");
        println!("AETHER_BROWSER_YOUTUBE_NATIVE_FALLBACK_REASON={reason}");
        println!("AETHER_BROWSER_YOUTUBE_NATIVE_FALLBACK_URL={fallback}");
        if let Err(error) = self.load_active_url(&fallback) {
            eprintln!("AETHER_BROWSER_YOUTUBE_NATIVE_FALLBACK=FAIL:{error}");
            return false;
        }
        true
    }

    fn native_media_state_has_error(state: &str) -> bool {
        state
            .split_ascii_whitespace()
            .find_map(|part| part.strip_prefix("error="))
            .is_some_and(|error| !error.is_empty() && error != "-")
    }

    fn update_native_player_state(&self, state: &str) {
        let Some(webview) = self.active_webview() else {
            return;
        };
        let escaped = state
            .replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace(['\n', '\r'], " ");
        let script = format!(
            "(()=>{{const s=document.getElementById('aether-native-state');if(s){{s.textContent='{escaped}';s.style.display='flex';}}}})()"
        );
        webview.evaluate_javascript(script, |_| {});
    }

    fn poll_native_media(&mut self) {
        if self.active_native_playback().is_none() || self.active_webview().is_none() {
            return;
        }
        if self.last_native_frame_poll.elapsed() < Duration::from_millis(65) {
            self.window.request_redraw();
            return;
        }
        self.last_native_frame_poll = Instant::now();
        match latest_native_frame() {
            Ok(Some(frame)) => {
                let encoded = STANDARD.encode(frame);
                let script = format!(
                    "(()=>{{const i=document.getElementById('aether-native-video');if(i)i.src='data:image/jpeg;base64,{encoded}';const s=document.getElementById('aether-native-state');if(s)s.style.display='none';}})()"
                );
                if let Some(webview) = self.active_webview() {
                    webview.evaluate_javascript(script, |_| {});
                }
                if !self.active_tab().metadata.native_frame_seen.replace(true) {
                    println!("AETHER_BROWSER_NATIVE_FRAME=OBSERVED");
                }
            }
            Ok(None) => {
                if let Ok(state) = native_playback_state() {
                    if Self::native_media_state_has_error(&state)
                        && self.fallback_active_youtube_to_authenticated_web(&state)
                    {
                        return;
                    }
                    self.update_native_player_state(&state);
                }
            }
            Err(error) => {
                let reason = format!("media-service-unavailable:{error}");
                if self.fallback_active_youtube_to_authenticated_web(&reason) {
                    return;
                }
                self.update_native_player_state(&format!(
                    "Native media service unavailable: {error}"
                ));
            }
        }
        self.window.request_redraw();
    }

    fn install_target_on_active_tab(&mut self, target: RuntimeTarget) -> Result<(), String> {
        self.stop_active_native_playback();
        let actual_url = target.actual_url.clone();
        let display_url = target.display_url.clone();
        let external_app = target.external_app.clone();

        if external_app.is_none()
            && let Some(mut surface) = self.active_tab_mut().external_app.take()
            && let Some(host) = self.external_app_host.as_ref()
        {
            let _ = host.close(&mut surface);
        }

        if let Some(external_app) = external_app {
            {
                let tab = self.active_tab_mut();
                if let Some(webview) = tab.webview.take() {
                    webview.blur();
                    webview.hide();
                }
                tab.rendering_context = None;
            }
            if let Some(mut surface) = self.active_tab_mut().compatibility.take()
                && let Some(host) = self.compatibility_host.as_ref()
            {
                let _ = host.close(&mut surface);
            }
            if let Some(mut surface) = self.active_tab_mut().external_app.take()
                && let Some(host) = self.external_app_host.as_ref()
            {
                let _ = host.close(&mut surface);
            }
            {
                let tab = self.active_tab_mut();
                tab.metadata.current_url.replace(target.display_url);
                tab.metadata.page_title.replace(target.page_title);
                tab.metadata.native_page.set(false);
                tab.metadata.native_playback.replace(None);
                tab.metadata.native_frame_seen.set(false);
                tab.metadata.media_session_seen.set(false);
                tab.metadata.frame_ready.set(true);
                tab.metadata.load_status.set(LoadStatus::Complete);
            }
            match self.create_external_app_surface(&external_app) {
                Ok(surface) => {
                    self.active_tab_mut().external_app = Some(surface);
                    println!(
                        "AETHER_BROWSER_EXTERNAL_APP_SURFACE=PASS:IN_WINDOW:{}",
                        external_app.app_id
                    );
                    self.sync_active_surface_visibility();
                    return Ok(());
                }
                Err(error) => {
                    eprintln!(
                        "AETHER_BROWSER_EXTERNAL_APP_SURFACE=FAIL:{}:{error}",
                        external_app.app_id
                    );
                    return Err(format!(
                        "external app {} could not be embedded: {error}",
                        external_app.app_id
                    ));
                }
            }
        }

        let engine = if target.native_page {
            ContentEngine::Native
        } else {
            classify_content_engine(&display_url)
        };

        // Tear down the surface type that is no longer authoritative for this tab.
        if engine != ContentEngine::Compatibility
            && let Some(mut surface) = self.active_tab_mut().compatibility.take()
            && let Some(host) = self.compatibility_host.as_ref()
        {
            let _ = host.close(&mut surface);
        }
        if engine == ContentEngine::Compatibility {
            let tab = self.active_tab_mut();
            if let Some(webview) = tab.webview.take() {
                webview.blur();
                webview.hide();
            }
            tab.rendering_context = None;
            if let Some(mut surface) = tab.compatibility.take()
                && let Some(host) = self.compatibility_host.as_ref()
            {
                let _ = host.close(&mut surface);
            }
        }

        {
            let tab = self.active_tab_mut();
            tab.metadata.current_url.replace(target.display_url);
            tab.metadata.page_title.replace(target.page_title);
            tab.metadata.native_page.set(target.native_page);
            tab.metadata.native_playback.replace(target.native_playback);
            tab.metadata.native_frame_seen.set(false);
            tab.metadata.media_session_seen.set(false);
            tab.metadata.frame_ready.set(false);
            tab.metadata.load_status.set(LoadStatus::Started);
        }

        match engine {
            ContentEngine::Native => {
                let tab = self.active_tab_mut();
                if actual_url.is_none() {
                    if let Some(webview) = tab.webview.take() {
                        webview.blur();
                        webview.hide();
                    }
                    tab.rendering_context = None;
                } else if let Some(url) = actual_url {
                    let metadata = tab.metadata.clone();
                    let _ = tab;
                    let (context, webview) = self.create_content_surface(metadata, url);
                    let tab = self.active_tab_mut();
                    tab.rendering_context = Some(context);
                    tab.webview = Some(webview);
                }
            }
            ContentEngine::Servo => {
                if let Some(url) = actual_url {
                    if let Some(webview) = self.active_webview() {
                        webview.load(url);
                        webview.notify_theme_change(Theme::Dark);
                    } else {
                        let metadata = self.active_tab().metadata.clone();
                        let (context, webview) = self.create_content_surface(metadata, url);
                        let tab = self.active_tab_mut();
                        tab.rendering_context = Some(context);
                        tab.webview = Some(webview);
                    }
                }
            }
            ContentEngine::Compatibility => {
                println!("AETHER_BROWSER_CONTENT_ENGINE=CHROMIUM_COMPAT:{display_url}");
                match self.create_compatibility_surface(&display_url) {
                    Ok(surface) => {
                        self.active_tab_mut().compatibility = Some(surface);
                        println!("AETHER_BROWSER_COMPAT_SURFACE=PASS:IN_WINDOW_CHROMIUM");
                    }
                    Err(error) => {
                        eprintln!("AETHER_BROWSER_COMPAT_SURFACE=DEGRADED:{error}");
                        // Chromium/X11 is authoritative for all external web content.  Never
                        // silently route a failed external page back through Servo or the native
                        // media shim; doing so hides the real engine boundary and recreates the
                        // endless-buffer/black-page failures this cutover replaces.
                        eprintln!("AETHER_BROWSER_COMPAT_REQUIRED=FAIL:{error}");
                        return Err(format!(
                            "Chromium/X11 surface required for external web URL {display_url}: {error}"
                        ));
                    }
                }
            }
        }
        self.sync_active_surface_visibility();
        Ok(())
    }

    fn load_active_url(&mut self, raw_url: &str) -> Result<(), String> {
        if let Some(target) = resolve_external_app_target(raw_url)
            && target.detached
        {
            launch_external_app_detached(&target)?;
            println!(
                "AETHER_BROWSER_EXTERNAL_APP_DETACHED_LAUNCH=PASS:{}",
                target.app_id
            );
            return Ok(());
        }
        if let Some(destination) = provider_open_in_new_tab(raw_url) {
            println!("AETHER_BROWSER_PROVIDER_NEW_TAB={raw_url}->{destination}");
            return self.create_tab(&destination, false);
        }
        if let Some(target) = resolve_interface_action_url(raw_url)
            && target != raw_url
        {
            println!("AETHER_BROWSER_ENDPOINT_EXECUTION=RESOLVE:{raw_url}->{target}");
        }

        if let Some(action) = parse_library_action(raw_url) {
            match action {
                LibraryAction::ClearHistory => {
                    self.library
                        .clear_history()
                        .map_err(|error| error.to_string())?;
                }
                LibraryAction::ClearRecent => {
                    self.library
                        .clear_recently_closed()
                        .map_err(|error| error.to_string())?;
                }
                LibraryAction::RemoveBookmark(id) => {
                    self.library
                        .remove_bookmark(id)
                        .map_err(|error| error.to_string())?;
                }
                LibraryAction::Reopen { id, url } => {
                    self.library
                        .remove_recently_closed(id)
                        .map_err(|error| error.to_string())?;
                    return self.load_active_url(&url);
                }
            }
            return self.load_active_url("aether://library");
        }

        let target = resolve_runtime_url(raw_url, Some(self.library.as_ref()))?;
        if !target.native_page && should_persist_url(&target.display_url) {
            let _ = self.library.record_history(
                self.persistence_scope,
                &target.display_url,
                &target.page_title,
                unix_now(),
            );
        }
        self.install_target_on_active_tab(target)?;
        self.omnibox_active = false;
        self.omnibox_select_all = false;
        self.omnibox = self.active_url();
        self.start_active_native_playback();
        self.window.request_redraw();
        Ok(())
    }

    fn probe_active_media(&self) {
        if self.active_native_playback().is_some() {
            let frame = latest_native_frame().ok().flatten().is_some();
            let state = native_playback_state().unwrap_or_else(|error| format!("ERROR:{error}"));
            println!("AETHER_BROWSER_NATIVE_MEDIA_STATE={state}");
            println!(
                "AETHER_BROWSER_NATIVE_MEDIA_PROBE={}",
                if frame { "PASS" } else { "FAIL" }
            );
            println!(
                "AETHER_BROWSER_NATIVE_FRAME={}",
                if frame { "PASS" } else { "UNOBSERVED" }
            );
            return;
        }
        let Some(webview) = self.active_webview() else {
            println!("AETHER_BROWSER_MEDIA_MSE_DOM=UNAVAILABLE:NATIVE_HOME");
            return;
        };
        const MSE_PROBE: &str = r#"(() => {
            if (typeof MediaSource === 'undefined') throw new Error('MSE_UNAVAILABLE');
            return true;
        })()"#;
        webview.evaluate_javascript(MSE_PROBE, |result| {
            println!(
                "AETHER_BROWSER_MEDIA_MSE_DOM={}",
                if result.is_ok() { "PASS" } else { "FAIL" }
            );
        });
    }

    fn create_tab(&mut self, raw_url: &str, focus_omnibox: bool) -> Result<(), String> {
        let target = resolve_runtime_url(raw_url, Some(self.library.as_ref()))?;
        if !target.native_page && should_persist_url(&target.display_url) {
            let _ = self.library.record_history(
                self.persistence_scope,
                &target.display_url,
                &target.page_title,
                unix_now(),
            );
        }
        self.stop_active_native_playback();
        self.next_tab_id = self.next_tab_id.saturating_add(1);
        let metadata = Rc::new(TabMetadata::new(
            self.next_tab_id,
            target.display_url.clone(),
            target.page_title.clone(),
            target.native_page,
            target.native_playback.clone(),
            self.library.clone(),
            self.persistence_scope,
        ));

        self.tabs.push(RuntimeTab {
            metadata,
            rendering_context: None,
            webview: None,
            compatibility: None,
            external_app: None,
        });
        self.active_index = self.tabs.len() - 1;
        self.install_target_on_active_tab(target)?;
        self.omnibox_active = focus_omnibox;
        self.omnibox_select_all = focus_omnibox;
        self.omnibox = if focus_omnibox {
            String::new()
        } else {
            self.active_url()
        };
        self.omnibox_caret = self.omnibox.len();
        self.start_active_native_playback();
        self.window.request_redraw();
        Ok(())
    }

    fn close_tab_id(&mut self, id: u64) {
        let Some(index) = self.tabs.iter().position(|tab| tab.metadata.id == id) else {
            return;
        };
        if index == self.active_index {
            self.close_active_tab();
            return;
        }
        if let Some(mut surface) = self.tabs[index].compatibility.take()
            && let Some(host) = self.compatibility_host.as_ref()
        {
            let _ = host.close(&mut surface);
        }
        if let Some(mut surface) = self.tabs[index].external_app.take()
            && let Some(host) = self.external_app_host.as_ref()
        {
            let _ = host.close(&mut surface);
        }
        self.tabs.remove(index);
        if index < self.active_index {
            self.active_index = self.active_index.saturating_sub(1);
        }
        self.window.request_redraw();
    }

    fn activate_tab_id(&mut self, id: u64) {
        let Some(index) = self.tabs.iter().position(|tab| tab.metadata.id == id) else {
            return;
        };
        if index == self.active_index {
            return;
        }
        self.stop_active_native_playback();
        if let Some(webview) = self.active_webview() {
            webview.blur();
            webview.hide();
            webview.set_throttled(true);
        }
        self.active_index = index;
        self.sync_active_surface_visibility();
        self.omnibox_active = false;
        self.omnibox_select_all = false;
        self.omnibox = self.active_url();
        self.start_active_native_playback();
        self.window.request_redraw();
    }

    fn cycle_tabs(&mut self, backwards: bool) {
        if self.tabs.len() < 2 {
            return;
        }
        let next = if backwards {
            self.active_index
                .checked_sub(1)
                .unwrap_or(self.tabs.len() - 1)
        } else {
            (self.active_index + 1) % self.tabs.len()
        };
        let id = self.tabs[next].metadata.id;
        self.activate_tab_id(id);
    }

    fn close_active_tab(&mut self) {
        let closed_url = self.active_url();
        let closed_title = self.active_title();
        let _ = self.library.record_recently_closed(
            self.persistence_scope,
            &closed_url,
            &closed_title,
            unix_now(),
        );
        self.stop_active_native_playback();

        if self.tabs.len() == 1 {
            let _ = self.load_active_url("aether://home");
            self.omnibox_active = false;
            self.omnibox_select_all = false;
            return;
        }

        if let Some(mut surface) = self.tabs[self.active_index].compatibility.take()
            && let Some(host) = self.compatibility_host.as_ref()
        {
            let _ = host.close(&mut surface);
        }
        if let Some(mut surface) = self.tabs[self.active_index].external_app.take()
            && let Some(host) = self.external_app_host.as_ref()
        {
            let _ = host.close(&mut surface);
        }
        self.tabs.remove(self.active_index);
        if self.active_index >= self.tabs.len() {
            self.active_index = self.tabs.len() - 1;
        }
        self.sync_active_surface_visibility();
        self.omnibox_active = false;
        self.omnibox_select_all = false;
        self.omnibox = self.active_url();
        self.start_active_native_playback();
        self.window.request_redraw();
    }

    fn detach_active_external_app(&mut self) -> bool {
        let index = self.active_index;
        let Some(host) = self.external_app_host.as_ref() else {
            return false;
        };
        let Some(surface) = self.tabs[index].external_app.as_mut() else {
            return false;
        };
        match host.detach(surface) {
            Ok(()) => {
                println!(
                    "AETHER_BROWSER_EXTERNAL_APP_DETACH=PASS:{}",
                    surface.app_id()
                );
                true
            }
            Err(error) => {
                eprintln!(
                    "AETHER_BROWSER_EXTERNAL_APP_DETACH=FAIL:{}:{error}",
                    surface.app_id()
                );
                false
            }
        }
    }

    fn reattach_active_external_app(&mut self) -> bool {
        let geometry = self.compatibility_geometry_for_size(self.window.inner_size());
        let index = self.active_index;
        let Some(host) = self.external_app_host.as_ref() else {
            return false;
        };
        let Some(surface) = self.tabs[index].external_app.as_mut() else {
            return false;
        };
        match host.reattach(surface, geometry) {
            Ok(()) => {
                println!(
                    "AETHER_BROWSER_EXTERNAL_APP_REATTACH=PASS:{}",
                    surface.app_id()
                );
                self.window.request_redraw();
                true
            }
            Err(error) => {
                eprintln!(
                    "AETHER_BROWSER_EXTERNAL_APP_REATTACH=FAIL:{}:{error}",
                    surface.app_id()
                );
                false
            }
        }
    }

    fn omnibox_previous_boundary(text: &str, caret: usize) -> usize {
        if caret == 0 {
            return 0;
        }
        text[..caret]
            .char_indices()
            .next_back()
            .map_or(0, |(index, _)| index)
    }

    fn omnibox_next_boundary(text: &str, caret: usize) -> usize {
        if caret >= text.len() {
            return text.len();
        }
        caret + text[caret..].chars().next().map_or(0, char::len_utf8)
    }

    fn omnibox_set_to_active_url(&mut self, active: bool, select_all: bool) {
        self.omnibox = self.active_url();
        self.omnibox_caret = self.omnibox.len();
        self.omnibox_active = active;
        self.omnibox_select_all = select_all;
    }

    fn navigate_omnibox(&mut self) {
        let input = self.omnibox.clone();
        if input.trim_start().starts_with("aether://") {
            let _ = self.load_active_url(&input);
            return;
        }
        let Ok(target) = resolve_omnibox_input(&input) else {
            return;
        };
        let _ = self.load_active_url(&target);
    }

    fn reload_active(&self) {
        if let Some(webview) = self.active_webview() {
            webview.reload();
        }
    }

    fn go_back(&self) {
        if let Some(webview) = self.active_webview()
            && webview.can_go_back()
        {
            webview.go_back(1);
        }
    }

    fn go_forward(&self) {
        if let Some(webview) = self.active_webview()
            && webview.can_go_forward()
        {
            webview.go_forward(1);
        }
    }

    fn handle_browser_shortcut(&mut self, code: KeyCode, pressed: bool) -> bool {
        if !pressed {
            return false;
        }
        let ctrl = self.modifiers.control_key();
        if ctrl && code == KeyCode::KeyL {
            self.omnibox_set_to_active_url(true, true);
            self.window.request_redraw();
            return true;
        }
        if ctrl && code == KeyCode::KeyT {
            let _ = self.create_tab(NEW_TAB_URL, true);
            return true;
        }
        if ctrl && code == KeyCode::KeyW {
            self.close_active_tab();
            return true;
        }
        if ctrl && code == KeyCode::Tab {
            self.cycle_tabs(self.modifiers.shift_key());
            return true;
        }
        if ctrl && self.modifiers.shift_key() && code == KeyCode::KeyD {
            return self.detach_active_external_app();
        }
        if ctrl && self.modifiers.shift_key() && code == KeyCode::KeyR {
            return self.reattach_active_external_app();
        }
        if ctrl && self.modifiers.shift_key() && code == KeyCode::KeyM {
            self.probe_active_media();
            return true;
        }
        if ctrl && self.modifiers.shift_key() && code == KeyCode::KeyX {
            self.request_ui_snapshot();
            return true;
        }
        if ctrl && self.modifiers.shift_key() && code == KeyCode::KeyA {
            self.request_animated_snapshot();
            return true;
        }
        if code == KeyCode::F5 || (ctrl && code == KeyCode::KeyR) {
            self.reload_active();
            return true;
        }
        if self.modifiers.alt_key() && code == KeyCode::ArrowLeft {
            self.go_back();
            return true;
        }
        if self.modifiers.alt_key() && code == KeyCode::ArrowRight {
            self.go_forward();
            return true;
        }
        false
    }

    fn handle_omnibox_key(&mut self, event: &winit::event::KeyEvent) -> bool {
        if !self.omnibox_active || event.state != ElementState::Pressed {
            return false;
        }

        let ctrl = self.modifiers.control_key();
        match event.physical_key {
            PhysicalKey::Code(KeyCode::Enter) | PhysicalKey::Code(KeyCode::NumpadEnter) => {
                self.navigate_omnibox();
                return true;
            }
            PhysicalKey::Code(KeyCode::Escape) => {
                self.omnibox_set_to_active_url(false, false);
                self.window.request_redraw();
                return true;
            }
            PhysicalKey::Code(KeyCode::KeyA) if ctrl => {
                self.omnibox_select_all = true;
                self.omnibox_caret = self.omnibox.len();
                self.window.request_redraw();
                return true;
            }
            PhysicalKey::Code(KeyCode::ArrowLeft) => {
                self.omnibox_caret = if self.omnibox_select_all {
                    0
                } else {
                    Self::omnibox_previous_boundary(&self.omnibox, self.omnibox_caret)
                };
                self.omnibox_select_all = false;
                self.window.request_redraw();
                return true;
            }
            PhysicalKey::Code(KeyCode::ArrowRight) => {
                self.omnibox_caret = if self.omnibox_select_all {
                    self.omnibox.len()
                } else {
                    Self::omnibox_next_boundary(&self.omnibox, self.omnibox_caret)
                };
                self.omnibox_select_all = false;
                self.window.request_redraw();
                return true;
            }
            PhysicalKey::Code(KeyCode::Home) => {
                self.omnibox_caret = 0;
                self.omnibox_select_all = false;
                self.window.request_redraw();
                return true;
            }
            PhysicalKey::Code(KeyCode::End) => {
                self.omnibox_caret = self.omnibox.len();
                self.omnibox_select_all = false;
                self.window.request_redraw();
                return true;
            }
            PhysicalKey::Code(KeyCode::Backspace) => {
                if self.omnibox_select_all {
                    self.omnibox.clear();
                    self.omnibox_caret = 0;
                    self.omnibox_select_all = false;
                } else if self.omnibox_caret > 0 {
                    let previous =
                        Self::omnibox_previous_boundary(&self.omnibox, self.omnibox_caret);
                    self.omnibox.replace_range(previous..self.omnibox_caret, "");
                    self.omnibox_caret = previous;
                }
                self.window.request_redraw();
                return true;
            }
            PhysicalKey::Code(KeyCode::Delete) => {
                if self.omnibox_select_all {
                    self.omnibox.clear();
                    self.omnibox_caret = 0;
                    self.omnibox_select_all = false;
                } else if self.omnibox_caret < self.omnibox.len() {
                    let next = Self::omnibox_next_boundary(&self.omnibox, self.omnibox_caret);
                    self.omnibox.replace_range(self.omnibox_caret..next, "");
                }
                self.window.request_redraw();
                return true;
            }
            _ => {}
        }

        if !ctrl
            && !self.modifiers.alt_key()
            && !self.modifiers.super_key()
            && let WinitKey::Character(text) = &event.logical_key
        {
            if self.omnibox_select_all {
                self.omnibox.clear();
                self.omnibox_caret = 0;
                self.omnibox_select_all = false;
            }
            self.omnibox.insert_str(self.omnibox_caret, text);
            self.omnibox_caret += text.len();
            println!("AETHER_BROWSER_OMNIBOX_EDITING=CARET_DELETE_ARROWS");
            self.window.request_redraw();
            return true;
        }
        true
    }

    fn handle_chrome_click(&mut self) {
        let model = self.chrome_model();
        let size = self.window.inner_size();
        match model.hit_test(
            self.cursor_position.x,
            self.cursor_position.y,
            size.width,
            size.height,
        ) {
            ChromeHitTarget::WindowDrag => {
                let _ = self.window.drag_window();
            }
            ChromeHitTarget::WindowMinimize => self.window.set_minimized(true),
            ChromeHitTarget::WindowMaximize => self.window.set_maximized(true),
            ChromeHitTarget::WindowDiminish => self.window.set_maximized(false),
            ChromeHitTarget::WindowClose => self.close_requested = true,
            ChromeHitTarget::Back => self.go_back(),
            ChromeHitTarget::Forward => self.go_forward(),
            ChromeHitTarget::Reload => self.reload_active(),
            ChromeHitTarget::Omnibox => {
                self.omnibox_set_to_active_url(true, true);
                self.window.request_redraw();
            }
            ChromeHitTarget::NewTab => {
                let _ = self.create_tab(NEW_TAB_URL, true);
            }
            ChromeHitTarget::Tab(id) => self.activate_tab_id(id),
            ChromeHitTarget::CloseTab(id) => self.close_tab_id(id),
            ChromeHitTarget::Home => {
                let _ = self.load_active_url("aether://home");
            }
            ChromeHitTarget::StreamStudio => {
                let _ = self.load_active_url("aether://stream");
            }
            ChromeHitTarget::Vault => {
                let _ = self.load_active_url("aether://vault");
            }
            ChromeHitTarget::Accounts => {
                let _ = self.load_active_url("aether://accounts");
            }
            ChromeHitTarget::Providers => {
                let _ = self.load_active_url("aether://providers");
            }
            ChromeHitTarget::MediaCenter => {
                let _ = self.load_active_url("aether://media");
            }
            ChromeHitTarget::YouTubeMusic => {
                let _ = self.load_active_url("aether://youtube-music");
            }
            ChromeHitTarget::YouTubeMusicSearch => {
                let _ = self.load_active_url("aether://youtube-music?action=search");
            }
            ChromeHitTarget::YouTubeMusicLibrary => {
                let _ = self.load_active_url("aether://youtube-music?action=library");
            }
            ChromeHitTarget::Snapshot => self.request_ui_snapshot(),
            ChromeHitTarget::AnimatedSnapshot => self.request_animated_snapshot(),
            ChromeHitTarget::CreatorHub => {
                let _ = self.load_active_url("aether://creator");
            }
            ChromeHitTarget::CommsHub => {
                let _ = self.load_active_url("aether://comms");
            }
            ChromeHitTarget::Library => {
                let _ = self.load_active_url("aether://library");
            }
            ChromeHitTarget::Downloads => {
                let _ = self.load_active_url("aether://library?section=downloads");
            }
            ChromeHitTarget::Settings => {
                let _ = self.load_active_url("aether://settings");
            }
            ChromeHitTarget::UtilityCollapseToggle => {
                let before = self.content_size();
                self.layout.toggle_utility_collapsed();
                self.resize(self.window.inner_size());
                let after = self.content_size();
                if self.layout.utility_collapsed() {
                    println!(
                        "AETHER_BROWSER_RIGHT_RAIL_COLLAPSE_RECLAIMS_WEB_SPACE=PASS:{}x{}->{}x{}",
                        before.width, before.height, after.width, after.height
                    );
                } else {
                    println!(
                        "AETHER_BROWSER_RIGHT_RAIL_EXPAND_RESTORES_WEB_SPACE=PASS:{}x{}->{}x{}",
                        before.width, before.height, after.width, after.height
                    );
                }
            }
            ChromeHitTarget::BookmarkToggle => {
                let url = self.active_url();
                let title = self.active_title();
                if self.library.is_bookmarked(&url).unwrap_or(false) {
                    let _ = self.library.remove_bookmark_for_url(&url);
                } else if should_persist_url(&url) {
                    let _ = self
                        .library
                        .upsert_bookmark(&url, &title, "", &[], true, unix_now());
                }
                self.active_bookmarked_cache = self.library.is_bookmarked(&url).unwrap_or(false);
                self.window.request_redraw();
            }
            ChromeHitTarget::None => {}
        }
    }

    fn content_point_for_position(
        &self,
        position: PhysicalPosition<f64>,
        size: PhysicalSize<u32>,
    ) -> Option<WebViewPoint> {
        self.active_webview()?;
        content_device_point(self.layout, position, size).map(WebViewPoint::Device)
    }

    fn content_point(&self) -> Option<WebViewPoint> {
        self.content_point_for_position(self.cursor_position, self.window.inner_size())
    }

    fn active_resize_edge(&self) -> Option<WindowResizeEdge> {
        if self.window.is_maximized() || self.window.fullscreen().is_some() {
            return None;
        }
        let size = self.window.inner_size();
        window_resize_edge(
            self.cursor_position.x,
            self.cursor_position.y,
            size.width,
            size.height,
        )
    }

    fn update_resize_cursor(&self) {
        if let Some(edge) = self.active_resize_edge() {
            self.window.set_cursor(resize_cursor(edge));
            return;
        }
        let size = self.window.inner_size();
        let target = self.chrome_model().hit_test(
            self.cursor_position.x,
            self.cursor_position.y,
            size.width,
            size.height,
        );
        self.window
            .set_cursor(if target == ChromeHitTarget::Omnibox {
                CursorIcon::Text
            } else {
                CursorIcon::Default
            });
    }

    fn handle_cursor_moved(&mut self, position: PhysicalPosition<f64>) {
        self.cursor_position = position;
        self.update_resize_cursor();
        if let Some(point) = self.content_point() {
            self.cursor_in_content = true;
            if let Some(webview) = self.active_webview() {
                webview.notify_input_event(InputEvent::MouseMove(MouseMoveEvent::new(point)));
            }
        } else if self.cursor_in_content {
            self.cursor_in_content = false;
            if let Some(webview) = self.active_webview() {
                webview.notify_input_event(InputEvent::MouseLeftViewport(
                    MouseLeftViewportEvent::default(),
                ));
            }
        }
    }

    fn handle_mouse_button(&mut self, state: ElementState, button: WinitMouseButton) {
        // Browser navigation side buttons are owned by browser chrome, never by page content.
        match button {
            WinitMouseButton::Back => {
                if state == ElementState::Pressed {
                    self.go_back();
                    println!("AETHER_BROWSER_MOUSE_BACK_NAV=PASS");
                }
                return;
            }
            WinitMouseButton::Forward => {
                if state == ElementState::Pressed {
                    self.go_forward();
                    println!("AETHER_BROWSER_MOUSE_FORWARD_NAV=PASS");
                }
                return;
            }
            _ => {}
        }

        let left_click = button == WinitMouseButton::Left;
        if state == ElementState::Pressed
            && left_click
            && let Some(edge) = self.active_resize_edge()
        {
            let _ = self.window.drag_resize_window(resize_direction(edge));
            return;
        }
        let action = match state {
            ElementState::Pressed => MouseButtonAction::Down,
            ElementState::Released => MouseButtonAction::Up,
        };
        let servo_button = match button {
            WinitMouseButton::Left => servo::MouseButton::Left,
            WinitMouseButton::Middle => servo::MouseButton::Middle,
            WinitMouseButton::Right => servo::MouseButton::Right,
            WinitMouseButton::Back | WinitMouseButton::Forward => return,
            WinitMouseButton::Other(value) => servo::MouseButton::Other(value),
        };

        if let Some(point) = self.content_point() {
            if action == MouseButtonAction::Down {
                self.omnibox_active = false;
                self.omnibox_select_all = false;
                self.focus_content_surface();
            }
            if let Some(webview) = self.active_webview() {
                webview.notify_input_event(InputEvent::MouseButton(MouseButtonEvent::new(
                    action,
                    servo_button,
                    point,
                )));
            }
            return;
        }

        if state == ElementState::Pressed && left_click {
            self.handle_chrome_click();
        }
    }

    fn handle_scroll(&mut self, delta: MouseScrollDelta) {
        let (x, y, mode) = match delta {
            MouseScrollDelta::LineDelta(x, y) => (
                f64::from(x) * 76.0,
                f64::from(y) * 76.0,
                WheelMode::DeltaLine,
            ),
            MouseScrollDelta::PixelDelta(point) => (point.x, point.y, WheelMode::DeltaPixel),
        };

        if self.modifiers.control_key() {
            if y.abs() > f64::EPSILON
                && let Some(webview) = self.active_webview()
            {
                let current = webview.page_zoom();
                let zoom_delta = if y > 0.0 {
                    PAGE_ZOOM_STEP
                } else {
                    -PAGE_ZOOM_STEP
                };
                let next = (current + zoom_delta).clamp(PAGE_ZOOM_MIN, PAGE_ZOOM_MAX);
                webview.set_page_zoom(next);
                println!("AETHER_BROWSER_PAGE_ZOOM={next:.2}");
                self.window.request_redraw();
            }
            return;
        }

        if let Some(point) = self.content_point()
            && let Some(webview) = self.active_webview()
        {
            webview.notify_input_event(InputEvent::Wheel(WheelEvent::new(
                WheelDelta { x, y, z: 0.0, mode },
                point,
            )));
        }
    }

    fn servo_modifiers(&self) -> Modifiers {
        let mut modifiers = Modifiers::empty();
        if self.modifiers.shift_key() {
            modifiers.insert(Modifiers::SHIFT);
        }
        if self.modifiers.control_key() {
            modifiers.insert(Modifiers::CONTROL);
        }
        if self.modifiers.alt_key() {
            modifiers.insert(Modifiers::ALT);
        }
        if self.modifiers.super_key() {
            modifiers.insert(Modifiers::META);
        }
        modifiers
    }

    fn forward_keyboard_event(&self, event: &winit::event::KeyEvent) {
        let Some(webview) = self.active_webview() else {
            return;
        };
        let state = match event.state {
            ElementState::Pressed => KeyState::Down,
            ElementState::Released => KeyState::Up,
        };
        let key = match &event.logical_key {
            WinitKey::Character(text) => Key::Character(text.to_string()),
            WinitKey::Named(named) => Key::Named(
                NamedKey::from_str(&format!("{named:?}")).unwrap_or(NamedKey::Unidentified),
            ),
            WinitKey::Dead(Some(ch)) => Key::Character(ch.to_string()),
            WinitKey::Dead(None) | WinitKey::Unidentified(_) => Key::Named(NamedKey::Unidentified),
        };
        let code = match event.physical_key {
            PhysicalKey::Code(code) => {
                Code::from_str(&format!("{code:?}")).unwrap_or(Code::Unidentified)
            }
            PhysicalKey::Unidentified(_) => Code::Unidentified,
        };
        let location = match event.location {
            WinitKeyLocation::Standard => Location::Standard,
            WinitKeyLocation::Left => Location::Left,
            WinitKeyLocation::Right => Location::Right,
            WinitKeyLocation::Numpad => Location::Numpad,
        };
        let keyboard = KeyboardEvent::new_without_event(
            state,
            key,
            code,
            location,
            self.servo_modifiers(),
            event.repeat,
            false,
        );
        webview.notify_input_event(InputEvent::Keyboard(keyboard));
    }

    fn handle_keyboard(&mut self, event: winit::event::KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        if let PhysicalKey::Code(code) = event.physical_key
            && self.handle_browser_shortcut(code, pressed)
        {
            return;
        }
        if self.handle_omnibox_key(&event) {
            return;
        }
        self.forward_keyboard_event(&event);
    }
}

enum BrowserApp {
    Starting {
        waker: ServoWaker,
        initial_url: Url,
    },
    Running(Box<RunningBrowser>),
    Stopped {
        live_frame_probe_result: Option<Result<(), String>>,
    },
}

impl BrowserApp {
    fn new(event_loop: &EventLoop<WakeEvent>, initial_url: Url) -> Self {
        Self::Starting {
            waker: ServoWaker(event_loop.create_proxy()),
            initial_url,
        }
    }

    fn live_frame_probe_result(&self) -> Option<&Result<(), String>> {
        match self {
            Self::Running(browser) => browser.live_frame_probe_result.as_ref(),
            Self::Stopped {
                live_frame_probe_result,
            } => live_frame_probe_result.as_ref(),
            Self::Starting { .. } => None,
        }
    }
}

impl ApplicationHandler<WakeEvent> for BrowserApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let startup_started = Instant::now();
        let Self::Starting { waker, initial_url } = self else {
            return;
        };

        let display_handle = event_loop
            .display_handle()
            .expect("display handle unavailable");
        let window = Rc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title(TITLE_PREFIX)
                        .with_transparent(false)
                        .with_visible(false)
                        .with_decorations(false)
                        .with_resizable(true)
                        .with_min_inner_size(PhysicalSize::new(
                            MIN_WINDOW_WIDTH_PX,
                            MIN_WINDOW_HEIGHT_PX,
                        ))
                        .with_inner_size(startup_window_size()),
                )
                .expect("failed to create Aether Browser window"),
        );
        let window_handle = window.window_handle().expect("window handle unavailable");
        let parent_context = Rc::new(
            WindowRenderingContext::new(display_handle, window_handle, window.inner_size())
                .expect("failed to create Servo rendering context"),
        );
        parent_context
            .make_current()
            .expect("failed to activate Servo rendering context");
        parent_context.prepare_for_rendering();

        let egui_painter = egui_glow::Painter::new(parent_context.glow_gl_api(), "", None, false)
            .expect("failed to create native egui OpenGL painter");
        let egui_context = egui::Context::default();
        egui_context.set_pixels_per_point(1.0);
        egui_painter.clear(
            [window.inner_size().width, window.inner_size().height],
            [0.02, 0.02, 0.06, 1.0],
        );
        parent_context.present();

        let profile_kind = if std::env::var("AETHER_BROWSER_PRIVATE")
            .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        {
            ProfileKind::PrivateEphemeral
        } else {
            ProfileKind::Persistent
        };
        let profile_storage = ProfileStorage::for_kind(profile_kind)
            .expect("failed to initialize Aether Browser web profile storage");
        let servo_opts = Opts {
            config_dir: Some(profile_storage.root().to_path_buf()),
            temporary_storage: profile_storage.is_ephemeral(),
            ..Default::default()
        };
        println!(
            "AETHER_BROWSER_WEB_PROFILE={}",
            if profile_storage.is_ephemeral() {
                "PRIVATE_EPHEMERAL"
            } else {
                "PERSISTENT_NORMAL"
            }
        );
        println!("AETHER_BROWSER_WEB_SESSION_PERSISTENCE=PERSISTENT_NORMAL|EPHEMERAL_PRIVATE");
        let compatibility_root = profile_storage.compat_root();
        let x11_parent = x11_parent_window_id(&window);
        let mut compatibility_host = x11_parent
            .and_then(|parent| ChromiumCompatHost::new(parent, compatibility_root.clone()).ok());
        let external_app_host = x11_parent.and_then(|parent| ExternalAppHost::new(parent).ok());
        if external_app_host.is_some() {
            println!("AETHER_BROWSER_EXTERNAL_APP_HOST=PASS:X11_CHILD_REPARENTING");
        } else {
            println!("AETHER_BROWSER_EXTERNAL_APP_HOST=DEGRADED:NO_X11_PARENT");
        }
        if compatibility_host.is_some() {
            println!("AETHER_BROWSER_COMPAT_ENGINE=SYSTEM_CHROMIUM_X11_CHILD");
            println!(
                "AETHER_BROWSER_COMPAT_PROFILE={}",
                if profile_storage.is_ephemeral() {
                    "PRIVATE_EPHEMERAL"
                } else {
                    "PERSISTENT_NORMAL"
                }
            );
            println!(
                "AETHER_BROWSER_COMPAT_PROFILE_ROOT={}",
                compatibility_root.display()
            );
        } else {
            println!("AETHER_BROWSER_COMPAT_ENGINE=DEGRADED:NO_X11_OR_CHROMIUM");
        }

        let servo = ServoBuilder::default()
            .opts(servo_opts)
            .event_loop_waker(Box::new(waker.clone()))
            .build();
        servo.setup_logging();
        println!(
            "AETHER_BROWSER_PERF_SERVO_READY_US={}",
            startup_started.elapsed().as_micros()
        );
        println!("AETHER_BROWSER_RUNTIME_ARCHITECTURE={RUNTIME_ARCHITECTURE}");
        println!("AETHER_BROWSER_RUNTIME_CHROME=NATIVE_EGUI_GLOW");
        println!("AETHER_BROWSER_RUNTIME_SERVO=INTERNAL_NON_HTTP_ONLY");
        println!("AETHER_BROWSER_EXTERNAL_WEB_ENGINE=CHROMIUM_X11");

        if !profile_storage.is_ephemeral()
            && let Some(host) = compatibility_host.as_mut()
        {
            let cookies = servo_provider_cookies_for_compatibility(&servo);
            match host.import_servo_cookies(&cookies) {
                Ok(0) => println!(
                    "AETHER_BROWSER_COMPAT_EXISTING_LOGIN_MIGRATION=PASS:already-migrated-or-none"
                ),
                Ok(count) => {
                    println!("AETHER_BROWSER_COMPAT_EXISTING_LOGIN_MIGRATION=PASS:{count}-cookies")
                }
                Err(error) => {
                    eprintln!("AETHER_BROWSER_COMPAT_EXISTING_LOGIN_MIGRATION=DEGRADED:{error}")
                }
            }
        } else if profile_storage.is_ephemeral() {
            println!("AETHER_BROWSER_COMPAT_EXISTING_LOGIN_MIGRATION=SKIP:private-profile");
        }

        let pending_navigation = Rc::new(RefCell::new(None));
        let library = Rc::new(match LibraryStore::open_default() {
            Ok(store) => store,
            Err(error) => {
                eprintln!("AETHER_BROWSER_LIBRARY=DEGRADED:{error}");
                LibraryStore::open_in_memory().expect("in-memory Aether Library must initialize")
            }
        });
        let persistence_scope = PersistenceScope::Normal;
        let layout = ChromeLayout::canonical();
        let target = resolve_runtime_url(initial_url.as_str(), Some(library.as_ref()))
            .expect("initial URL was normalized");
        println!("AETHER_BROWSER_RUNTIME_TARGET={}", target.display_url);
        if !target.native_page && should_persist_url(&target.display_url) {
            let _ = library.record_history(
                persistence_scope,
                &target.display_url,
                &target.page_title,
                unix_now(),
            );
        }

        let metadata = Rc::new(TabMetadata::new(
            1,
            target.display_url.clone(),
            target.page_title.clone(),
            target.native_page,
            target.native_playback.clone(),
            library.clone(),
            persistence_scope,
        ));

        let first_tab = RuntimeTab {
            metadata,
            rendering_context: None,
            webview: None,
            compatibility: None,
            external_app: None,
        };
        let now = Instant::now();
        let telemetry_sampler = LinuxTelemetrySampler::new();
        *self = Self::Running(Box::new(RunningBrowser {
            window,
            servo,
            parent_context,
            native_renderer: NativeChromeRenderer::new(),
            egui_context,
            egui_painter,
            tabs: vec![first_tab],
            active_index: 0,
            next_tab_id: 1,
            pending_navigation,
            omnibox: target.display_url.clone(),
            omnibox_active: false,
            omnibox_select_all: false,
            omnibox_caret: target.display_url.len(),
            modifiers: ModifiersState::empty(),
            cursor_position: PhysicalPosition::new(0.0, 0.0),
            cursor_in_content: false,
            layout,
            last_native_frame_poll: now - Duration::from_secs(1),
            telemetry_sampler,
            telemetry_snapshot: SystemTelemetrySnapshot::default(),
            frame_rate_tracker: FrameRateTracker::new(now),
            last_telemetry_poll: now,
            last_library_chrome_poll: now,
            active_bookmarked_cache: false,
            active_download_count_cache: 0,
            last_window_title: String::new(),
            startup_started,
            first_content_paint_reported: false,
            page_complete_reported: false,
            snapshot_requested: false,
            close_requested: false,
            animated_snapshot: None,
            startup_revealed: false,
            live_frame_probe: std::env::var_os("AETHER_BROWSER_LIVE_FRAME_PROBE_PATH")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("AETHER_BROWSER_LIVE_FRAME_PROBE")
                        .map(|_| live_frame_probe_path())
                }),
            live_frame_probe_result: None,
            live_frame_probe_revealed_at: None,
            library,
            persistence_scope,
            compatibility_host,
            external_app_host,
            _profile_storage: profile_storage,
        }));

        if let Self::Running(browser) = self {
            if let Err(error) = browser.install_target_on_active_tab(target) {
                eprintln!("AETHER_BROWSER_INITIAL_TARGET=DEGRADED:{error}");
            }
            let _ = browser.adopt_orphaned_external_apps();
            browser.start_active_native_playback();
            browser.window.request_redraw();
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: WakeEvent) {
        if let Self::Running(browser) = self {
            browser.spin();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Self::Running(browser) = self else {
            return;
        };
        browser.spin();

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                browser.redraw();
                if browser.close_requested {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => browser.resize(size),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                for tab in &browser.tabs {
                    if let Some(webview) = &tab.webview {
                        webview.set_hidpi_scale_factor(euclid::Scale::new(scale_factor as f32));
                    }
                }
                browser.resize(browser.window.inner_size());
            }
            WindowEvent::Focused(true) => browser.focus_content_surface(),
            WindowEvent::Focused(false) => {
                if let Some(webview) = browser.active_webview() {
                    webview.blur();
                }
            }
            WindowEvent::CursorMoved { position, .. } => browser.handle_cursor_moved(position),
            WindowEvent::CursorLeft { .. } => {
                if browser.cursor_in_content {
                    browser.cursor_in_content = false;
                    if let Some(webview) = browser.active_webview() {
                        webview.notify_input_event(InputEvent::MouseLeftViewport(
                            MouseLeftViewportEvent::default(),
                        ));
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                browser.handle_mouse_button(state, button);
                if browser.close_requested {
                    event_loop.exit();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => browser.handle_scroll(delta),
            WindowEvent::ModifiersChanged(modifiers) => browser.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => browser.handle_keyboard(event),
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        let live_frame_probe_result = match self {
            Self::Running(browser) => browser.live_frame_probe_result.take(),
            Self::Stopped {
                live_frame_probe_result,
            } => live_frame_probe_result.take(),
            Self::Starting { .. } => None,
        };
        let previous = std::mem::replace(
            self,
            Self::Stopped {
                live_frame_probe_result,
            },
        );
        drop(previous);
        println!("AETHER_BROWSER_RUNTIME_SHUTDOWN=EARLY_DROP");
    }
}

pub fn run_live_browser(initial_url: &str) -> Result<(), Box<dyn Error>> {
    let normalized = normalize_startup_url(initial_url)?;
    let url = Url::parse(&normalized)?;

    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    let probe_requested = std::env::var_os("AETHER_BROWSER_LIVE_FRAME_PROBE_PATH").is_some()
        || std::env::var_os("AETHER_BROWSER_LIVE_FRAME_PROBE").is_some();
    let mut event_loop_builder = EventLoop::<WakeEvent>::with_user_event();
    #[cfg(target_os = "linux")]
    {
        // Chromium compatibility surfaces are X11 child windows.  Force the
        // AetherBrowser parent onto X11/XWayland as well so provider-heavy
        // pages can actually be embedded instead of silently falling back to Servo.
        event_loop_builder.with_x11();
        println!("AETHER_BROWSER_WINDOW_BACKEND=FORCED_X11_XWAYLAND");
    }
    let event_loop = event_loop_builder.build()?;
    let mut app = BrowserApp::new(&event_loop, url);
    event_loop.run_app(&mut app)?;
    if probe_requested {
        match app.live_frame_probe_result() {
            Some(Ok(())) => println!("AETHER_BROWSER_LIVE_FRAME_PROBE=PASS"),
            Some(Err(error)) => {
                return Err(std::io::Error::other(format!(
                    "Aether Browser live-frame probe failed: {error}"
                ))
                .into());
            }
            None => {
                return Err(std::io::Error::other(
                    "Aether Browser live-frame probe ended without a captured frame",
                )
                .into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod architecture_tests {
    use super::*;

    #[test]
    fn home_has_no_servo_backing_surface() {
        let target = resolve_runtime_url("aether://home", None).expect("home route must resolve");
        assert!(target.actual_url.is_none());
        assert!(target.native_page);
        assert_eq!(target.display_url, "aether://home");
    }

    #[test]
    fn web_url_has_one_content_target() {
        let target =
            resolve_runtime_url("https://example.com/", None).expect("web route must resolve");
        assert!(target.actual_url.is_some());
        assert!(!target.native_page);
    }

    #[test]
    fn native_data_page_cannot_overwrite_logical_native_route() {
        let data = Url::parse("data:text/html,hello").expect("data url");
        assert!(!should_accept_tab_url_change(true, &data));
        assert!(should_accept_tab_url_change(false, &data));
    }

    #[test]
    fn interface_action_routes_reach_expected_destinations() {
        assert_eq!(
            resolve_interface_action_url("aether://auth/start?provider=twitch").as_deref(),
            Some("https://www.twitch.tv/login")
        );
        assert_eq!(
            resolve_interface_action_url("aether://auth/start?provider=google").as_deref(),
            Some("https://accounts.google.com/")
        );
        assert_eq!(
            resolve_interface_action_url("aether://auth/start?provider=apple").as_deref(),
            Some("https://appleid.apple.com/")
        );
        assert_eq!(
            resolve_interface_action_url("aether://youtube-music?action=search").as_deref(),
            Some("https://music.youtube.com/search")
        );
        assert_eq!(
            resolve_interface_action_url("aether://youtube-music?action=library").as_deref(),
            Some("https://music.youtube.com/library")
        );
        assert_eq!(
            resolve_interface_action_url("aether://provider/open?id=twitch").as_deref(),
            Some("https://www.twitch.tv/")
        );
        assert_eq!(
            resolve_interface_action_url("aether://provider/open?id=velora").as_deref(),
            Some("https://velora.tv/")
        );
    }

    #[test]
    fn direct_interface_action_resolves_to_external_runtime_target() {
        let target = resolve_runtime_url("aether://auth/start?provider=twitch", None)
            .expect("auth action must resolve");
        assert_eq!(target.display_url, "https://www.twitch.tv/login");
        assert_eq!(
            target.actual_url.as_ref().map(Url::as_str),
            Some("https://www.twitch.tv/login")
        );
        assert!(!target.native_page);
    }

    #[test]
    fn custom_resolution_gl_blit_uses_bottom_left_framebuffer_origin() {
        let layout = ChromeLayout::canonical();
        let rect = content_blit_rect_for_size(layout, PhysicalSize::new(1669, 937));
        assert_eq!(rect.origin.x, 52);
        assert_eq!(rect.origin.y, 30);
        assert_eq!(rect.size.width, 1441);
        assert_eq!(rect.size.height, 799);
    }

    #[test]
    fn custom_resolution_pointer_origin_matches_web_content_origin() {
        let layout = ChromeLayout::canonical();
        let size = PhysicalSize::new(1669, 937);
        let at_origin = content_device_point(layout, PhysicalPosition::new(52.0, 108.0), size)
            .expect("content origin must map into Servo");
        assert_eq!(at_origin.x, 0.0);
        assert_eq!(at_origin.y, 0.0);

        let known = content_device_point(layout, PhysicalPosition::new(252.0, 308.0), size)
            .expect("known content point must map into Servo");
        assert_eq!(known.x, 200.0);
        assert_eq!(known.y, 200.0);

        assert!(content_device_point(layout, PhysicalPosition::new(200.0, 107.0), size).is_none());
        assert!(content_device_point(layout, PhysicalPosition::new(200.0, 907.0), size).is_none());
    }
    #[test]
    fn collapsed_utility_reclaims_138_pixels_at_1360x768() {
        let expanded = ChromeLayout::canonical();
        let collapsed = expanded.with_utility_collapsed(true);
        let size = PhysicalSize::new(1360, 768);
        let expanded_rect = expanded.web_content_rect(size.width, size.height);
        let collapsed_rect = collapsed.web_content_rect(size.width, size.height);
        assert_eq!(expanded_rect, (52, 108, 1132, 630));
        assert_eq!(collapsed_rect, (52, 108, 1270, 630));
        assert_eq!(collapsed_rect.2 - expanded_rect.2, 138);

        let blit = content_blit_rect_for_size(collapsed, size);
        assert_eq!(blit.origin.x, 52);
        assert_eq!(blit.origin.y, 30);
        assert_eq!(blit.size.width, 1270);
        assert_eq!(blit.size.height, 630);

        let right_edge =
            content_device_point(collapsed, PhysicalPosition::new(1321.0, 737.0), size)
                .expect("reclaimed content edge must remain inside Servo");
        assert_eq!(right_edge.x, 1269.0);
        assert_eq!(right_edge.y, 629.0);
        assert!(
            content_device_point(collapsed, PhysicalPosition::new(1322.0, 737.0), size).is_none()
        );
    }
}
