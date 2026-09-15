use crate::{theme, widgets};
use eframe::egui::{self, RichText};

pub fn show(ui: &mut egui::Ui, profiles: &[String]) {
    widgets::page_heading(
        ui,
        "Profiles",
        "Persistent ForgeHX profiles remain compatible with the v0.1 schema.",
    );
    if profiles.is_empty() {
        ui.label("No profiles saved yet.");
        ui.label(
            RichText::new("Use `forgehx profile save <file>` to import a profile.")
                .color(theme::text_secondary()),
        );
    } else {
        for profile in profiles {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(profile).size(18.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("Profile schema v1").color(theme::text_secondary()));
                    });
                });
            });
            ui.add_space(7.0);
        }
    }
}
