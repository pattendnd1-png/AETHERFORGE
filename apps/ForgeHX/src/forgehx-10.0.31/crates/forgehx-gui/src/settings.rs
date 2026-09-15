use crate::{theme, widgets};
use eframe::egui::{self, RichText};
use forgehx_core::{BackendKind, BackendStatus};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiPreferences {
    pub automatic_inventory_refresh: bool,
    pub inventory_refresh_seconds: u64,
    pub refresh_after_changes: bool,
    pub show_backend_details: bool,
    pub live_mic_dsp_edits: bool,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            automatic_inventory_refresh: true,
            inventory_refresh_seconds: 1,
            refresh_after_changes: true,
            show_backend_details: true,
            live_mic_dsp_edits: true,
        }
    }
}

impl UiPreferences {
    pub fn load() -> Self {
        let path = preferences_path();
        let Ok(bytes) = fs::read(path) else {
            return Self::default();
        };
        let mut value: Self = serde_json::from_slice(&bytes).unwrap_or_default();
        value.inventory_refresh_seconds = value.inventory_refresh_seconds.clamp(1, 60);
        value
    }

    pub fn save(&self) -> Result<(), String> {
        let path = preferences_path();
        let parent = path
            .parent()
            .ok_or_else(|| "ForgeHX settings path has no parent".to_owned())?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, bytes).map_err(|error| error.to_string())?;
        fs::rename(&tmp, &path).map_err(|error| error.to_string())
    }
}

fn preferences_path() -> PathBuf {
    let root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"));
    root.join("forgehx/gui/settings.json")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsAction {
    Rescan,
    EnsureBackends,
    RestartBackend(BackendKind),
}

#[derive(Debug, Default)]
pub struct SettingsResponse {
    pub action: Option<SettingsAction>,
    pub changed: bool,
}

pub fn show(
    ui: &mut egui::Ui,
    status: &str,
    error: Option<&str>,
    hyperx_count: usize,
    backends: &[BackendStatus],
    preferences: &mut UiPreferences,
) -> SettingsResponse {
    let before = preferences.clone();
    let mut action = None;
    widgets::page_heading(ui, "Settings", "Daemon status, editable application behavior, control backends, discovery, and safety policy.");

    ui.group(|ui| {
        ui.heading("Application behavior");
        ui.checkbox(&mut preferences.automatic_inventory_refresh, "Automatic inventory refresh");
        ui.add_enabled_ui(preferences.automatic_inventory_refresh, |ui| {
            ui.add(egui::Slider::new(&mut preferences.inventory_refresh_seconds, 1..=60).text("External-state refresh interval (seconds)"));
        });
        ui.checkbox(&mut preferences.refresh_after_changes, "Refresh immediately after setting changes");
        ui.checkbox(&mut preferences.show_backend_details, "Show backend detail text");
        ui.label(RichText::new("Microphone DSP controls are always user-editable and apply live; the legacy live-edit preference is retained only for settings-file compatibility.").small().color(theme::text_secondary()));
    });

    ui.add_space(12.0);
    ui.group(|ui| {
        ui.heading("ForgeHX daemon");
        ui.label(status);
        if let Some(error) = error {
            ui.colored_label(theme::status_blocked(), error);
        }
        ui.add_space(6.0);
        widgets::info_row(ui, "HyperX devices", hyperx_count);
        widgets::info_row(
            ui,
            "Automatic inventory refresh",
            if preferences.automatic_inventory_refresh {
                format!("Every {} seconds", preferences.inventory_refresh_seconds)
            } else {
                "Off".into()
            },
        );
        widgets::info_row(
            ui,
            "Refresh after changes",
            if preferences.refresh_after_changes {
                "Immediate"
            } else {
                "Use fallback interval"
            },
        );
    });

    ui.add_space(12.0);
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.heading("Control backends");
            if ui.button("Ensure backends").clicked() {
                action = Some(SettingsAction::EnsureBackends);
            }
        });
        for backend in backends {
            ui.horizontal_wrapped(|ui| {
                let color = if backend.health.is_ready() {
                    theme::status_ok()
                } else {
                    theme::status_warn()
                };
                ui.colored_label(color, RichText::new(backend.backend.to_string()).strong());
                ui.label(format!("{:?}", backend.health));
                if preferences.show_backend_details {
                    if let Some(detail) = &backend.detail {
                        ui.label(RichText::new(detail).small().color(theme::text_secondary()));
                    }
                }
                if matches!(
                    backend.backend,
                    BackendKind::ForgeHxDsp
                        | BackendKind::LinuxStandard
                        | BackendKind::OpenRgb
                        | BackendKind::Ratbag
                ) && ui.small_button("Restart").clicked()
                {
                    action = Some(SettingsAction::RestartBackend(backend.backend));
                }
            });
        }
        if backends.is_empty() {
            ui.label("Backend status is not available yet.");
        }
    });

    ui.add_space(12.0);
    ui.group(|ui| {
        ui.heading("Hardware safety");
        ui.label("One backend owns each capability. ForgeHX Native wins, then Linux-standard interfaces, then compatibility backends.");
        ui.label(RichText::new("Unknown vendor HID writes and generic/raw firmware flashing are not exposed by IPC. Model-gated microphone firmware operations use typed commands only.").color(theme::text_secondary()));
    });

    ui.add_space(12.0);
    if ui
        .add_sized(
            [200.0, 34.0],
            egui::Button::new("Rescan devices & backends").fill(theme::accent()),
        )
        .clicked()
    {
        action = Some(SettingsAction::Rescan);
    }

    SettingsResponse {
        action,
        changed: *preferences != before,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_refresh_interval_is_bounded() {
        let settings = UiPreferences::default();
        assert_eq!(settings.inventory_refresh_seconds, 1);
        assert!(settings.automatic_inventory_refresh);
        assert!(settings.refresh_after_changes);
    }
}
