#![forbid(unsafe_code)]
//! Native Aether Browser chrome and Home surface.
//!
//! v2.1.60 deliberately contains no HTML browser shell. The top-level browser UI is
//! painted directly by Rust/egui into the parent rendering context. Servo is reserved
//! for web-page content.

use aether_telemetry::SystemTelemetrySnapshot;
use egui::{
    Align2, Color32, ColorImage, Context, FontFamily, FontId, Painter, Pos2, Rect, Stroke,
    TextureHandle, TextureOptions, pos2, vec2,
};

pub const WINDOW_TITLEBAR_HEIGHT_PX: u32 = 32;
pub const TAB_STRIP_TOP_PX: u32 = WINDOW_TITLEBAR_HEIGHT_PX;
pub const TAB_STRIP_HEIGHT_PX: u32 = 30;
pub const NAV_ROW_TOP_PX: u32 = TAB_STRIP_TOP_PX + TAB_STRIP_HEIGHT_PX;
pub const NAV_ROW_BOTTOM_PX: u32 = 107;
pub const CHROME_HEIGHT_PX: u32 = 108;
pub const TAB_MIN_WIDTH_PX: u32 = 104;
pub const TAB_MAX_WIDTH_PX: u32 = 188;
pub const LAUNCHER_WIDTH_PX: u32 = 52;
pub const UTILITY_DOCK_WIDTH_PX: u32 = 176;
pub const UTILITY_DOCK_COLLAPSED_WIDTH_PX: u32 = 38;
pub const STATUS_BAR_HEIGHT_PX: u32 = 30;
pub const RESIZE_BORDER_PX: u32 = 7;
pub const MIN_WINDOW_WIDTH_PX: u32 = 760;
pub const MIN_WINDOW_HEIGHT_PX: u32 = 520;
pub const WINDOW_CONTROL_MINIMIZE_X: u32 = 70;
pub const WINDOW_CONTROL_MAXIMIZE_X: u32 = 104;
pub const WINDOW_CONTROL_DIMINISH_X: u32 = 138;
pub const WINDOW_CONTROL_CLOSE_X: u32 = 172;
pub const WINDOW_CONTROL_HIT_RADIUS_PX: u32 = 15;
pub const WINDOW_TITLE_TEXT_X: u32 = 198;
pub const APPROVED_RENDER_REFERENCE_WIDTH_PX: u32 = 1672;
pub const APPROVED_RENDER_REFERENCE_HEIGHT_PX: u32 = 941;
pub const NATIVE_BROWSER_SURFACE_REVISION: &str = "AETHER_BROWSER_NATIVE_CHROME_V2";
pub const OPERA_GX_REFERENCE_VERSION: &str = "135.0.5973.135";
pub const OPERA_GX_REFERENCE_PACKAGE: &str = "opera-gx-stable_135.0.5973.135_amd64.deb";
pub const OPERA_GX_REFERENCE_SHA256: &str =
    "960bbce3c7a993d481568f0b3a32a70417b159f902b6083dcfa43b416ccab668";
pub const GX_REFERENCE_SIDEBAR_CONTROL: &str = "SIDEBAR_GX_CONTROL";
pub const GX_REFERENCE_WORKSPACES: &str = "SIDEBAR_WORKSPACE";
pub const GX_REFERENCE_LEFT_TAB_STRIP: &str = "LEFT_TAB_STRIP";
pub const GX_REFERENCE_PLAYER_SERVICE: &str = "SIDEBAR_PLAYER_SERVICE";
pub const AETHERFORGE_GX_DRAGONGLASS: &str = "AETHERFORGE_GX_DRAGONGLASS";
pub const APPROVED_RENDER_VISUAL_LOCK: &str = "AETHER_BROWSER_APPROVED_RENDER_2026_09_09";
pub const HOME_DISPLAY_URL: &str = "https://aetherforge.app";
pub const OMNIBOX_CARET_VISUAL_CONTRACT: &str = "AETHER_BROWSER_OMNIBOX_CARET_VISIBLE_WHEN_ACTIVE";

const HERO_BYTES: &[u8] = include_bytes!("../assets/aetherforge-cosmic-wallpaper.jpg");
const STREAM_BYTES: &[u8] = include_bytes!("../assets/aether-stream-studio-preview.png");
const VAULT_BYTES: &[u8] = include_bytes!("../assets/aether-vault-preview.png");

const BG: Color32 = Color32::from_rgb(6, 7, 16);
const CHROME: Color32 = Color32::from_rgba_premultiplied(11, 12, 28, 244);
const GLASS: Color32 = Color32::from_rgba_premultiplied(18, 18, 42, 222);
const GLASS_SOFT: Color32 = Color32::from_rgba_premultiplied(28, 24, 56, 195);
const GLASS_LIGHT: Color32 = Color32::from_rgba_premultiplied(50, 42, 92, 175);
const VIOLET: Color32 = Color32::from_rgb(153, 92, 255);
const INDIGO: Color32 = Color32::from_rgb(102, 87, 255);
const CYAN: Color32 = Color32::from_rgb(88, 224, 255);
const TEXT: Color32 = Color32::from_rgb(243, 239, 255);
const TEXT_SOFT: Color32 = Color32::from_rgb(188, 180, 219);
const TEXT_MUTED: Color32 = Color32::from_rgb(128, 122, 158);
const GREEN: Color32 = Color32::from_rgb(95, 235, 172);
const RED: Color32 = Color32::from_rgb(243, 90, 129);
const GOLD: Color32 = Color32::from_rgb(255, 202, 96);
const GX_EDGE_GLOW: Color32 = Color32::from_rgba_premultiplied(126, 82, 255, 92);
const ACTIVE_RAIL: Color32 = Color32::from_rgb(116, 94, 255);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChromeLayout {
    utility_collapsed: bool,
}

impl ChromeLayout {
    #[must_use]
    pub const fn canonical() -> Self {
        Self {
            utility_collapsed: false,
        }
    }

    #[must_use]
    pub const fn launcher_width(self) -> u32 {
        LAUNCHER_WIDTH_PX
    }

    #[must_use]
    pub const fn utility_width(self) -> u32 {
        if self.utility_collapsed {
            UTILITY_DOCK_COLLAPSED_WIDTH_PX
        } else {
            UTILITY_DOCK_WIDTH_PX
        }
    }

    #[must_use]
    pub const fn utility_collapsed(self) -> bool {
        self.utility_collapsed
    }

    #[must_use]
    pub const fn with_utility_collapsed(mut self, collapsed: bool) -> Self {
        self.utility_collapsed = collapsed;
        self
    }

    pub fn toggle_utility_collapsed(&mut self) {
        self.utility_collapsed = !self.utility_collapsed;
    }

    #[must_use]
    pub const fn status_height(self) -> u32 {
        STATUS_BAR_HEIGHT_PX
    }

