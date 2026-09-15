use crate::theme;
use eframe::egui::{self, RichText};
use forgehx_core::{FirmwareIdentity, FirmwarePackageInfo, FirmwareValidation};

#[derive(Debug, Clone)]
pub enum FirmwareControl {
    Stage(String),
    Validate(String),
    Begin(String),
    Forget(String),
    Refresh,
}

pub fn show(
    ui: &mut egui::Ui,
    identity: Option<&FirmwareIdentity>,
    package: Option<&FirmwarePackageInfo>,
    path: &mut String,
) -> Option<FirmwareControl> {
    ui.heading("Firmware");
    ui.label(
        RichText::new("HyperX microphone firmware inventory, validation, and model-gated updates.")
            .color(theme::text_secondary()),
    );
    ui.add_space(8.0);

    let Some(identity) = identity else {
        ui.label("No HyperX microphone firmware adapter is available for this device.");
        return None;
    };

    ui.group(|ui| {
        ui.label(RichText::new(&identity.model).strong());
        ui.label(format!(
            "Installed firmware: {}",
            identity
                .firmware_version
                .as_deref()
                .unwrap_or("Not exposed")
        ));
        ui.label(format!(
            "Hardware revision: {}",
            identity
                .hardware_revision
                .as_deref()
                .unwrap_or("Not exposed")
        ));
        ui.label(format!("Support: {:?}", identity.support_level));
        if !identity.support_level.can_update() {
            ui.label(
                RichText::new(
                    "Firmware inventory only — no verified model-specific writer is enabled.",
                )
                .color(theme::status_warn()),
            );
        }
    });

    ui.add_space(10.0);
    ui.label("Firmware package path");
    ui.text_edit_singleline(path);
    let mut action = None;
    ui.horizontal(|ui| {
        if ui.button("Stage + Validate").clicked() && !path.trim().is_empty() {
            action = Some(FirmwareControl::Stage(path.trim().to_owned()));
        }
        if ui.button("Refresh Inventory").clicked() {
            action = Some(FirmwareControl::Refresh);
        }
    });

    if let Some(package) = package {
        ui.add_space(10.0);
        ui.group(|ui| {
            ui.label(RichText::new("Staged firmware").strong());
            ui.label(format!("File: {}", package.original_filename));
            ui.label(format!("SHA-256: {}", package.sha256));
            ui.label(format!("Size: {} bytes", package.size_bytes));
            ui.label(format!("Format: {}", package.parsed_format));
            ui.label(format!("Compatibility: {:?}", package.validation));
            ui.label(format!(
                "Vendor signature verified: {}",
                package.vendor_signature_verified
            ));
        });
        ui.horizontal(|ui| {
            if ui.button("Validate Again").clicked() {
                action = Some(FirmwareControl::Validate(package.staged_id.clone()));
            }
            let enabled = identity.support_level.can_update()
                && package.validation == FirmwareValidation::Valid;
            if ui
                .add_enabled(enabled, egui::Button::new("Update Firmware"))
                .clicked()
            {
                action = Some(FirmwareControl::Begin(package.staged_id.clone()));
            }
            if ui.button("Forget Package").clicked() {
                action = Some(FirmwareControl::Forget(package.staged_id.clone()));
            }
        });
    }
    action
}
