use crate::{theme, widgets};
use eframe::egui::{self, RichText};
use forgehx_core::{DeviceId, DeviceInfo};

pub fn show(ui: &mut egui::Ui, devices: &[DeviceInfo]) -> Option<DeviceId> {
    widgets::page_heading(
        ui,
        "Home",
        "Your HyperX gear, profiles, and support status at a glance.",
    );
    if devices.is_empty() {
        ui.add_space(70.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("No HyperX hardware detected").size(22.0).strong());
            ui.label(RichText::new("Connect a supported HyperX keyboard, mouse, headset, microphone, controller, camera, monitor, or illuminated accessory.").color(theme::text_secondary()));
        });
        return None;
    }

    ui.horizontal_wrapped(|ui| {
        for device in devices {
            if widgets::device_card(ui, device, false).clicked() {
                return Some(device.id.clone());
            }
        }
        None
    })
    .inner
}
