//! v0.1.21 DragonGlass UI/UX additions for the read-only On Device startup profile.

use crate::on_device::{OnDeviceSnapshot, SnapshotOrigin};
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum OnDeviceUiState {
    #[default]
    Idle,
    Reading,
    Loaded {
        origin: SnapshotOrigin,
        serial: String,
        firmware: String,
        imported: usize,
        failed: usize,
    },
    Unavailable(String),
    LocalEdit,
    LocalProfile,
}

impl OnDeviceUiState {
    pub fn from_snapshot(snapshot: &OnDeviceSnapshot) -> Self {
        Self::Loaded {
            origin: snapshot.origin,
            serial: snapshot.serial.clone(),
            firmware: snapshot.firmware.clone(),
            imported: snapshot.imported_messages,
            failed: snapshot.failed_messages,
        }
    }

    pub const fn short_label(&self) -> &'static str {
        match self {
            Self::Idle => "ON DEVICE IDLE",
            Self::Reading => "READING MIC MEMORY",
            Self::Loaded {
                origin: SnapshotOrigin::Device,
                ..
            } => "ON DEVICE ACTIVE",
            Self::Loaded {
                origin: SnapshotOrigin::Cache,
                ..
            } => "ON DEVICE CACHED",
            Self::Unavailable(_) => "ON DEVICE UNAVAILABLE",
            Self::LocalEdit => "LOCAL EDIT",
            Self::LocalProfile => "LOCAL PROFILE",
        }
    }

    pub const fn is_device_loaded(&self) -> bool {
        matches!(
            self,
            Self::Loaded {
                origin: SnapshotOrigin::Device,
                ..
            }
        )
    }

    pub const fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded { .. })
    }

    pub const fn is_reading(&self) -> bool {
        matches!(self, Self::Reading)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OnDeviceUiAction {
    pub reload: bool,
}

pub fn profile_memory_strip(ui: &mut egui::Ui, state: &OnDeviceUiState) -> OnDeviceUiAction {
    let mut action = OnDeviceUiAction::default();
    let positive = state.is_loaded();
    let stroke = if positive {
        Color32::from_rgba_unmultiplied(116, 99, 255, 110)
    } else {
        Color32::from_rgba_unmultiplied(115, 124, 148, 55)
    };

    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(13, 16, 31, 232))
        .stroke(Stroke::new(1.0, stroke))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(10, 7))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("MIC MEMORY")
                        .size(9.5)
                        .strong()
                        .color(Color32::from_rgb(183, 174, 229)),
                );
                ui.label(
                    RichText::new(state.short_label())
                        .size(10.0)
                        .strong()
                        .color(if positive {
                            Color32::from_rgb(206, 197, 255)
                        } else {
                            Color32::from_rgb(171, 178, 199)
                        }),
                );

                match state {
                    OnDeviceUiState::Loaded {
                        origin,
                        serial,
                        firmware,
                        imported,
                        failed,
                    } => {
                        let source = match origin {
                            SnapshotOrigin::Device => "Mic",
                            SnapshotOrigin::Cache => "Cache fallback",
                        };
                        ui.label(
                            RichText::new(format!(
                                "{source} · FW {firmware} · {imported} imported / {failed} unavailable"
                            ))
                            .small()
                            .color(Color32::from_rgb(166, 173, 197)),
                        );
                        if !serial.is_empty() {
                            ui.label(
                                RichText::new(format!("S/N {serial}"))
                                    .small()
                                    .color(Color32::from_rgb(133, 146, 177)),
                            );
                        }
                    }
                    OnDeviceUiState::Unavailable(error) => {
                        ui.label(
                            RichText::new(error)
                                .small()
                                .color(Color32::from_rgb(181, 169, 190)),
                        );
                    }
                    OnDeviceUiState::LocalEdit => {
                        ui.label(
                            RichText::new("Mic memory is unchanged; edits are saved locally.")
                                .small()
                                .color(Color32::from_rgb(166, 173, 197)),
                        );
                    }
                    OnDeviceUiState::LocalProfile => {
                        ui.label(
                            RichText::new("Local profile active; mic memory remains unchanged.")
                                .small()
                                .color(Color32::from_rgb(166, 173, 197)),
                        );
                    }
                    OnDeviceUiState::Reading => {
                        ui.spinner();
                        ui.label(
                            RichText::new("Read-only startup scan")
                                .small()
                                .color(Color32::from_rgb(166, 173, 197)),
                        );
                    }
                    OnDeviceUiState::Idle => {}
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    action.reload = ui
                        .add_enabled(!state.is_reading(), egui::Button::new(RichText::new("RELOAD MIC").small()))
                        .on_hover_text("Re-read the profile stored on the BEACN Mic")
                        .clicked();
                });
            });
        });

    action
}
