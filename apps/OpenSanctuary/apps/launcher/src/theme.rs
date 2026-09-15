use eframe::egui::{self, Color32, Vec2};

pub(super) const PANEL_BG: Color32 = Color32::from_rgb(4, 8, 14);
pub(super) const SURFACE_BG: Color32 = Color32::from_rgb(8, 15, 23);
pub(super) const EXTREME_BG: Color32 = Color32::from_rgb(3, 6, 11);
pub(super) const ACCENT_BLUE: Color32 = Color32::from_rgb(45, 157, 225);
pub(super) const SUCCESS_GREEN: Color32 = Color32::from_rgb(70, 190, 132);
pub(super) const WARNING_AMBER: Color32 = Color32::from_rgb(221, 153, 65);
pub(super) const DANGER_RED: Color32 = Color32::from_rgb(204, 83, 83);

pub(super) fn configure_style(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.spacing.item_spacing = Vec2::new(8.0, 6.0);
        style.spacing.button_padding = Vec2::new(13.0, 7.0);
        style.visuals.dark_mode = true;
        style.visuals.panel_fill = PANEL_BG;
        style.visuals.window_fill = SURFACE_BG;
        style.visuals.extreme_bg_color = EXTREME_BG;
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(16, 27, 39);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(23, 65, 99);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(20, 107, 177);
    });
}
