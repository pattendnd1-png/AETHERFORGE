use crate::{theme, widgets};
use eframe::egui::{self, RichText};
use forgehx_core::{Capability, DeviceInfo, DpiConfig, MouseDeviceState, MouseLimits};

#[derive(Debug, Clone)]
pub enum MouseControl {
    Dpi(DpiConfig),
    PollingRate(u16),
    Profile(u8),
    Button { button: u32, action: String },
    LiftOffDistance(u8),
}

#[allow(clippy::too_many_arguments)]
pub fn show(
    ui: &mut egui::Ui,
    device: &DeviceInfo,
    state: Option<&MouseDeviceState>,
    mouse_limits: Option<&MouseLimits>,
    stages: &mut [u16],
    active: &mut usize,
    polling_rate: &mut u16,
    mouse_profile: &mut u8,
    button_number: &mut u32,
    button_action: &mut String,
    lift_off_distance: &mut u8,
) -> Option<MouseControl> {
    ui.heading("Performance");
    let dpi_min = mouse_limits
        .and_then(|limits| limits.dpi_min)
        .unwrap_or(100);
    let dpi_max = mouse_limits
        .and_then(|limits| limits.dpi_max)
        .unwrap_or(30000);
    let dpi_step = mouse_limits
        .and_then(|limits| limits.dpi_step)
        .unwrap_or(50);
    let fallback_polling = [125u16, 250, 500, 1000, 2000, 4000, 8000];
    let polling_rates = mouse_limits
        .map(|limits| limits.polling_rates_hz.as_slice())
        .filter(|rates| !rates.is_empty())
        .unwrap_or(&fallback_polling);
    if let Some(state) = state {
        widgets::info_row(ui, "Backend device", &state.backend_device_id);
        if let Some(rate) = state.report_rate_hz {
            widgets::info_row(ui, "Current polling", format!("{rate} Hz"));
        }
        if let Some(profile) = state.active_profile {
            widgets::info_row(ui, "Active profile", profile);
        }
        if let Some(mm) = state.lift_off_distance_mm {
            widgets::info_row(ui, "Lift-off distance", format!("{mm} mm"));
        }
    }
    if let Some(owner) = device
        .selected_owner(Capability::Dpi)
        .or_else(|| device.selected_owner(Capability::PollingRate))
    {
        widgets::info_row(ui, "Mouse backend", owner.backend);
    }
    ui.label(RichText::new("Changes are written live. Continuous DPI edits are briefly debounced, then ForgeHX reads the hardware state back.").small().color(theme::text_secondary()));
    ui.add_space(8.0);

    if device.supports(Capability::Dpi) {
        ui.label(RichText::new("DPI stages").strong());
        let writable = is_writable(device, Capability::Dpi);
        let mut dpi_changed = false;
        ui.add_enabled_ui(writable, |ui| {
            for (index, dpi) in stages.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    if ui
                        .radio_value(active, index, format!("Stage {}", index + 1))
                        .changed()
                    {
                        dpi_changed = true;
                    }
                    if ui
                        .add(
                            egui::DragValue::new(dpi)
                                .range(dpi_min..=dpi_max)
                                .speed(f64::from(dpi_step)),
                        )
                        .changed()
                    {
                        dpi_changed = true;
                    }
                });
            }
        });
        if dpi_changed {
            return Some(MouseControl::Dpi(DpiConfig {
                stages: stages.to_owned(),
                active_stage: *active,
            }));
        }
        ui.add_space(10.0);
    }

    if device.supports(Capability::PollingRate) {
        let writable = is_writable(device, Capability::PollingRate);
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Polling rate");
            ui.add_enabled_ui(writable, |ui| {
                egui::ComboBox::from_id_salt("forgehx_polling_rate")
                    .selected_text(format!("{} Hz", polling_rate))
                    .show_ui(ui, |ui| {
                        for &rate in polling_rates {
                            if ui
                                .selectable_value(polling_rate, rate, format!("{rate} Hz"))
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });
            });
        });
        if changed {
            let action = Some(MouseControl::PollingRate(*polling_rate));
            return action;
        }
    }

    if let Some(distances) = mouse_limits
        .map(|limits| limits.lift_off_distances_mm.as_slice())
        .filter(|values| !values.is_empty())
    {
        let writable = is_writable(device, Capability::Dpi);
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Lift-off distance");
            if !distances.contains(lift_off_distance) {
                *lift_off_distance = distances[0];
            }
            ui.add_enabled_ui(writable, |ui| {
                egui::ComboBox::from_id_salt("forgehx_lift_off_distance")
                    .selected_text(format!("{} mm", lift_off_distance))
                    .show_ui(ui, |ui| {
                        for &mm in distances {
                            if ui
                                .selectable_value(lift_off_distance, mm, format!("{mm} mm"))
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });
            });
        });
        if changed {
            return Some(MouseControl::LiftOffDistance(*lift_off_distance));
        }
    }

    if device.supports(Capability::Profiles) {
        let writable = is_writable(device, Capability::Profiles);
        let max_profile = mouse_limits
            .and_then(|limits| limits.onboard_profiles)
            .map(|count| count.saturating_sub(1))
            .unwrap_or(7);
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("On-device profile");
            ui.add_enabled_ui(writable, |ui| {
                changed = ui
                    .add(egui::DragValue::new(mouse_profile).range(0..=max_profile))
                    .changed();
            });
        });
        if changed {
            let action = Some(MouseControl::Profile(*mouse_profile));
            return action;
        }
    }

    if device.supports(Capability::Bindings) {
        ui.separator();
        ui.label(RichText::new("Button assignment").strong());
        let writable = is_writable(device, Capability::Bindings);
        let max_button = mouse_limits
            .and_then(|limits| limits.button_count)
            .map(|count| count.saturating_sub(1) as u32)
            .unwrap_or(31);
        let mut assignment_changed = false;
        ui.add_enabled_ui(writable, |ui| {
            ui.horizontal(|ui| {
                ui.label("Button");
                if ui
                    .add(egui::DragValue::new(button_number).range(0..=max_button))
                    .changed()
                {
                    if let Some(binding) = state
                        .and_then(|state| state.button_assignments.get(*button_number as usize))
                    {
                        *button_action = binding.action.clone();
                    }
                }
                let response = ui.text_edit_singleline(button_action);
                assignment_changed =
                    response.changed() && assignment_syntax_looks_complete(button_action);
            });
        });
        if let Some(state) = state {
            if !state.button_assignments.is_empty() {
                ui.label(RichText::new("Current assignments").small().strong());
                for (index, binding) in state.button_assignments.iter().enumerate() {
                    ui.label(format!("Button {index}: {}", binding.action));
                }
            }
        }
        ui.label(RichText::new("Haste Wireless native actions: disabled, dpi-toggle, mouse:left/right/middle/back/forward, key:<HID usage>, media:play-pause/stop/previous/next/mute/volume-down/volume-up, shortcut:cut/copy/paste/close-window, or macro:<HID usage>[+modifier]. Valid assignments commit as soon as the entry is complete.").small().color(theme::text_secondary()));
        if assignment_changed {
            return Some(MouseControl::Button {
                button: *button_number,
                action: button_action.trim().to_owned(),
            });
        }
    }
    None
}

fn assignment_syntax_looks_complete(action: &str) -> bool {
    let action = action.trim();
    if matches!(action, "disabled" | "dpi-toggle") {
        return true;
    }
    let Some((kind, value)) = action.split_once(':') else {
        return false;
    };
    if value.trim().is_empty() {
        return false;
    }
    match kind {
        "mouse" => matches!(value, "left" | "right" | "middle" | "back" | "forward"),
        "media" => matches!(
            value,
            "play-pause" | "stop" | "previous" | "next" | "mute" | "volume-down" | "volume-up"
        ),
        "shortcut" => matches!(
            value,
            "task-manager"
                | "system-utility"
                | "show-desktop"
                | "cycle-apps"
                | "close-window"
                | "cut"
                | "copy"
                | "paste"
        ),
        "key" | "macro" => true,
        _ => false,
    }
}

fn is_writable(device: &DeviceInfo, capability: Capability) -> bool {
    device
        .selected_owner(capability)
        .map(|owner| owner.writable)
        .unwrap_or(false)
}
