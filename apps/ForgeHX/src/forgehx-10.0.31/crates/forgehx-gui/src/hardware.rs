use crate::{theme, widgets};
use eframe::egui::{self, RichText};
use forgehx_core::{AudioNode, DeviceClass, DeviceInfo};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HardwareControl {
    SetVolume(u32, f32),
    SetMute(u32, bool),
}

pub fn show(
    ui: &mut egui::Ui,
    device: &DeviceInfo,
    audio_nodes: &[AudioNode],
) -> Option<HardwareControl> {
    ui.heading(format!("{} Hardware", device.device_class));
    ui.label(RichText::new(class_summary(device.device_class)).color(theme::text_secondary()));
    ui.add_space(10.0);

    ui.group(|ui| {
        ui.heading("Device status");
        if let Some(percent) = device.battery_percent {
            ui.add(
                egui::ProgressBar::new(percent as f32 / 100.0).text(format!("Battery {percent}%")),
            );
        } else {
            widgets::info_row(ui, "Battery", "Not reported");
        }
        if let Some(state) = &device.battery_state {
            widgets::info_row(ui, "Power state", state);
        }
        widgets::info_row(ui, "Connection", format!("{:?}", device.connection_kind));
        widgets::info_row(ui, "Support", device.support_level);
    });

    if matches!(
        device.device_class,
        DeviceClass::Headset | DeviceClass::AudioInterface | DeviceClass::UsbAudio
    ) {
        ui.add_space(10.0);
        if let Some(control) = audio_controls(ui, device, audio_nodes) {
            return Some(control);
        }
    }

    ui.add_space(10.0);
    ui.group(|ui| {
        ui.heading("Capability owners");
        if device.capability_owners.is_empty() {
            ui.label(
                "No verified writable backend is currently associated with this HyperX device.",
            );
        }
        for owner in &device.capability_owners {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(format!("{:?}", owner.capability)).strong());
                ui.label("Backend");
                ui.label(owner.backend.to_string());
                let mode = if owner.writable {
                    "Writable"
                } else {
                    "Read-only"
                };
                let color = if owner.writable {
                    theme::status_ok()
                } else {
                    theme::text_secondary()
                };
                ui.colored_label(color, mode);
                if let Some(detail) = &owner.detail {
                    ui.label(RichText::new(detail).small().color(theme::text_secondary()));
                }
            });
        }
    });
    None
}

fn audio_controls(
    ui: &mut egui::Ui,
    device: &DeviceInfo,
    nodes: &[AudioNode],
) -> Option<HardwareControl> {
    ui.heading("Audio controls");
    let ids = device
        .interfaces
        .iter()
        .filter_map(|interface| interface.audio_node_id)
        .collect::<Vec<_>>();
    let relevant = nodes
        .iter()
        .filter(|node| ids.contains(&node.id) && node.kind == "sink")
        .collect::<Vec<_>>();
    if relevant.is_empty() {
        ui.label("No associated PipeWire playback node is currently available.");
        return None;
    }
    for node in relevant {
        let mut volume = node.volume.unwrap_or(1.0);
        let mut muted = node.muted.unwrap_or(false);
        let mut action = None;
        ui.group(|ui| {
            ui.label(RichText::new(&node.name).strong());
            if ui
                .add(egui::Slider::new(&mut volume, 0.0..=1.5).text("Volume"))
                .changed()
            {
                action = Some(HardwareControl::SetVolume(node.id, volume));
            }
            if ui.checkbox(&mut muted, "Muted").changed() {
                action = Some(HardwareControl::SetMute(node.id, muted));
            }
        });
        if action.is_some() {
            return action;
        }
    }
    None
}

fn class_summary(class: DeviceClass) -> &'static str {
    match class {
        DeviceClass::Headset => "HyperX headset playback, battery, and verified backend settings.",
        DeviceClass::Controller => "HyperX controller status and verified capability ownership.",
        DeviceClass::Webcam => "HyperX camera status and verified capability ownership.",
        DeviceClass::Mousepad => {
            "HyperX illuminated surface status and verified capability ownership."
        }
        DeviceClass::Monitor => "HyperX display status and verified capability ownership.",
        DeviceClass::AudioInterface => {
            "HyperX audio-interface playback and verified backend settings."
        }
        DeviceClass::UsbAudio => "HyperX USB-audio playback and verified backend settings.",
        _ => "Verified HyperX hardware settings.",
    }
}
