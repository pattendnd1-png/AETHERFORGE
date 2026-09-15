use crate::{registry::FirmwareAdapterDescriptor, StagedFirmware};
use forgehx_core::{FirmwarePackageInfo, FirmwareValidation};

pub fn validate_package(
    adapter: &FirmwareAdapterDescriptor,
    staged: &StagedFirmware,
) -> FirmwarePackageInfo {
    // Until a model-specific container parser is verified, staging is truthful inventory only.
    FirmwarePackageInfo {
        staged_id: staged.staged_id.clone(),
        original_filename: staged.original_filename.clone(),
        sha256: staged.sha256.clone(),
        size_bytes: staged.size_bytes,
        parsed_format: "opaque".into(),
        claimed_model: adapter
            .exact_hardware_match
            .then(|| adapter.model.to_owned()),
        claimed_hardware_revision: None,
        claimed_firmware_version: None,
        validation: FirmwareValidation::VersionUnknown,
        vendor_signature_verified: false,
    }
}
