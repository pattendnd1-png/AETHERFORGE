use forgehx_core::{Capability, DeviceClass, DeviceInfo, VendorFamily};

pub const SOLOCAST_2_VENDOR_ID: u16 = 0x03f0;
pub const SOLOCAST_2_PRODUCT_ID: u16 = 0x0fbf;
pub const SOLOCAST_2_SUPPORT_ID: &str = "hyperx-solocast-2-forgehx-v1";

#[derive(Debug, Clone, Copy)]
pub struct MicrophoneSupportContract {
    pub id: &'static str,
    pub name: &'static str,
    pub vendor_id: u16,
    pub product_id: u16,
    pub required_capabilities: &'static [Capability],
    /// `complete` means ForgeHX has a verified owner for every capability in the
    /// application support contract. It does not grant undocumented raw-HID write authority.
    pub complete: bool,
}

const SOLOCAST_2_REQUIRED: &[Capability] = &[
    Capability::Audio,
    Capability::Microphone,
    Capability::Volume,
    Capability::Mute,
    Capability::MicDsp,
    Capability::MicFirmwareInventory,
];

pub const MICROPHONE_SUPPORT_CONTRACTS: &[MicrophoneSupportContract] =
    &[MicrophoneSupportContract {
        id: SOLOCAST_2_SUPPORT_ID,
        name: "HyperX SoloCast 2",
        vendor_id: SOLOCAST_2_VENDOR_ID,
        product_id: SOLOCAST_2_PRODUCT_ID,
        required_capabilities: SOLOCAST_2_REQUIRED,
        complete: true,
    }];

pub fn support_contract_for(device: &DeviceInfo) -> Option<&'static MicrophoneSupportContract> {
    if device.vendor_family != VendorFamily::HyperX
        || device.device_class != DeviceClass::Microphone
    {
        return None;
    }
    MICROPHONE_SUPPORT_CONTRACTS.iter().find(|contract| {
        contract.vendor_id == device.vendor_id && contract.product_id == device.product_id
    })
}

pub fn complete_microphone_support(device: &DeviceInfo) -> bool {
    let Some(contract) = support_contract_for(device) else {
        return false;
    };
    contract.complete
        && contract.required_capabilities.iter().all(|capability| {
            device
                .selected_owner(*capability)
                .is_some_and(|owner| owner.verified)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use forgehx_core::{BackendKind, CapabilityOwner, ConnectionKind, DeviceId, SupportLevel};

    fn solocast() -> DeviceInfo {
        let required = SOLOCAST_2_REQUIRED
            .iter()
            .map(|capability| CapabilityOwner {
                capability: *capability,
                backend: match capability {
                    Capability::MicDsp => BackendKind::ForgeHxDsp,
                    Capability::MicFirmwareInventory => BackendKind::ForgeHxNative,
                    _ => BackendKind::LinuxStandard,
                },
                backend_device_id: Some("verified".into()),
                verified: true,
                writable: *capability != Capability::MicFirmwareInventory,
                detail: None,
            })
            .collect();
        DeviceInfo {
            id: DeviceId("03f0:0fbf:test".into()),
            name: "HyperX SoloCast 2".into(),
            manufacturer: Some("HP, Inc".into()),
            vendor_id: SOLOCAST_2_VENDOR_ID,
            product_id: SOLOCAST_2_PRODUCT_ID,
            serial: None,
            vendor_family: VendorFamily::HyperX,
            device_class: DeviceClass::Microphone,
            support_level: SupportLevel::PartiallySupported,
            connection_kind: ConnectionKind::Composite,
            interfaces: vec![],
            capabilities: vec![],
            generic_capabilities: vec![],
            capability_owners: required,
            battery_percent: None,
            battery_state: None,
            protocol: None,
        }
    }

    #[test]
    fn exact_solocast_2_contract_requires_verified_composite_stack() {
        let mut device = solocast();
        assert!(complete_microphone_support(&device));
        device
            .capability_owners
            .retain(|owner| owner.capability != Capability::MicDsp);
        assert!(!complete_microphone_support(&device));
    }
}
