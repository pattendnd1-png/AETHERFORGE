mod app;
mod diagnostics;
mod keyboard;
mod services;
mod firmware;

use app::ReForgeApp;
use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 760.0])
            .with_min_inner_size([860.0, 620.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ReForge Logitech Control Center",
        options,
        Box::new(|creation_context| {
            creation_context
                .egui_ctx
                .set_theme(egui::ThemePreference::System);
            Ok(Box::new(ReForgeApp::new(&creation_context.egui_ctx)))
        }),
    )
}
