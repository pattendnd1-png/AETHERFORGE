use aether_core::ConversationId;
pub use aetherforge_ui::{AetherForgeVisualContract, Rgba};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub struct DragonGlassTheme {
    pub app_fill: Rgba,
    pub panel_fill: Rgba,
    pub panel_fallback: Rgba,
    pub terminal_fill: Rgba,
    pub text_primary: Rgba,
    pub text_muted: Rgba,
    pub accent: Rgba,
    pub accent_secondary: Rgba,
    pub danger: Rgba,
    pub corner_radius: f32,
    pub compact_row_height: f32,
}

fn from_terminal_rgba(color: aether_terminal_ui::Rgba) -> Rgba {
    Rgba::new(color.r, color.g, color.b, color.a)
}

impl DragonGlassTheme {
    #[must_use]
    pub fn terminal_canonical() -> Self {
        let t = aether_terminal_ui::DragonGlassTheme::canonical();
        Self {
            app_fill: from_terminal_rgba(t.background),
            panel_fill: from_terminal_rgba(t.panel),
            panel_fallback: from_terminal_rgba(t.glass_surface_alt),
            terminal_fill: from_terminal_rgba(t.terminal_surface),
            text_primary: from_terminal_rgba(t.value_text),
            text_muted: from_terminal_rgba(t.label_text),
            accent: from_terminal_rgba(t.accent),
            accent_secondary: from_terminal_rgba(t.interaction_cyan),
            danger: from_terminal_rgba(t.decoration.close.active),
            corner_radius: t.decoration.window_radius,
            compact_row_height: t.decoration.title_height,
        }
    }
    #[must_use]
    pub fn contract() -> AetherForgeVisualContract {
        AetherForgeVisualContract::terminal_canonical()
    }
}
impl Default for DragonGlassTheme {
    fn default() -> Self {
        Self::terminal_canonical()
    }
}
#[must_use]
pub fn current_theme() -> DragonGlassTheme {
    DragonGlassTheme::terminal_canonical()
}
#[must_use]
pub fn current_visual_contract() -> AetherForgeVisualContract {
    AetherForgeVisualContract::terminal_canonical()
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PanelState {
    pub open: bool,
    pub width: f32,
    pub min_width: f32,
    pub max_width: f32,
}
impl PanelState {
    pub fn toggle(&mut self) {
        self.open = !self.open
    }
    pub fn set_width(&mut self, width: f32) {
        self.width = width.clamp(self.min_width, self.max_width)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesktopPage {
    Chat,
    Projects,
    Files,
    Terminal,
    Models,
    Artifacts,
    Search,
    Activity,
    Settings,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesktopUiState {
    pub navigation: PanelState,
    pub processing: PanelState,
    pub page: DesktopPage,
    pub last_conversation: Option<ConversationId>,
    pub ui_scale: f32,
    pub first_run_complete: bool,
}
impl Default for DesktopUiState {
    fn default() -> Self {
        Self {
            navigation: PanelState {
                open: true,
                width: 272.0,
                min_width: 190.0,
                max_width: 440.0,
            },
            processing: PanelState {
                open: true,
                width: 332.0,
                min_width: 260.0,
                max_width: 520.0,
            },
            page: DesktopPage::Chat,
            last_conversation: None,
            ui_scale: 1.0,
            first_run_complete: false,
        }
    }
}
impl DesktopUiState {
    pub fn conversation_width(&self, total: f32) -> f32 {
        let nav = if self.navigation.open {
            self.navigation.width
        } else {
            0.0
        };
        let proc = if self.processing.open {
            self.processing.width
        } else {
            0.0
        };
        (total - nav - proc).max(320.0)
    }
    pub fn responsive_collapse(&mut self, total: f32) {
        if total < 1080.0 {
            self.processing.open = false
        }
        if total < 760.0 {
            self.navigation.open = false
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dragon_glass_is_compact_and_translucent() {
        let t = DragonGlassTheme::default();
        assert!(t.panel_fill.a < 128);
        assert!(t.corner_radius >= 10.0);
        assert!(t.compact_row_height <= 34.0);
        assert!(t.text_primary.r > 220);
    }
    #[test]
    fn panel_width_survives_toggle() {
        let mut s = DesktopUiState::default();
        s.processing.set_width(360.0);
        s.processing.toggle();
        s.processing.toggle();
        assert_eq!(s.processing.width, 360.0);
    }
    #[test]
    fn both_panels_serialize() {
        let s = DesktopUiState::default();
        let j = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<DesktopUiState>(&j).unwrap(), s);
    }
    #[test]
    fn collapsed_panels_expand_chat() {
        let mut s = DesktopUiState::default();
        let full = s.conversation_width(1400.0);
        s.processing.open = false;
        assert!(s.conversation_width(1400.0) > full);
    }
}

#[cfg(test)]
mod v032_terminal_theme_tests {
    use super::*;

    #[test]
    fn v032_aether_terminal_theme_contract() {
        let t = DragonGlassTheme::default();
        assert_eq!(t.app_fill.a, 64);
        assert_eq!(t.terminal_fill.a, 64);
        assert_eq!(t.compact_row_height, 28.0);
        assert!(t.text_primary.r >= 240 && t.text_primary.b >= 250);
        assert!(t.accent.b >= 240);
        assert!(t.accent_secondary.b >= 240);
        assert!(t.panel_fill.a <= 96);
    }
}
