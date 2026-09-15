use eframe::egui::{self, Color32};

pub fn bg_app() -> Color32 {
    Color32::from_rgb(20, 20, 23)
}
pub fn bg_nav() -> Color32 {
    Color32::from_rgb(14, 14, 17)
}
pub fn bg_panel() -> Color32 {
    Color32::from_rgb(28, 28, 32)
}
pub fn bg_card() -> Color32 {
    Color32::from_rgb(36, 36, 41)
}
pub fn accent() -> Color32 {
    Color32::from_rgb(235, 39, 52)
}
pub fn text_primary() -> Color32 {
    Color32::from_rgb(244, 244, 246)
}
pub fn text_secondary() -> Color32 {
    Color32::from_rgb(164, 166, 174)
}
pub fn status_ok() -> Color32 {
    Color32::from_rgb(76, 194, 121)
}
pub fn status_warn() -> Color32 {
    Color32::from_rgb(236, 176, 63)
}
pub fn status_blocked() -> Color32 {
    Color32::from_rgb(224, 78, 78)
}

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = bg_app();
    visuals.window_fill = bg_panel();
    visuals.extreme_bg_color = bg_nav();
    visuals.override_text_color = Some(text_primary());
    visuals.selection.bg_fill = accent();
    visuals.hyperlink_color = accent();
    ctx.set_visuals(visuals);
}