    #[must_use]
    pub fn web_content_rect(self, width: u32, height: u32) -> (u32, u32, u32, u32) {
        let x = self.launcher_width();
        let y = CHROME_HEIGHT_PX;
        let w = width
            .saturating_sub(self.launcher_width() + self.utility_width())
            .max(1);
        let h = height
            .saturating_sub(CHROME_HEIGHT_PX + self.status_height())
            .max(1);
        (x, y, w, h)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NamedAccent {
    pub name: &'static str,
    pub rgb: (u8, u8, u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DragonGlassTokens {
    pub surface_transparency_percent: u8,
    pub smoky_surface_percent: u8,
    pub rounded_geometry: bool,
    pub compact_chrome: bool,
    pub accent_primary: NamedAccent,
    pub accent_secondary: NamedAccent,
}

impl DragonGlassTokens {
    #[must_use]
    pub const fn canonical() -> Self {
        Self {
            surface_transparency_percent: 90,
            smoky_surface_percent: 10,
            rounded_geometry: true,
            compact_chrome: true,
            accent_primary: NamedAccent {
                name: "violet",
                rgb: (153, 92, 255),
            },
            accent_secondary: NamedAccent {
                name: "indigo",
                rgb: (102, 87, 255),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChromeTab {
    pub id: u64,
    pub title: String,
    pub url: String,
    pub active: bool,
}

impl ChromeTab {
    #[must_use]
    pub fn label(&self) -> &str {
        let title = self.title.trim();
        if title.is_empty() { "New Tab" } else { title }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromeHitTarget {
    None,
    WindowDrag,
    WindowMinimize,
    WindowMaximize,
    WindowDiminish,
    WindowClose,
    Back,
    Forward,
    Reload,
    Omnibox,
    NewTab,
    Tab(u64),
    CloseTab(u64),
    Home,
    StreamStudio,
    Vault,
    Accounts,
    Providers,
    MediaCenter,
    YouTubeMusic,
    YouTubeMusicSearch,
    YouTubeMusicLibrary,
    Snapshot,
    AnimatedSnapshot,
    CreatorHub,
    CommsHub,
    Library,
    Downloads,
    Settings,
    BookmarkToggle,
    UtilityCollapseToggle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowResizeEdge {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

#[must_use]
pub fn window_resize_edge(x: f64, y: f64, width: u32, height: u32) -> Option<WindowResizeEdge> {
    if x < 0.0 || y < 0.0 || x >= f64::from(width) || y >= f64::from(height) {
        return None;
    }
    let border = f64::from(RESIZE_BORDER_PX);
    let west = x < border;
    let east = x >= f64::from(width.saturating_sub(RESIZE_BORDER_PX));
    let north = y < border;
    let south = y >= f64::from(height.saturating_sub(RESIZE_BORDER_PX));
    match (north, south, west, east) {
        (true, _, true, _) => Some(WindowResizeEdge::NorthWest),
        (true, _, _, true) => Some(WindowResizeEdge::NorthEast),
        (_, true, true, _) => Some(WindowResizeEdge::SouthWest),
        (_, true, _, true) => Some(WindowResizeEdge::SouthEast),
        (true, _, _, _) => Some(WindowResizeEdge::North),
        (_, true, _, _) => Some(WindowResizeEdge::South),
        (_, _, true, _) => Some(WindowResizeEdge::West),
        (_, _, _, true) => Some(WindowResizeEdge::East),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChromeModel {
    pub tabs: Vec<ChromeTab>,
    pub omnibox: String,
    pub omnibox_active: bool,
    pub omnibox_caret: usize,
    pub home_mode: bool,
    pub layout: ChromeLayout,
    pub telemetry: SystemTelemetrySnapshot,
    pub active_bookmarked: bool,
    pub active_download_count: usize,
}

impl ChromeModel {
    #[must_use]
    pub fn new(tabs: Vec<ChromeTab>, omnibox: impl Into<String>) -> Self {
        let omnibox = omnibox.into();
        let omnibox_caret = omnibox.len();
        Self {
            tabs,
            omnibox,
            omnibox_active: false,
            omnibox_caret,
            home_mode: false,
            layout: ChromeLayout::canonical(),
            telemetry: SystemTelemetrySnapshot::default(),
            active_bookmarked: false,
            active_download_count: 0,
        }
    }

    #[must_use]
    pub fn display_omnibox(&self) -> &str {
        if self.omnibox_active {
            &self.omnibox
        } else if self.home_mode {
            HOME_DISPLAY_URL
        } else {
            &self.omnibox
        }
    }

    #[must_use]
    pub const fn native_surface_owns_content(&self) -> bool {
        self.home_mode
    }

    #[must_use]
    pub fn tab_width_for_count(&self, width: u32) -> u32 {
        let count = self.tabs.len().max(1) as u32;
        let reserved = self.layout.launcher_width().saturating_add(72);
        let available = width.saturating_sub(reserved).max(TAB_MIN_WIDTH_PX);
        (available / count).clamp(TAB_MIN_WIDTH_PX, TAB_MAX_WIDTH_PX)
    }

    #[must_use]
    pub fn hit_test(&self, x: f64, y: f64, width: u32, height: u32) -> ChromeHitTarget {
        let launcher = self.layout.launcher_width();
        let utility = self.layout.utility_width();
        let status = self.layout.status_height();

        if y < f64::from(WINDOW_TITLEBAR_HEIGHT_PX) {
            let hit_radius = f64::from(WINDOW_CONTROL_HIT_RADIUS_PX);
            for (center, target) in [
                (WINDOW_CONTROL_MINIMIZE_X, ChromeHitTarget::WindowMinimize),
                (WINDOW_CONTROL_MAXIMIZE_X, ChromeHitTarget::WindowMaximize),
                (WINDOW_CONTROL_DIMINISH_X, ChromeHitTarget::WindowDiminish),
                (WINDOW_CONTROL_CLOSE_X, ChromeHitTarget::WindowClose),
            ] {
                let center = f64::from(center);
                if (center - hit_radius..=center + hit_radius).contains(&x) {
                    return target;
                }
            }
            return ChromeHitTarget::WindowDrag;
        }

        if y >= f64::from(TAB_STRIP_TOP_PX) && y < f64::from(TAB_STRIP_TOP_PX + TAB_STRIP_HEIGHT_PX)
        {
            let tab_start = launcher.saturating_add(8);
            let tab_width = self.tab_width_for_count(width);
            let tab_region = tab_width.saturating_mul(self.tabs.len() as u32);
            if x >= f64::from(tab_start) && x < f64::from(tab_start + tab_region) {
                let local = x - f64::from(tab_start);
                let index = (local / f64::from(tab_width)) as usize;
                if let Some(tab) = self.tabs.get(index) {
                    let within = local % f64::from(tab_width);
                    if within >= f64::from(tab_width.saturating_sub(25)) {
                        return ChromeHitTarget::CloseTab(tab.id);
                    }
                    return ChromeHitTarget::Tab(tab.id);
                }
            }
            if x >= f64::from(tab_start + tab_region + 2)
                && x <= f64::from(tab_start + tab_region + 38)
            {
                return ChromeHitTarget::NewTab;
            }
            return ChromeHitTarget::None;
        }

        if (f64::from(NAV_ROW_TOP_PX)..=f64::from(NAV_ROW_BOTTOM_PX)).contains(&y) {
            let nav_start = f64::from(launcher) + 14.0;
            let right = f64::from(width);
            if (nav_start..=nav_start + 31.0).contains(&x) {
                return ChromeHitTarget::Back;
            }
            if (nav_start + 39.0..=nav_start + 70.0).contains(&x) {
                return ChromeHitTarget::Forward;
            }
            if (nav_start + 78.0..=nav_start + 109.0).contains(&x) {
                return ChromeHitTarget::Reload;
            }
            let omni_right = (right - 154.0).max(nav_start + 238.0);
            if x >= omni_right - 35.0 && x <= omni_right {
                return ChromeHitTarget::BookmarkToggle;
            }
            if x >= nav_start + 118.0 && x <= omni_right - 36.0 {
                return ChromeHitTarget::Omnibox;
            }
            for (center, target) in [
                (right - 122.0, ChromeHitTarget::Downloads),
                (right - 86.0, ChromeHitTarget::Settings),
            ] {
                if (center - 15.0..=center + 15.0).contains(&x) {
                    return target;
                }
            }
            return ChromeHitTarget::None;
        }

        if x < f64::from(launcher)
            && y >= f64::from(CHROME_HEIGHT_PX)
            && y < f64::from(height.saturating_sub(status))
        {
            for (index, target) in [
                ChromeHitTarget::Home,
                ChromeHitTarget::Providers,
                ChromeHitTarget::StreamStudio,
                ChromeHitTarget::Vault,
                ChromeHitTarget::Accounts,
                ChromeHitTarget::CreatorHub,
                ChromeHitTarget::MediaCenter,
                ChromeHitTarget::YouTubeMusic,
            ]
            .into_iter()
            .enumerate()
            {
                let center = f64::from(CHROME_HEIGHT_PX) + 42.0 + index as f64 * 46.0;
                if (center - 20.0..=center + 20.0).contains(&y) {
                    return target;
                }
            }
            return ChromeHitTarget::None;
        }

        if self.home_mode {
            let content_left = f64::from(launcher);
            let content_right = f64::from(width.saturating_sub(utility));
            let content_top = f64::from(CHROME_HEIGHT_PX);
            let content_bottom = f64::from(height.saturating_sub(status));
            let content_width = (content_right - content_left).max(0.0);
            let content_height = (content_bottom - content_top).max(0.0);
            let hero_h = (content_height * 0.44).clamp(260.0, 350.0);
            let cards_top = content_top + hero_h - 2.0;
            let cards_bottom = content_bottom - 10.0;
            let available_h = (cards_bottom - cards_top).max(180.0);
            let gap = 10.0;
            let quick_w = 176.0;
            let main_w = ((content_width - quick_w - gap * 4.0) / 2.0).max(250.0);
            let card_top = cards_top + gap;
            let card_bottom = card_top + available_h - gap;
            let stream_left = content_left + gap;
            let stream_right = stream_left + main_w;
            let vault_left = stream_right + gap;
            let vault_right = vault_left + main_w;
            let quick_left = vault_right + gap;
            let quick_right = content_right - gap;

            if x >= stream_left && x <= stream_right && y >= card_top && y <= card_bottom {
                return ChromeHitTarget::StreamStudio;
            }
            if x >= vault_left && x <= vault_right && y >= card_top && y <= card_bottom {
                return ChromeHitTarget::Vault;
            }
            if x >= quick_left && x <= quick_right && y >= card_top && y <= cards_bottom {
                let first_y = card_top + 42.0;
                for (index, target) in [
                    ChromeHitTarget::MediaCenter,
                    ChromeHitTarget::StreamStudio,
                    ChromeHitTarget::CreatorHub,
                ]
                .into_iter()
                .enumerate()
                {
                    let row_y = first_y + index as f64 * 40.0;
                    if (row_y - 18.0..=row_y + 18.0).contains(&y) {
                        return target;
                    }
                }
            }
        }

        let utility_left = width.saturating_sub(utility);
        if x >= f64::from(utility_left)
            && y >= f64::from(CHROME_HEIGHT_PX)
            && y < f64::from(height.saturating_sub(status))
        {
            let top = f64::from(CHROME_HEIGHT_PX);
            if (top + 4.0..=top + 34.0).contains(&y) {
                return ChromeHitTarget::UtilityCollapseToggle;
            }
            if self.layout.utility_collapsed() {
                if (top + 48.0..=top + 88.0).contains(&y) {
                    return ChromeHitTarget::YouTubeMusic;
                }
                if (top + 98.0..=top + 138.0).contains(&y) {
                    return ChromeHitTarget::Library;
                }
                if (top + 148.0..=top + 188.0).contains(&y) {
                    return ChromeHitTarget::StreamStudio;
                }
                return ChromeHitTarget::None;
            }
            if (top + 42.0..=top + 134.0).contains(&y) {
                if y >= top + 100.0 {
                    let split = f64::from(utility_left) + f64::from(utility) / 2.0;
                    return if x < split {
                        ChromeHitTarget::YouTubeMusicSearch
                    } else {
                        ChromeHitTarget::YouTubeMusicLibrary
                    };
                }
                return ChromeHitTarget::YouTubeMusic;
            }
            if (top + 144.0..=top + 242.0).contains(&y) {
                return ChromeHitTarget::Library;
            }
            if (top + 252.0..=top + 402.0).contains(&y) {
                return ChromeHitTarget::StreamStudio;
            }
        }

        ChromeHitTarget::None
    }
}

fn provider_tab_glyph(tab: &ChromeTab) -> &'static str {
    let url = tab.url.to_ascii_lowercase();
    if url.contains("youtube.com") || url.contains("youtu.be") {
        "Y"
    } else if url.contains("twitch.tv") {
        "T"
    } else if url.starts_with("aether://") {
        "A"
    } else {
        "•"
    }
}

pub struct NativeChromeRenderer {
    hero: Option<TextureHandle>,
    stream_preview: Option<TextureHandle>,
    vault_preview: Option<TextureHandle>,
    home_shell_frame_seen: bool,
}

impl Default for NativeChromeRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeChromeRenderer {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            hero: None,
            stream_preview: None,
            vault_preview: None,
            home_shell_frame_seen: false,
        }
    }

    fn ensure_home_assets(&mut self, ctx: &Context) {
        if self.hero.is_none() {
            self.hero = decode_texture(ctx, "aether-home-wallpaper", HERO_BYTES);
        }
        if !self.home_shell_frame_seen {
            self.home_shell_frame_seen = true;
            ctx.request_repaint();
            return;
        }
        if self.stream_preview.is_none() {
            self.stream_preview = decode_texture(ctx, "aether-stream-preview", STREAM_BYTES);
        }
        if self.vault_preview.is_none() {
            self.vault_preview = decode_texture(ctx, "aether-vault-preview", VAULT_BYTES);
        }
    }

    /// Paint one complete native browser shell frame.
    ///
    /// This method only emits egui paint commands. The caller owns the OpenGL context,
    /// texture upload, and final buffer present.
    pub fn paint(&mut self, ctx: &Context, model: &ChromeModel, width: u32, height: u32) {
        if model.home_mode {
            self.ensure_home_assets(ctx);
        }
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(width as f32, height as f32));
        let painter = ctx.layer_painter(egui::LayerId::background());
        if model.native_surface_owns_content() {
            painter.rect_filled(screen, 0.0, BG);
        }

        self.paint_top_chrome(&painter, model, width);
        self.paint_left_launcher(&painter, height);
        self.paint_right_utility(&painter, model, width, height);
        self.paint_status(&painter, model, width, height);

        if model.home_mode {
            self.paint_home(&painter, model, width, height);
        }
    }

    fn paint_top_chrome(&self, painter: &Painter, model: &ChromeModel, width: u32) {
        let width = width as f32;
        let launcher = model.layout.launcher_width() as f32;
        painter.rect_filled(
            Rect::from_min_max(pos2(0.0, 0.0), pos2(width, CHROME_HEIGHT_PX as f32)),
            0.0,
            CHROME,
        );
        // Opera-GX-inspired structure, translated into AetherForge DragonGlass:
        // a restrained accent edge instead of importing any vendor artwork.
        painter.rect_filled(
            Rect::from_min_max(
                pos2(0.0, CHROME_HEIGHT_PX as f32 - 1.0),
                pos2(width, CHROME_HEIGHT_PX as f32),
            ),
            0.0,
            GX_EDGE_GLOW,
        );
        painter.rect_filled(
            Rect::from_min_max(
                pos2(0.0, 0.0),
                pos2(launcher, WINDOW_TITLEBAR_HEIGHT_PX as f32),
            ),
            0.0,
            Color32::from_rgb(12, 12, 30),
        );
        let title_y = WINDOW_TITLEBAR_HEIGHT_PX as f32 / 2.0;
        let brand = Rect::from_center_size(pos2(launcher / 2.0, title_y), vec2(30.0, 20.0));
        painter.rect_filled(
            brand,
            7.0,
            Color32::from_rgba_premultiplied(101, 69, 224, 205),
        );
        painter.rect_filled(
            Rect::from_min_max(pos2(brand.left(), brand.bottom() - 2.0), brand.max),
            1.0,
            CYAN,
        );
        text(
            painter,
            brand.center(),
            Align2::CENTER_CENTER,
            "AF",
            9.5,
            TEXT,
            true,
        );
        for (center_x, symbol, fill) in [
            (WINDOW_CONTROL_MINIMIZE_X as f32, "–", INDIGO),
            (WINDOW_CONTROL_MAXIMIZE_X as f32, "□", VIOLET),
            (WINDOW_CONTROL_DIMINISH_X as f32, "❐", GLASS_LIGHT),
            (WINDOW_CONTROL_CLOSE_X as f32, "×", RED),
        ] {
            let center = pos2(center_x, title_y);
            painter.circle_filled(center, 9.0, fill);
            text(
                painter,
                center,
                Align2::CENTER_CENTER,
                symbol,
                11.0,
                TEXT,
                true,
            );
        }
        text(
            painter,
            pos2(WINDOW_TITLE_TEXT_X as f32, title_y),
            Align2::LEFT_CENTER,
            "AetherForge Browser v2.1.60",
            12.0,
            TEXT_SOFT,
            true,
        );

        let tab_bottom = (TAB_STRIP_TOP_PX + TAB_STRIP_HEIGHT_PX) as f32;
        painter.rect_filled(
            Rect::from_min_max(pos2(0.0, TAB_STRIP_TOP_PX as f32), pos2(width, tab_bottom)),
            0.0,
            Color32::from_rgb(9, 9, 23),
        );
        painter.rect_filled(
            Rect::from_min_max(
                pos2(0.0, TAB_STRIP_TOP_PX as f32),
                pos2(launcher, tab_bottom),
            ),
            0.0,
            Color32::from_rgb(15, 13, 34),
        );
        let tab_y = TAB_STRIP_TOP_PX as f32 + TAB_STRIP_HEIGHT_PX as f32 / 2.0;
        painter.circle_filled(pos2(launcher / 2.0, tab_y), 4.0, CYAN);

        let tab_start = launcher + 8.0;
        let tab_width = model.tab_width_for_count(width as u32) as f32;
        for (index, tab) in model.tabs.iter().enumerate() {
            let x = tab_start + index as f32 * tab_width;
            let rect = Rect::from_min_size(
                pos2(x, TAB_STRIP_TOP_PX as f32 + 2.0),
                vec2(tab_width - 4.0, TAB_STRIP_HEIGHT_PX as f32 - 4.0),
            );
            let fill = if tab.active { GLASS_LIGHT } else { GLASS_SOFT };
            painter.rect_filled(rect, 9.0, fill);
            if tab.active {
                painter.rect_filled(
                    Rect::from_min_max(
                        pos2(rect.left() + 8.0, rect.bottom() - 2.0),
                        pos2(rect.right() - 8.0, rect.bottom()),
                    ),
                    1.0,
                    ACTIVE_RAIL,
                );
            }
            let glyph = provider_tab_glyph(tab);
            text(
                painter,
                pos2(x + 13.0, tab_y),
                Align2::CENTER_CENTER,
                glyph,
                9.0,
                if tab.active { CYAN } else { TEXT_MUTED },
                true,
            );
            let max_chars = ((tab_width - 52.0) / 6.2).floor().max(8.0) as usize;
            text(
                painter,
                pos2(x + 24.0, tab_y),
                Align2::LEFT_CENTER,
                truncate(tab.label(), max_chars),
                10.5,
                if tab.active { TEXT } else { TEXT_SOFT },
                tab.active,
            );
            text(
                painter,
                pos2(rect.right() - 13.0, tab_y),
                Align2::CENTER_CENTER,
                "×",
                11.0,
                TEXT_MUTED,
                false,
            );
        }
        let plus_x = tab_start + model.tabs.len() as f32 * tab_width + 16.0;
        text(
            painter,
            pos2(plus_x, tab_y),
            Align2::CENTER_CENTER,
            "+",
            18.0,
            TEXT_SOFT,
            false,
        );

        painter.rect_filled(
            Rect::from_min_max(
                pos2(0.0, NAV_ROW_TOP_PX as f32),
                pos2(width, CHROME_HEIGHT_PX as f32),
            ),
            0.0,
            Color32::from_rgb(10, 10, 25),
        );
        painter.rect_filled(
            Rect::from_min_max(
                pos2(0.0, NAV_ROW_TOP_PX as f32),
                pos2(launcher, CHROME_HEIGHT_PX as f32),
            ),
            0.0,
            Color32::from_rgb(15, 13, 34),
        );
        let nav_y = (NAV_ROW_TOP_PX as f32 + CHROME_HEIGHT_PX as f32) / 2.0;
        text(
            painter,
            pos2(launcher / 2.0, nav_y),
            Align2::CENTER_CENTER,
            "«",
            16.0,
            TEXT_SOFT,
            true,
        );

        let nav_start = launcher + 14.0;
        for (x, glyph) in [
            (nav_start + 15.0, "‹"),
            (nav_start + 54.0, "›"),
            (nav_start + 93.0, "↻"),
        ] {
            painter.circle_filled(pos2(x, nav_y), 13.0, GLASS_SOFT);
            text(
                painter,
                pos2(x, nav_y),
                Align2::CENTER_CENTER,
                glyph,
                17.0,
                TEXT_SOFT,
                true,
            );
        }

        let omni_left = nav_start + 118.0;
        let omni_right = (width - 190.0).max(omni_left + 120.0);
        let omni = Rect::from_min_max(
            pos2(omni_left, nav_y - 15.0),
            pos2(omni_right, nav_y + 15.0),
        );
        if model.omnibox_active {
            painter.rect_filled(
                omni.expand(1.0),
                16.0,
                Color32::from_rgba_premultiplied(102, 87, 255, 150),
            );
        }
        painter.rect_filled(
            omni,
            15.0,
            Color32::from_rgba_premultiplied(25, 23, 48, 235),
        );
        painter.circle_filled(pos2(omni.left() + 16.0, nav_y), 4.5, GREEN);
        let text_clip = Rect::from_min_max(
            pos2(omni.left() + 8.0, omni.top()),
            pos2(omni.right() - 36.0, omni.bottom()),
        );
        let omni_painter = painter.with_clip_rect(text_clip);
        let _ = text(
            &omni_painter,
            pos2(omni.left() + 29.0, nav_y),
            Align2::LEFT_CENTER,
            model.display_omnibox(),
            12.0,
            if model.omnibox_active {
                TEXT
            } else {
                TEXT_SOFT
            },
            false,
        );
        if model.omnibox_active {
            let display = model.display_omnibox();
            let caret = model.omnibox_caret.min(display.len());
            let caret = (0..=caret)
                .rev()
                .find(|index| display.is_char_boundary(*index))
                .unwrap_or(0);
            let caret_prefix = &model.display_omnibox()[..caret];
            let caret_width = omni_painter
                .layout_no_wrap(
                    caret_prefix.to_owned(),
                    FontId::new(12.0, FontFamily::Proportional),
                    TEXT,
                )
                .size()
                .x;
            let caret_x = (omni.left() + 29.0 + caret_width + 1.5).min(omni.right() - 40.0);
            omni_painter.line_segment(
                [pos2(caret_x, nav_y - 9.0), pos2(caret_x, nav_y + 9.0)],
                Stroke::new(1.5_f32, CYAN),
            );
        }
        let star = if model.active_bookmarked {
            "★"
        } else {
            "☆"
        };
        text(
            painter,
            pos2(omni.right() - 17.0, nav_y),
            Align2::CENTER_CENTER,
            star,
            17.0,
            GOLD,
            false,
        );

        for (x, glyph) in [(width - 122.0, "↓"), (width - 86.0, "☰")] {
            painter.circle_filled(pos2(x, nav_y), 13.0, GLASS_SOFT);
            text(
                painter,
                pos2(x, nav_y),
                Align2::CENTER_CENTER,
                glyph,
                9.5,
                TEXT_SOFT,
                true,
            );
        }
    }

    fn paint_left_launcher(&self, painter: &Painter, height: u32) {
        let bottom = height.saturating_sub(STATUS_BAR_HEIGHT_PX) as f32;
        let width = LAUNCHER_WIDTH_PX as f32;
        let center_x = width / 2.0;
        painter.rect_filled(
            Rect::from_min_max(pos2(0.0, CHROME_HEIGHT_PX as f32), pos2(width, bottom)),
            0.0,
            Color32::from_rgba_premultiplied(10, 9, 25, 205),
        );
        painter.rect_filled(
            Rect::from_min_max(
                pos2(width - 1.0, CHROME_HEIGHT_PX as f32),
                pos2(width, bottom),
            ),
            0.0,
            GX_EDGE_GLOW,
        );

        // The Linux Opera GX 135 package exposes GX-control, workspace, left-tab-strip,
        // player-service, and messenger sidebar concepts. AetherBrowser mirrors that
        // information architecture with original AetherForge controls and artwork.
        self.paint_workspace_switcher(painter, center_x);
        self.paint_sidebar_tool_stack(painter, center_x, bottom);
    }

    fn paint_workspace_switcher(&self, painter: &Painter, center_x: f32) {
        let control_y = CHROME_HEIGHT_PX as f32 + 25.0;
        let control = Rect::from_center_size(pos2(center_x, control_y), vec2(34.0, 32.0));
        painter.rect_filled(
            control,
            11.0,
            Color32::from_rgba_premultiplied(101, 69, 224, 158),
        );
        painter.rect_stroke(
            control,
            11.0,
            Stroke::new(1.0_f32, GX_EDGE_GLOW),
            egui::StrokeKind::Inside,
        );
        text(
            painter,
            control.center(),
            Align2::CENTER_CENTER,
            "AF",
            9.5,
            TEXT,
            true,
        );

        let workspace_y = control.bottom() + 20.0;
        for index in 0..3 {
            let y = workspace_y + index as f32 * 14.0;
            let active = index == 0;
            painter.circle_filled(
                pos2(center_x, y),
                if active { 4.0 } else { 2.8 },
                if active { CYAN } else { TEXT_MUTED },
            );
            if active {
                painter.rect_filled(
                    Rect::from_center_size(pos2(2.0, y), vec2(4.0, 14.0)),
                    2.0,
                    ACTIVE_RAIL,
                );
            }
        }
    }

    fn paint_sidebar_tool_stack(&self, painter: &Painter, center_x: f32, bottom: f32) {
        let primary_top = CHROME_HEIGHT_PX as f32 + 106.0;
        let primary = [
            ("⌂", "Home"),
            ("◎", "Search"),
            ("S", "Studio"),
            ("V", "Vault"),
        ];
        for (index, (glyph, _label)) in primary.iter().enumerate() {
            let y = primary_top + index as f32 * 44.0;
            let active = index == 0;
            self.paint_sidebar_button(painter, center_x, y, glyph, active);
        }

        let separator_y = primary_top + primary.len() as f32 * 44.0 + 4.0;
        painter.line_segment(
            [
                pos2(13.0, separator_y),
                pos2(LAUNCHER_WIDTH_PX as f32 - 13.0, separator_y),
            ],
            Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(126, 82, 255, 58)),
        );

        // Integrated-service rail: creator, communications, and music/player.
        let service_top = separator_y + 24.0;
        let services = [("C", "Creator"), ("M", "Messages"), ("♫", "Player")];
        for (index, (glyph, _label)) in services.iter().enumerate() {
            let y = service_top + index as f32 * 44.0;
            if y + 20.0 < bottom {
                self.paint_sidebar_button(painter, center_x, y, glyph, false);
            }
        }

        let settings_y = bottom - 26.0;
        if settings_y > service_top + services.len() as f32 * 44.0 {
            self.paint_sidebar_button(painter, center_x, settings_y, "⚙", false);
        }
    }

    fn paint_sidebar_button(
        &self,
        painter: &Painter,
        center_x: f32,
        y: f32,
        glyph: &str,
        active: bool,
    ) {
        let button = Rect::from_center_size(pos2(center_x, y), vec2(34.0, 34.0));
        if active {
            painter.rect_filled(
                button,
                11.0,
                Color32::from_rgba_premultiplied(110, 70, 225, 165),
            );
            painter.rect_filled(
                Rect::from_center_size(pos2(2.0, y), vec2(4.0, 24.0)),
                2.0,
                ACTIVE_RAIL,
            );
        } else {
            painter.rect_filled(
                button,
                11.0,
                Color32::from_rgba_premultiplied(25, 22, 52, 110),
            );
        }
        text(
            painter,
            pos2(center_x, y),
            Align2::CENTER_CENTER,
            glyph,
            if glyph.len() > 1 { 9.0 } else { 13.0 },
            if active { TEXT } else { TEXT_SOFT },
            true,
        );
    }

    fn paint_right_utility(&self, painter: &Painter, model: &ChromeModel, width: u32, height: u32) {
        let utility_width = model.layout.utility_width() as f32;
        let x0 = width as f32 - utility_width;
        let top = CHROME_HEIGHT_PX as f32;
        let bottom = height.saturating_sub(STATUS_BAR_HEIGHT_PX) as f32;
        painter.rect_filled(
            Rect::from_min_max(pos2(x0, top), pos2(width as f32, bottom)),
            0.0,
            Color32::from_rgba_premultiplied(9, 9, 23, 244),
        );

        let toggle_center = pos2(x0 + utility_width / 2.0, top + 19.0);
        painter.circle_filled(toggle_center, 12.0, GLASS_SOFT);
        text(
            painter,
            toggle_center,
            Align2::CENTER_CENTER,
            if model.layout.utility_collapsed() {
                "«"
            } else {
                "»"
            },
            14.0,
            TEXT_SOFT,
            true,
        );

        if model.layout.utility_collapsed() {
            for (index, (glyph, accent)) in [("♫", INDIGO), ("↓", CYAN), ("S", GREEN)]
                .iter()
                .enumerate()
            {
                let y = top + 68.0 + index as f32 * 50.0;
                let rect =
                    Rect::from_center_size(pos2(x0 + utility_width / 2.0, y), vec2(30.0, 34.0));
                painter.rect_filled(rect, 10.0, GLASS);
                text(
                    painter,
                    rect.center(),
                    Align2::CENTER_CENTER,
                    glyph,
                    9.5,
                    *accent,
                    true,
                );
            }
            return;
        }

        utility_card(
            painter,
            Rect::from_min_max(
                pos2(x0 + 10.0, top + 42.0),
                pos2(width as f32 - 10.0, top + 134.0),
            ),
            "MEDIA",
            "YouTube Music",
            "♫",
            INDIGO,
        );
        text(
            painter,
            pos2(x0 + 24.0, top + 114.0),
            Align2::LEFT_CENTER,
            "Search   Library",
            9.5,
            TEXT_MUTED,
            false,
        );

        let dl = Rect::from_min_max(
            pos2(x0 + 10.0, top + 144.0),
            pos2(width as f32 - 10.0, top + 242.0),
        );
        painter.rect_filled(dl, 12.0, GLASS);
        text(
            painter,
            pos2(dl.left() + 12.0, dl.top() + 16.0),
            Align2::LEFT_CENTER,
            "DOWNLOADS",
            9.5,
            VIOLET,
            true,
        );
        text(
            painter,
            pos2(dl.left() + 12.0, dl.top() + 42.0),
            Align2::LEFT_CENTER,
            download_text(model.active_download_count),
            10.0,
            TEXT_SOFT,
            false,
        );
        let track = Rect::from_min_max(
            pos2(dl.left() + 12.0, dl.top() + 64.0),
            pos2(dl.right() - 12.0, dl.top() + 70.0),
        );
        painter.rect_filled(track, 3.0, Color32::from_rgb(41, 36, 65));
        let progress = if model.active_download_count == 0 {
            0.0
        } else {
            0.52
        };
        painter.rect_filled(
            Rect::from_min_max(
                track.min,
                pos2(track.left() + track.width() * progress, track.bottom()),
            ),
            3.0,
            INDIGO,
        );

        let stream_bottom = (top + 402.0).min(bottom - 10.0);
        let stream = Rect::from_min_max(
            pos2(x0 + 10.0, top + 252.0),
            pos2(width as f32 - 10.0, stream_bottom),
        );
        painter.rect_filled(stream, 12.0, GLASS);
        text(
            painter,
            pos2(stream.left() + 12.0, stream.top() + 16.0),
            Align2::LEFT_CENTER,
            "STREAM STATUS",
            9.5,
            VIOLET,
            true,
        );
        let stream_state = if model.telemetry.stream.active {
            "LIVE"
        } else {
            "OFFLINE"
        };
        text(
            painter,
            pos2(stream.right() - 12.0, stream.top() + 16.0),
            Align2::RIGHT_CENTER,
            stream_state,
            9.5,
            if model.telemetry.stream.active {
                GREEN
            } else {
                TEXT_MUTED
            },
            true,
        );
        text(
            painter,
            pos2(stream.left() + 12.0, stream.top() + 43.0),
            Align2::LEFT_CENTER,
            format_fps(
                "FPS",
                model
                    .telemetry
                    .stream
                    .render_fps_milli
                    .or(model.telemetry.browser_fps_milli),
            ),
            10.0,
            TEXT_SOFT,
            false,
        );
        text(
            painter,
            pos2(stream.left() + 12.0, stream.top() + 66.0),
            Align2::LEFT_CENTER,
            format_network(
                model.telemetry.network_rx_bytes_per_sec,
                model.telemetry.network_tx_bytes_per_sec,
            ),
            9.5,
            TEXT_MUTED,
            false,
        );
        text(
            painter,
            pos2(stream.left() + 12.0, stream.top() + 89.0),
            Align2::LEFT_CENTER,
            format!("LOST {}", model.telemetry.stream.total_lost_frames()),
            9.5,
            TEXT_MUTED,
            false,
        );
        text(
            painter,
            pos2(stream.left() + 12.0, stream.bottom() - 16.0),
            Align2::LEFT_CENTER,
            "Open Studio",
            10.0,
            CYAN,
            true,
        );
    }

    fn paint_status(&self, painter: &Painter, model: &ChromeModel, width: u32, height: u32) {
        let y = height.saturating_sub(STATUS_BAR_HEIGHT_PX) as f32;
        let rect = Rect::from_min_max(pos2(0.0, y), pos2(width as f32, height as f32));
        painter.rect_filled(rect, 0.0, Color32::from_rgb(8, 8, 20));
        text(
            painter,
            pos2(14.0, y + 15.0),
            Align2::LEFT_CENTER,
            "AETHERFORGE",
            9.5,
            VIOLET,
            true,
        );
        let cpu = format_percent("CPU", model.telemetry.cpu_percent);
        let ram = format_memory_pair(
            "RAM",
            model.telemetry.ram_used_bytes,
            model.telemetry.ram_total_bytes,
        );
        let gpu = format_percent("GPU", model.telemetry.gpu_percent);
        let net = format_network(
            model.telemetry.network_rx_bytes_per_sec,
            model.telemetry.network_tx_bytes_per_sec,
        );
        let status = format!("{cpu}     {ram}     {gpu}     {net}");
        text(
            painter,
            pos2(126.0, y + 15.0),
            Align2::LEFT_CENTER,
            status,
            9.5,
            TEXT_MUTED,
            false,
        );
        text(
            painter,
            pos2(width as f32 - 14.0, y + 15.0),
            Align2::RIGHT_CENTER,
            "CHROMIUM X11 • NATIVE CHROME",
            9.5,
            TEXT_MUTED,
            false,
        );
    }

    fn paint_home(&self, painter: &Painter, model: &ChromeModel, width: u32, height: u32) {
        let content = content_rect(model.layout, width, height);
        let cp = painter.with_clip_rect(content);
        cp.rect_filled(content, 0.0, BG);
        if let Some(hero) = &self.hero {
            cp.image(
                hero.id(),
                content,
                Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        cp.rect_filled(content, 0.0, Color32::from_rgba_premultiplied(5, 4, 17, 88));

        let hero_h = (content.height() * 0.44).clamp(260.0, 350.0);
        let hero = Rect::from_min_max(content.min, pos2(content.right(), content.top() + hero_h));
        cp.rect_filled(hero, 0.0, Color32::from_rgba_premultiplied(9, 5, 28, 44));

        let left = hero.left() + 34.0;
        text(
            &cp,
            pos2(left, hero.top() + 42.0),
            Align2::LEFT_TOP,
            "AETHERFORGE",
            13.0,
            CYAN,
            true,
        );
        text(
            &cp,
            pos2(left, hero.top() + 66.0),
            Align2::LEFT_TOP,
            "Aether Browser",
            42.0,
            TEXT,
            true,
        );
        text(
            &cp,
            pos2(left, hero.top() + 118.0),
            Align2::LEFT_TOP,
            "Built for those who play, create, and explore.",
            16.0,
            TEXT_SOFT,
            false,
        );

        let mut chip_x = left;
        for (label, accent) in [
            ("Browse Freely", VIOLET),
            ("Stream Instantly", INDIGO),
            ("Keep It Yours", CYAN),
            ("Go Further", GOLD),
        ] {
            let width = label.len() as f32 * 7.3 + 30.0;
            let chip = Rect::from_min_size(pos2(chip_x, hero.top() + 160.0), vec2(width, 30.0));
            cp.rect_filled(
                chip,
                17.0,
                Color32::from_rgba_premultiplied(accent.r(), accent.g(), accent.b(), 82),
            );
            text(
                &cp,
                chip.center(),
                Align2::CENTER_CENTER,
                label,
                11.0,
                TEXT,
                true,
            );
            chip_x += width + 10.0;
        }

        let side_x = hero.right() - 270.0;
        text(
            &cp,
            pos2(side_x, hero.top() + 50.0),
            Align2::LEFT_TOP,
            "MORE THAN A BROWSER",
            12.0,
            VIOLET,
            true,
        );
        text(
            &cp,
            pos2(side_x, hero.top() + 84.0),
            Align2::LEFT_TOP,
            "SPEED",
            11.0,
            TEXT_SOFT,
            true,
        );
        text(
            &cp,
            pos2(side_x, hero.top() + 108.0),
            Align2::LEFT_TOP,
            "PRIVACY",
            11.0,
            TEXT_SOFT,
            true,
        );
        text(
            &cp,
            pos2(side_x, hero.top() + 132.0),
            Align2::LEFT_TOP,
            "CREATORS",
            11.0,
            TEXT_SOFT,
            true,
        );
        text(
            &cp,
            pos2(side_x, hero.top() + 156.0),
            Align2::LEFT_TOP,
            "WITHOUT LIMITS",
            11.0,
            TEXT_SOFT,
            true,
        );
        cp.line_segment(
            [
                pos2(side_x - 18.0, hero.top() + 46.0),
                pos2(side_x - 18.0, hero.top() + 186.0),
            ],
            Stroke::new(2.0_f32, VIOLET),
        );

        let cards_top = hero.bottom() - 2.0;
        let cards_bottom = content.bottom() - 10.0;
        let available_h = (cards_bottom - cards_top).max(180.0);
        let gap = 10.0;
        let quick_w = 176.0;
        let main_w = ((content.width() - quick_w - gap * 4.0) / 2.0).max(250.0);
        let stream = Rect::from_min_size(
            pos2(content.left() + gap, cards_top + gap),
            vec2(main_w, available_h - gap),
        );
        let vault = Rect::from_min_size(
            pos2(stream.right() + gap, cards_top + gap),
            vec2(main_w, available_h - gap),
        );
        let quick = Rect::from_min_max(
            pos2(vault.right() + gap, cards_top + gap),
            pos2(content.right() - gap, cards_bottom),
        );

        self.feature_card(
            &cp,
            stream,
            "AETHER STREAM STUDIO",
            "Go live, control scenes, and monitor output.",
            self.stream_preview.as_ref(),
            INDIGO,
        );
        self.feature_card(
            &cp,
            vault,
            "AETHER VAULT",
            "Private credentials, sessions, and protected data.",
            self.vault_preview.as_ref(),
            VIOLET,
        );

        cp.rect_filled(
            quick,
            14.0,
            Color32::from_rgba_premultiplied(14, 13, 34, 225),
        );
        text(
            &cp,
            pos2(quick.left() + 14.0, quick.top() + 18.0),
            Align2::LEFT_CENTER,
            "QUICK ACTIONS",
            10.0,
            VIOLET,
            true,
        );
        for (i, (glyph, label)) in [
            ("M", "Media Hub"),
            ("S", "Stream Studio"),
            ("C", "Creator Hub"),
        ]
        .iter()
        .enumerate()
        {
            let y = quick.top() + 42.0 + i as f32 * 40.0;
            cp.circle_filled(
                pos2(quick.left() + 22.0, y),
                13.0,
                if i % 2 == 0 { VIOLET } else { INDIGO },
            );
            text(
                &cp,
                pos2(quick.left() + 22.0, y),
                Align2::CENTER_CENTER,
                glyph,
                9.0,
                TEXT,
                true,
            );
            text(
                &cp,
                pos2(quick.left() + 44.0, y),
                Align2::LEFT_CENTER,
                label,
                11.0,
                TEXT_SOFT,
                false,
            );
        }
    }

    fn feature_card(
        &self,
        painter: &Painter,
        rect: Rect,
        title: &str,
        subtitle: &str,
        texture: Option<&TextureHandle>,
        accent: Color32,
    ) {
        painter.rect_filled(
            rect,
            14.0,
            Color32::from_rgba_premultiplied(13, 12, 32, 230),
        );
        text(
            painter,
            pos2(rect.left() + 16.0, rect.top() + 18.0),
            Align2::LEFT_CENTER,
            title,
            11.0,
            accent,
            true,
        );
        text(
            painter,
            pos2(rect.left() + 16.0, rect.top() + 41.0),
            Align2::LEFT_CENTER,
            subtitle,
            11.0,
            TEXT_SOFT,
            false,
        );
        let image_rect = Rect::from_min_max(
            pos2(rect.left() + 14.0, rect.top() + 64.0),
            pos2(rect.right() - 14.0, rect.bottom() - 14.0),
        );
        painter.rect_filled(image_rect, 10.0, Color32::from_rgb(9, 8, 24));
        if let Some(texture) = texture {
            painter.image(
                texture.id(),
                image_rect,
                Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                Color32::from_white_alpha(210),
            );
        }
    }
}

fn decode_texture(ctx: &Context, name: &str, bytes: &[u8]) -> Option<TextureHandle> {
    let image = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let pixels = image.into_raw();
    let color = ColorImage::from_rgba_unmultiplied(size, &pixels);
    Some(ctx.load_texture(name, color, TextureOptions::LINEAR))
}

fn utility_card(
    painter: &Painter,
    rect: Rect,
    title: &str,
    value: &str,
    glyph: &str,
    accent: Color32,
) {
    painter.rect_filled(rect, 14.0, GLASS);
    painter.circle_filled(pos2(rect.left() + 24.0, rect.center().y), 15.0, accent);
    text(
        painter,
        pos2(rect.left() + 24.0, rect.center().y),
        Align2::CENTER_CENTER,
        glyph,
        9.0,
        TEXT,
        true,
    );
    text(
        painter,
        pos2(rect.left() + 49.0, rect.top() + 24.0),
        Align2::LEFT_CENTER,
        title,
        10.0,
        accent,
        true,
    );
    text(
        painter,
        pos2(rect.left() + 49.0, rect.top() + 49.0),
        Align2::LEFT_CENTER,
        value,
        11.0,
        TEXT_SOFT,
        false,
    );
}

fn content_rect(layout: ChromeLayout, width: u32, height: u32) -> Rect {
    let (x, y, w, h) = layout.web_content_rect(width, height);
    Rect::from_min_size(pos2(x as f32, y as f32), vec2(w as f32, h as f32))
}

fn text(
    painter: &Painter,
    pos: Pos2,
    anchor: Align2,
    value: impl ToString,
    size: f32,
    color: Color32,
    strong: bool,
) -> Rect {
    let _ = strong;
    painter.text(
        pos,
        anchor,
        value,
        FontId::new(size, FontFamily::Proportional),
        color,
    )
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let head: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

fn download_text(count: usize) -> String {
    match count {
        0 => "No active downloads".to_owned(),
        1 => "1 active download".to_owned(),
        value => format!("{value} active downloads"),
    }
}

fn format_percent(label: &str, value: Option<u8>) -> String {
    value.map_or_else(
        || format!("{label} N/A"),
        |value| format!("{label} {value}%"),
    )
}

fn format_memory_pair(label: &str, used: Option<u64>, total: Option<u64>) -> String {
    match (used, total) {
        (Some(used), Some(total)) if total > 0 => format!(
            "{label} {}.{:01}/{}.{:01}G",
            used / 1_073_741_824,
            (used % 1_073_741_824) * 10 / 1_073_741_824,
            total / 1_073_741_824,
            (total % 1_073_741_824) * 10 / 1_073_741_824
        ),
        _ => format!("{label} N/A"),
    }
}

fn format_network(rx: Option<u64>, tx: Option<u64>) -> String {
    match (rx, tx) {
        (Some(rx), Some(tx)) => format!("NET ↓{}/s ↑{}/s", format_rate(rx), format_rate(tx)),
        _ => "NET N/A".to_owned(),
    }
}

fn format_rate(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!(
            "{}.{:01}M",
            bytes / 1_048_576,
            (bytes % 1_048_576) * 10 / 1_048_576
        )
    } else {
        format!("{}K", bytes / 1024)
    }
}

fn format_fps(label: &str, milli: Option<u32>) -> String {
    milli.map_or_else(
        || format!("{label} N/A"),
        |value| format!("{label} {}.{:03}", value / 1000, value % 1000),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_layout_matches_native_surface_contract() {
        let layout = ChromeLayout::canonical();
        assert_eq!(layout.launcher_width(), 52);
        assert_eq!(layout.utility_width(), 176);
        assert_eq!(layout.with_utility_collapsed(true).utility_width(), 38);
        assert_eq!(RESIZE_BORDER_PX, 7);
        assert_eq!(MIN_WINDOW_WIDTH_PX, 760);
        assert_eq!(MIN_WINDOW_HEIGHT_PX, 520);
        assert_eq!(layout.status_height(), 30);
        assert_eq!(layout.web_content_rect(1672, 941), (52, 108, 1444, 803));
        assert_eq!(layout.web_content_rect(1669, 937), (52, 108, 1441, 799));
        assert_eq!(layout.web_content_rect(1920, 1080), (52, 108, 1692, 942));
        assert_eq!(layout.web_content_rect(2560, 1440), (52, 108, 2332, 1302));
        assert_eq!(layout.web_content_rect(760, 520), (52, 108, 532, 382));
        let collapsed = layout.with_utility_collapsed(true);
        assert_eq!(collapsed.web_content_rect(1669, 937), (52, 108, 1579, 799));
        let (_, y, _, h) = layout.web_content_rect(1669, 937);
        assert_eq!(y + h, 937 - STATUS_BAR_HEIGHT_PX);
    }

    #[test]
    fn window_controls_are_left_aligned_and_non_overlapping() {
        let model = ChromeModel::new(Vec::new(), "aether://home");
        assert_eq!(
            model.hit_test(WINDOW_CONTROL_MINIMIZE_X as f64, 17.0, 1669, 937),
            ChromeHitTarget::WindowMinimize
        );
        assert_eq!(
            model.hit_test(WINDOW_CONTROL_MAXIMIZE_X as f64, 17.0, 1669, 937),
            ChromeHitTarget::WindowMaximize
        );
        assert_eq!(
            model.hit_test(WINDOW_CONTROL_DIMINISH_X as f64, 17.0, 1669, 937),
            ChromeHitTarget::WindowDiminish
        );
        assert_eq!(
            model.hit_test(WINDOW_CONTROL_CLOSE_X as f64, 17.0, 1669, 937),
            ChromeHitTarget::WindowClose
        );
        assert_eq!(
            model.hit_test(220.0, 17.0, 1669, 937),
            ChromeHitTarget::WindowDrag
        );
        println!("AETHER_BROWSER_WINDOW_CONTROLS_LEFT=PASS");
    }

    #[test]
    fn home_uses_public_display_url_without_dom_patch() {
        let mut model = ChromeModel::new(Vec::new(), "aether://home");
        model.home_mode = true;
        assert_eq!(model.display_omnibox(), HOME_DISPLAY_URL);
        model.home_mode = false;
        model.omnibox = "https://example.com/".into();
        assert_eq!(model.display_omnibox(), "https://example.com/");
    }
}
