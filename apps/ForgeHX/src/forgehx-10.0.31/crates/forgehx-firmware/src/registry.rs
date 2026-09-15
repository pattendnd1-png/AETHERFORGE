use forgehx_core::{DeviceClass, DeviceInfo, FirmwareSupportLevel, VendorFamily};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UsbIdentity {
    pub vendor_id: u16,
    pub product_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareAdapterDescriptor {
    pub id: &'static str,
    pub model: &'static str,
    pub support_level: FirmwareSupportLevel,
    pub normal_ids: &'static [UsbIdentity],
    pub update_ids: &'static [UsbIdentity],
    pub exact_hardware_match: bool,
}

const SOLOCAST_2_IDS: &[UsbIdentity] = &[UsbIdentity {
    vendor_id: 0x03f0,
    product_id: 0x0fbf,
}];
const QUADCAST_S_IDS: &[UsbIdentity] = &[
    UsbIdentity {
        vendor_id: 0x0951,
        product_id: 0x171f,
    },
    UsbIdentity {
        vendor_id: 0x03f0,
        product_id: 0x0f8b,
    },
];
const QUADCAST_2S_IDS: &[UsbIdentity] = &[
    UsbIdentity {
        vendor_id: 0x03f0,
        product_id: 0x0d84,
    },
    UsbIdentity {
        vendor_id: 0x03f0,
        product_id: 0x02b5,
    },
];

pub struct FirmwareRegistry;

impl FirmwareRegistry {
    pub fn for_device(device: &DeviceInfo) -> Option<FirmwareAdapterDescriptor> {
        if device.device_class != DeviceClass::Microphone
            || device.vendor_family != VendorFamily::HyperX
        {
            return None;
        }

        let ids = device
            .interfaces
            .iter()
            .filter_map(|interface| {
                Some(UsbIdentity {
                    vendor_id: interface
                        .vendor_id
                        .or((device.vendor_id != 0).then_some(device.vendor_id))?,
                    product_id: interface
                        .product_id
                        .or((device.product_id != 0).then_some(device.product_id))?,
                })
            })
            .collect::<Vec<_>>();
        let top = (device.vendor_id != 0 && device.product_id != 0).then_some(UsbIdentity {
            vendor_id: device.vendor_id,
            product_id: device.product_id,
        });

        let exact = |known: &'static [UsbIdentity]| {
            top.map(|id| known.contains(&id)).unwrap_or(false)
                || ids.iter().any(|id| known.contains(id))
        };

        if exact(SOLOCAST_2_IDS) {
            return Some(FirmwareAdapterDescriptor {
                id: "hyperx-solocast-2-firmware",
                model: "HyperX SoloCast 2",
                support_level: FirmwareSupportLevel::InventoryOnly,
                normal_ids: SOLOCAST_2_IDS,
                update_ids: &[],
                exact_hardware_match: true,
            });
        }
        if exact(QUADCAST_S_IDS) {
            return Some(FirmwareAdapterDescriptor {
                id: "hyperx-quadcast-s-firmware",
                model: "HyperX QuadCast S",
                support_level: FirmwareSupportLevel::InventoryOnly,
                normal_ids: QUADCAST_S_IDS,
                update_ids: &[],
                exact_hardware_match: true,
            });
        }
        if exact(QUADCAST_2S_IDS) {
            return Some(FirmwareAdapterDescriptor {
                id: "hyperx-quadcast-2s-firmware",
                model: "HyperX QuadCast 2 S",
                support_level: FirmwareSupportLevel::InventoryOnly,
                normal_ids: QUADCAST_2S_IDS,
                update_ids: &[],
                exact_hardware_match: true,
            });
        }

        // Recognition by name is inventory-only. It never grants a firmware writer.
        let lower = device.name.to_ascii_lowercase();
        let model = if lower.contains("flipcast") {
            "HyperX FlipCast"
        } else if lower.contains("solocast 2") {
            "HyperX SoloCast 2"
        } else if lower.contains("solocast") {
            "HyperX SoloCast"
        } else if lower.contains("duocast") {
            "HyperX DuoCast"
        } else if lower.contains("quadcast 2 s") || lower.contains("quadcast 2s") {
            "HyperX QuadCast 2 S"
        } else if lower.contains("quadcast 2") {
            "HyperX QuadCast 2"
        } else if lower.contains("quadcast s") {
            "HyperX QuadCast S"
        } else {
            return None;
        };

        Some(FirmwareAdapterDescriptor {
            id: "hyperx-mic-inventory-only",
            model,
            support_level: FirmwareSupportLevel::InventoryOnly,
            normal_ids: &[],
            update_ids: &[],
            exact_hardware_match: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forgehx_core::{ConnectionKind, DeviceId, SupportLevel};

    fn mic(vendor_id: u16, product_id: u16, name: &str, vendor_family: VendorFamily) -> DeviceInfo {
        DeviceInfo {
            id: DeviceId("mic".into()),
            name: name.into(),
            manufacturer: Some("HyperX".into()),
            vendor_id,
            product_id,
            serial: None,
            vendor_family,
            device_class: DeviceClass::Microphone,
            support_level: SupportLevel::GenericControls,
            connection_kind: ConnectionKind::Usb,
            interfaces: vec![],
            capabilities: vec![],
            generic_capabilities: vec![],
            capability_owners: vec![],
            battery_percent: None,
            battery_state: None,
            protocol: None,
        }
    }

    #[test]
    fn exact_quadcast_s_matches_inventory_adapter() {
        let adapter = FirmwareRegistry::for_device(&mic(
            0x0951,
            0x171f,
            "HyperX QuadCast S",
            VendorFamily::HyperX,
        ))
        .unwrap();
        assert!(adapter.exact_hardware_match);
        assert!(!adapter.support_level.can_update());
    }

    #[test]
    fn exact_solocast_2_matches_inventory_adapter() {
        let adapter = FirmwareRegistry::for_device(&mic(
            0x03f0,
            0x0fbf,
            "HyperX SoloCast 2",
            VendorFamily::HyperX,
        ))
        .unwrap();
        assert_eq!(adapter.id, "hyperx-solocast-2-firmware");
        assert!(adapter.exact_hardware_match);
        assert!(!adapter.support_level.can_update());
    }

    #[test]
    fn unknown_hp_vid_never_matches_by_vendor_id_alone() {
        assert!(FirmwareRegistry::for_device(&mic(
            0x03f0,
            0xffff,
            "HP USB Microphone",
            VendorFamily::Hp
        ))
        .is_none());
    }

    #[test]
    fn name_only_hyperx_recognition_remains_inventory_only() {
        let adapter = FirmwareRegistry::for_device(&mic(
            0,
            0,
            "HyperX SoloCast Microphone",
            VendorFamily::HyperX,
        ))
        .unwrap();
        assert!(!adapter.exact_hardware_match);
        assert_eq!(adapter.support_level, FirmwareSupportLevel::InventoryOnly);
    }
}
