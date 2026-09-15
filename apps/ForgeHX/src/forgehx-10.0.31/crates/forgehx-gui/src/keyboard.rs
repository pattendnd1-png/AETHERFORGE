use eframe::egui::{self, RichText};
use forgehx_core::{Capability, DeviceInfo, KeyboardLightingTopology, KeyboardModelInfo};

pub fn show(ui: &mut egui::Ui, device: &DeviceInfo, model: Option<&KeyboardModelInfo>) {
    ui.heading("Keyboard");
    ui.label("HyperX keyboard identity, topology, performance limits, and capability ownership.");
    ui.add_space(8.0);

    let Some(model) = model else {
        ui.label(
            RichText::new("No HyperX keyboard registry metadata is available for this device yet.")
                .strong(),
        );
        ui.label("Standard Linux keyboard input remains available. Proprietary writes stay disabled until the hardware identity and protocol are verified.");
        show_ownership(ui, device);
        return;
    };

    egui::Grid::new("forgehx_keyboard_model_grid")
        .num_columns(2)
        .spacing([18.0, 6.0])
        .show(ui, |ui| {
            row(ui, "Model", &model.name);
            row(ui, "Protocol family", &model.protocol_family.to_string());
            row(
                ui,
                "Identity",
                if model.exact_hardware_match {
                    "Exact hardware match"
                } else {
                    "Metadata match only"
                },
            );
            row(ui, "Lighting", &lighting_label(model));
            row(ui, "Polling", &polling_label(model));
            row(
                ui,
                "Onboard profiles",
                &model
                    .limits
                    .onboard_profiles
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "Unknown".into()),
            );
            row(ui, "Wireless", yes_no(model.limits.wireless));
            row(ui, "Battery telemetry", yes_no(model.limits.battery));
            row(ui, "Hall-effect keys", yes_no(model.limits.hall_effect));
            row(ui, "Rapid Trigger", yes_no(model.limits.rapid_trigger));
            row(
                ui,
                "ForgeHX native driver",
                model.native_driver.as_deref().unwrap_or("Not registered"),
            );
        });

    ui.add_space(10.0);
    if !model.exact_hardware_match {
        ui.label(RichText::new("Metadata match only — proprietary writes disabled.").strong());
    } else if model.native_driver.is_none() {
        ui.label(RichText::new("Exact device identified; this release still uses safe Linux/OpenRGB backends. No ForgeHX native keyboard writer is registered.").strong());
    }

    show_ownership(ui, device);
}

fn show_ownership(ui: &mut egui::Ui, device: &DeviceInfo) {
    ui.add_space(12.0);
    ui.heading("Capability ownership");
    if device.capability_owners.is_empty() {
        ui.label("No writable keyboard capability backend is currently selected.");
        return;
    }

    for owner in &device.capability_owners {
        if owner.capability == Capability::Diagnostics {
            continue;
        }
        let mode = if owner.writable {
            "writable"
        } else {
            "read-only"
        };
        ui.label(format!(
            "{:?}: {} ({mode})",
            owner.capability, owner.backend
        ));
    }

    if let Some(owner) = device.selected_owner(Capability::Lighting) {
        if owner.backend == forgehx_core::BackendKind::OpenRgb {
            ui.label("Lighting backend: OpenRGB");
        }
    }
}

fn row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.label(RichText::new(label).strong());
    ui.label(value);
    ui.end_row();
}

fn lighting_label(model: &KeyboardModelInfo) -> String {
    match model.limits.lighting {
        KeyboardLightingTopology::None => "Not declared".into(),
        KeyboardLightingTopology::Zone { zones } => format!("{zones}-zone RGB"),
        KeyboardLightingTopology::PerKey => "Per-key RGB".into(),
    }
}

fn polling_label(model: &KeyboardModelInfo) -> String {
    if model.limits.polling_rates_hz.is_empty() {
        "Unknown".into()
    } else {
        model
            .limits
            .polling_rates_hz
            .iter()
            .map(|rate| format!("{rate} Hz"))
            .collect::<Vec<_>>()
            .join(" / ")
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "Yes"
    } else {
        "No"
    }
}
