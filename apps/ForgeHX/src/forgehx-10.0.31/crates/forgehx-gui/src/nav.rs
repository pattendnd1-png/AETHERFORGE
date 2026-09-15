use crate::{app::Page, theme};
use eframe::egui::{self, RichText};
use forgehx_core::{DeviceId, DeviceInfo};

pub fn show(
    ctx: &egui::Context,
    page: &mut Page,
    hyperx_devices: &[DeviceInfo],
    selected_device: &mut Option<DeviceId>,
) {
    egui::SidePanel::left("forgehx_nav")
        .resizable(false)
        .exact_width(218.0)
        .frame(
            egui::Frame::new()
                .fill(theme::bg_nav())
                .inner_margin(egui::Margin::same(14)),
        )
        .show(ctx, |ui| {
            ui.label(
                RichText::new("FORGEHX")
                    .size(23.0)
                    .strong()
                    .color(theme::accent()),
            );
            ui.label(
                RichText::new("DEVICE CONTROL")
                    .small()
                    .color(theme::text_secondary()),
            );
            ui.add_space(22.0);

            nav_button(ui, page, Page::Home, "⌂  Home");
            nav_button(ui, page, Page::Devices, "◇  Devices");
            nav_button(ui, page, Page::OutputDsp, "≋  Output DSP");
            nav_button(ui, page, Page::Profiles, "▱  Profiles");
            nav_button(ui, page, Page::Settings, "⚙  Settings");

            ui.add_space(24.0);
            ui.separator();
            ui.add_space(10.0);
            ui.label(
                RichText::new("HYPERX DEVICES")
                    .small()
                    .strong()
                    .color(theme::text_secondary()),
            );
            ui.add_space(6.0);
            if hyperx_devices.is_empty() {
                ui.small("No HyperX devices detected");
            }
            for device in hyperx_devices {
                let selected = selected_device.as_ref() == Some(&device.id);
                if ui.selectable_label(selected, &device.name).clicked() {
                    *selected_device = Some(device.id.clone());
                    *page = Page::Devices;
                }
                ui.small(
                    RichText::new(device.support_level.to_string()).color(theme::text_secondary()),
                );
                ui.add_space(5.0);
            }
        });
}

fn nav_button(ui: &mut egui::Ui, page: &mut Page, target: Page, label: &str) {
    let selected = *page == target;
    if ui
        .add_sized([188.0, 34.0], egui::Button::new(label).selected(selected))
        .clicked()
    {
        *page = target;
    }
}
