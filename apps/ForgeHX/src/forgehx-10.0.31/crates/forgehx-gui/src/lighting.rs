use crate::{theme, widgets};
use eframe::egui::{self, RichText};
use forgehx_core::{
    Capability, DeviceInfo, LightingConfig, LightingControllerMetadata, LightingEffect,
};

pub fn show(
    ui: &mut egui::Ui,
    device: &DeviceInfo,
    config: &mut LightingConfig,
    metadata: Option<&LightingControllerMetadata>,
) -> bool {
    let before = config.clone();
    ui.heading("Lights");
    let writable = device
        .selected_owner(Capability::Lighting)
        .map(|owner| owner.writable)
        .unwrap_or(false);
    if let Some(owner) = device.selected_owner(Capability::Lighting) {
        widgets::info_row(ui, "Control backend", owner.backend);
        widgets::info_row(
            ui,
            "Mapping",
            owner.backend_device_id.as_deref().unwrap_or("native"),
        );
        if !owner.writable {
            ui.colored_label(
                theme::status_blocked(),
                "This lighting backend is read-only.",
            );
        }
    } else {
        ui.colored_label(
            theme::status_blocked(),
            "No lighting backend owns this device.",
        );
    }
    if let Some(metadata) = metadata {
        ui.label(
            RichText::new(format!(
                "{} LEDs • {} zones • {} modes",
                metadata.led_count,
                metadata.zones.len(),
                metadata.modes.len()
            ))
            .color(theme::text_secondary()),
        );
    }
    ui.add_space(8.0);
    ui.add_enabled_ui(writable, |ui| {
        ui.color_edit_button_srgb(&mut config.color);
        ui.add(egui::Slider::new(&mut config.brightness, 0..=100).text("Brightness"));
        ui.add(egui::Slider::new(&mut config.speed, 0..=100).text("Effect speed"));
        egui::ComboBox::from_label("Effect")
            .selected_text(format!("{:?}", config.effect))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut config.effect, LightingEffect::Static, "Static");
                ui.selectable_value(&mut config.effect, LightingEffect::Breathing, "Breathing");
                ui.selectable_value(&mut config.effect, LightingEffect::Wave, "Wave");
                ui.selectable_value(&mut config.effect, LightingEffect::Spectrum, "Spectrum");
            });
    });
    if writable && config != &before {
        ui.label(
            RichText::new(
                "Live update queued — ForgeHX will reconcile the accepted values from hardware.",
            )
            .small()
            .color(theme::text_secondary()),
        );
        true
    } else {
        false
    }
}
