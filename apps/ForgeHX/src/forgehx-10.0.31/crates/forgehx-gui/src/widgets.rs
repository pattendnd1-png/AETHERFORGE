use crate::theme;
use eframe::egui::{self, RichText};
use forgehx_core::{Capability, DeviceInfo, SupportLevel};

pub fn page_heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.heading(RichText::new(title).size(28.0).strong());
    ui.label(RichText::new(subtitle).color(theme::text_secondary()));
    ui.add_space(14.0);
}

pub fn support_label(ui: &mut egui::Ui, support: SupportLevel) {
    let color = match support {
        SupportLevel::FullySupported => theme::status_ok(),
        SupportLevel::PartiallySupported | SupportLevel::GenericControls => theme::status_warn(),
        SupportLevel::DiagnosticOnly | SupportLevel::Unavailable => theme::status_blocked(),
    };
    ui.label(RichText::new(support.to_string()).color(color).strong());
}

pub fn capability_chips(ui: &mut egui::Ui, device: &DeviceInfo) {
    ui.horizontal_wrapped(|ui| {
        for capability in device.all_capabilities() {
            ui.label(
                RichText::new(capability_label(capability))
                    .small()
                    .color(theme::text_secondary()),
            );
        }
    });
}

pub fn device_card(ui: &mut egui::Ui, device: &DeviceInfo, selected: bool) -> egui::Response {
    let label = format!(
        "{}\n{}  •  {}\n{}",
        device.name, device.vendor_family, device.device_class, device.support_level
    );
    ui.add_sized(
        [260.0, 82.0],
        egui::Button::new(label)
            .selected(selected)
            .fill(if selected {
                theme::bg_panel()
            } else {
                theme::bg_card()
            }),
    )
}

pub fn info_row(ui: &mut egui::Ui, key: &str, value: impl ToString) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(key).color(theme::text_secondary()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(value.to_string());
        });
    });
}

fn capability_label(capability: Capability) -> &'static str {
    match capability {
        Capability::Diagnostics => "Diagnostics",
        Capability::Lighting => "Lighting",
        Capability::Dpi => "DPI",
        Capability::PollingRate => "Polling Rate",
        Capability::Profiles => "On-device Profiles",
        Capability::Bindings => "Assignments",
        Capability::MacroAssignments => "Macros",
        Capability::Audio => "Audio",
        Capability::Microphone => "Microphone",
        Capability::MicDsp => "Input DSP",
        Capability::MicFirmwareInventory => "Firmware",
        Capability::MicFirmwareUpdate => "Firmware Update",
        Capability::MicFirmwareRecovery => "Firmware Recovery",
        Capability::Eq => "EQ",
        Capability::AudioRouting => "Audio Routing",
        Capability::BatteryStatus => "Battery",
        Capability::Volume => "Volume",
        Capability::Mute => "Mute",
    }
}
