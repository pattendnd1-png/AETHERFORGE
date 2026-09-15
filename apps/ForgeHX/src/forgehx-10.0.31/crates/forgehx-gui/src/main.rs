mod app;
mod device_page;
mod equalizer;
mod firmware;
mod hardware;
mod home;
mod ipc;
mod keyboard;
mod lighting;
mod microphone;
mod mouse;
mod nav;
mod output_dsp;
mod profiles;
mod settings;
mod theme;
mod widgets;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([980.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ForgeHX",
        options,
        Box::new(|cc| Ok(Box::new(app::ForgeHxApp::new(&cc.egui_ctx)))),
    )
}
