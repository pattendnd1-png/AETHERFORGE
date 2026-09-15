use forgehx_core::{Capability, MouseLimits, MouseModelInfo, MouseProtocolFamily};

pub const HASTE_V1_DRIVER_ID: &str = "hyperx-pulsefire-haste-wireless-v1";
pub const SAGA_PRO_DRIVER_ID: &str = "hyperx-pulsefire-saga-pro-v1";

#[derive(Debug, Clone)]
pub struct MouseModelDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub protocol_family: MouseProtocolFamily,
    pub exact_ids: &'static [(u16, u16)],
    pub native_driver: Option<&'static str>,
    pub native_capabilities: &'static [Capability],
    pub complete: bool,
}

#[derive(Debug, Clone)]
pub struct MouseModelMatch {
    pub definition: &'static MouseModelDefinition,
    pub exact_hardware_match: bool,
    pub limits: MouseLimits,
}

const NONE: &[Capability] = &[];
const HASTE_NATIVE: &[Capability] = &[
    Capability::Lighting,
    Capability::Dpi,
    Capability::PollingRate,
    Capability::Profiles,
    Capability::Bindings,
    Capability::MacroAssignments,
    Capability::BatteryStatus,
];
const SAGA_NATIVE: &[Capability] = &[Capability::Lighting, Capability::BatteryStatus];

// Verified USB identities from OpenRGB, Chromium device metadata, the Haste protocol project,
// and the Saga Pro Linux protocol project. Alias-only entries remain write-protected natively.
const SURGE_IDS: &[(u16, u16)] = &[(0x0951, 0x16d3), (0x03f0, 0x0490)];
const FPS_PRO_IDS: &[(u16, u16)] = &[(0x0951, 0x16d7)];
const CORE_IDS: &[(u16, u16)] = &[(0x0951, 0x16de), (0x03f0, 0x0d8f)];
const DART_IDS: &[(u16, u16)] = &[
    (0x0951, 0x16e1),
    (0x03f0, 0x068e),
    (0x0951, 0x16e2),
    (0x03f0, 0x088e),
];
const RAID_IDS: &[(u16, u16)] = &[(0x0951, 0x16e4)];
const HASTE_WIRED_IDS: &[(u16, u16)] = &[(0x0951, 0x1727), (0x03f0, 0x0f8f)];
const HASTE_WIRELESS_IDS: &[(u16, u16)] = &[(0x03f0, 0x028e), (0x03f0, 0x048e)];
const HASTE2_WIRED_IDS: &[(u16, u16)] = &[(0x03f0, 0x0b97)];
const HASTE2_WIRELESS_IDS: &[(u16, u16)] = &[(0x03f0, 0x0f98), (0x03f0, 0x0b97)];
const SAGA_PRO_IDS: &[(u16, u16)] = &[(0x03f0, 0x04bf), (0x03f0, 0x06bf)];

pub const MOUSE_MODELS: &[MouseModelDefinition] = &[
    MouseModelDefinition {
        id: "pulsefire-surge",
        name: "HyperX Pulsefire Surge",
        aliases: &["Pulsefire Surge", "HyperX Pulsefire Surge Gaming Mouse"],
        protocol_family: MouseProtocolFamily::LegacyPulsefire,
        exact_ids: SURGE_IDS,
        native_driver: None,
        native_capabilities: NONE,
        complete: false,
    },
    MouseModelDefinition {
        id: "pulsefire-raid",
        name: "HyperX Pulsefire Raid",
        aliases: &["Pulsefire Raid"],
        protocol_family: MouseProtocolFamily::LegacyPulsefire,
        exact_ids: RAID_IDS,
        native_driver: None,
        native_capabilities: NONE,
        complete: false,
    },
    MouseModelDefinition {
        id: "pulsefire-core",
        name: "HyperX Pulsefire Core",
        aliases: &["Pulsefire Core"],
        protocol_family: MouseProtocolFamily::LegacyPulsefire,
        exact_ids: CORE_IDS,
        native_driver: None,
        native_capabilities: NONE,
        complete: false,
    },
    MouseModelDefinition {
        id: "pulsefire-fps-pro",
        name: "HyperX Pulsefire FPS Pro",
        aliases: &["Pulsefire FPS Pro"],
        protocol_family: MouseProtocolFamily::LegacyPulsefire,
        exact_ids: FPS_PRO_IDS,
        native_driver: None,
        native_capabilities: NONE,
        complete: false,
    },
    MouseModelDefinition {
        id: "pulsefire-dart",
        name: "HyperX Pulsefire Dart",
        aliases: &["Pulsefire Dart"],
        protocol_family: MouseProtocolFamily::Dart,
        exact_ids: DART_IDS,
        native_driver: None,
        native_capabilities: NONE,
        complete: false,
    },
    MouseModelDefinition {
        id: "pulsefire-haste",
        name: "HyperX Pulsefire Haste",
        aliases: &["Pulsefire Haste", "HyperX Pulsefire Haste Gaming Mouse"],
        protocol_family: MouseProtocolFamily::HasteV1,
        exact_ids: HASTE_WIRED_IDS,
        native_driver: None,
        native_capabilities: NONE,
        complete: false,
    },
    MouseModelDefinition {
        id: "pulsefire-haste-wireless",
        name: "HyperX Pulsefire Haste Wireless",
        aliases: &["Pulsefire Haste Wireless"],
        protocol_family: MouseProtocolFamily::HasteV1,
        exact_ids: HASTE_WIRELESS_IDS,
        native_driver: Some(HASTE_V1_DRIVER_ID),
        native_capabilities: HASTE_NATIVE,
        complete: true,
    },
    MouseModelDefinition {
        id: "pulsefire-haste-2",
        name: "HyperX Pulsefire Haste 2",
        aliases: &[
            "Pulsefire Haste 2",
            "Pulsefire Haste 2 Wired",
            "Pulsefire Haste 2 - wired",
            "HyperX Pulsefire Haste 2 Gaming Mouse",
        ],
        protocol_family: MouseProtocolFamily::Haste2,
        exact_ids: HASTE2_WIRED_IDS,
        native_driver: None,
        native_capabilities: NONE,
        complete: false,
    },
    MouseModelDefinition {
        id: "pulsefire-haste-2-wireless",
        name: "HyperX Pulsefire Haste 2 Wireless",
        aliases: &["Pulsefire Haste 2 Wireless"],
        protocol_family: MouseProtocolFamily::Haste2,
        exact_ids: HASTE2_WIRELESS_IDS,
        native_driver: None,
        native_capabilities: NONE,
        complete: false,
    },
    MouseModelDefinition {
        id: "pulsefire-haste-2-mini-wireless",
        name: "HyperX Pulsefire Haste 2 Mini Wireless",
        aliases: &["Pulsefire Haste 2 Mini Wireless"],
        protocol_family: MouseProtocolFamily::Haste2,
        exact_ids: &[],
        native_driver: None,
        native_capabilities: NONE,
        complete: false,
    },
    MouseModelDefinition {
        id: "pulsefire-haste-2-core-wireless",
        name: "HyperX Pulsefire Haste 2 Core Wireless",
        aliases: &["Pulsefire Haste 2 Core Wireless"],
        protocol_family: MouseProtocolFamily::Haste2,
        exact_ids: &[],
        native_driver: None,
        native_capabilities: NONE,
        complete: false,
    },
    MouseModelDefinition {
        id: "pulsefire-haste-2-s-wireless",
        name: "HyperX Pulsefire Haste 2 S Wireless",
        aliases: &["Pulsefire Haste 2 S Wireless", "Haste 2 S"],
        protocol_family: MouseProtocolFamily::Haste2,
        exact_ids: &[],
        native_driver: None,
        native_capabilities: NONE,
        complete: false,
    },
    MouseModelDefinition {
        id: "pulsefire-fuse-wireless",
        name: "HyperX Pulsefire Fuse Wireless",
        aliases: &["Pulsefire Fuse Wireless", "Pulsefire Fuse"],
        protocol_family: MouseProtocolFamily::Fuse,
        exact_ids: &[],
        native_driver: None,
        native_capabilities: NONE,
        complete: false,
    },
    MouseModelDefinition {
        id: "pulsefire-haste-2-pro",
        name: "HyperX Pulsefire Haste 2 Pro",
        aliases: &[
            "Pulsefire Haste 2 Pro",
            "Pulsefire Haste 2 Pro 4K Wireless",
            "Pulsefire Haste 2 Pro - 4K Wireless",
        ],
        protocol_family: MouseProtocolFamily::Haste2,
        exact_ids: &[],
        native_driver: None,
        native_capabilities: NONE,
        complete: false,
    },
    MouseModelDefinition {
        id: "pulsefire-saga",
        name: "HyperX Pulsefire Saga",
        aliases: &["Pulsefire Saga"],
        protocol_family: MouseProtocolFamily::Saga,
        exact_ids: &[],
        native_driver: None,
        native_capabilities: NONE,
        complete: false,
    },
    MouseModelDefinition {
        id: "pulsefire-saga-pro",
        name: "HyperX Pulsefire Saga Pro Wireless",
        aliases: &[
            "Pulsefire Saga Pro Wireless",
            "Pulsefire Saga Pro",
            "HyperX Pulsefire Saga Pro",
        ],
        protocol_family: MouseProtocolFamily::Saga,
        exact_ids: SAGA_PRO_IDS,
        native_driver: Some(SAGA_PRO_DRIVER_ID),
        native_capabilities: SAGA_NATIVE,
        complete: false,
    },
];

pub fn model_for(vendor_id: u16, product_id: u16, name: &str) -> Option<MouseModelMatch> {
    let normalized = normalize(name);
    let by_name = MOUSE_MODELS
        .iter()
        .filter_map(|model| name_match_score(model, &normalized).map(|score| (score, model)))
        .max_by_key(|(score, _)| *score)
        .map(|(_, model)| model);
    let definition = by_name.or_else(|| {
        MOUSE_MODELS
            .iter()
            .find(|model| model.exact_ids.contains(&(vendor_id, product_id)))
    })?;
    let exact_hardware_match = definition.exact_ids.contains(&(vendor_id, product_id));
    Some(MouseModelMatch {
        definition,
        exact_hardware_match,
        limits: limits_for(definition.id, product_id),
    })
}

fn name_match_score(model: &MouseModelDefinition, normalized_name: &str) -> Option<(u8, usize)> {
    std::iter::once(model.name)
        .chain(model.aliases.iter().copied())
        .filter_map(|candidate| {
            let candidate = normalize(candidate);
            if candidate.is_empty() {
                return None;
            }
            if normalized_name == candidate {
                Some((2, candidate.len()))
            } else if normalized_name.contains(&candidate) {
                Some((1, candidate.len()))
            } else {
                None
            }
        })
        .max()
}

pub fn native_driver_for(vendor_id: u16, product_id: u16, name: &str) -> Option<&'static str> {
    let matched = model_for(vendor_id, product_id, name)?;
    matched
        .exact_hardware_match
        .then_some(matched.definition.native_driver)
        .flatten()
}

impl MouseModelMatch {
    pub fn info(&self) -> MouseModelInfo {
        MouseModelInfo {
            id: self.definition.id.into(),
            name: self.definition.name.into(),
            protocol_family: self.definition.protocol_family,
            limits: self.limits.clone(),
            exact_hardware_match: self.exact_hardware_match,
            native_driver: self.definition.native_driver.map(str::to_owned),
            complete: self.definition.complete,
        }
    }
}

fn limits_for(id: &str, product_id: u16) -> MouseLimits {
    match id {
        // 0x028e is the 2.4 GHz receiver and 0x048e is the same wireless-capable mouse
        // connected over USB. Battery/charging telemetry remains meaningful on both transports.
        "pulsefire-haste-wireless" => MouseLimits {
            dpi_min: Some(200),
            dpi_max: Some(16000),
            dpi_step: Some(100),
            max_dpi_stages: Some(5),
            polling_rates_hz: vec![125, 250, 500, 1000],
            button_count: Some(6),
            onboard_profiles: Some(1),
            wireless: true,
            battery: true,
            lift_off_distances_mm: vec![1, 2],
        },
        "pulsefire-haste" => MouseLimits {
            dpi_min: Some(200),
            dpi_max: Some(16000),
            dpi_step: Some(100),
            max_dpi_stages: Some(5),
            polling_rates_hz: vec![125, 250, 500, 1000],
            button_count: Some(6),
            onboard_profiles: Some(1),
            wireless: false,
            battery: false,
            lift_off_distances_mm: vec![1, 2],
        },
        "pulsefire-haste-2" => MouseLimits {
            dpi_min: None,
            dpi_max: Some(26000),
            dpi_step: None,
            max_dpi_stages: Some(4),
            polling_rates_hz: vec![],
            button_count: Some(6),
            onboard_profiles: Some(1),
            wireless: false,
            battery: false,
            lift_off_distances_mm: vec![],
        },
        "pulsefire-haste-2-wireless" => MouseLimits {
            dpi_min: None,
            dpi_max: Some(26000),
            dpi_step: None,
            max_dpi_stages: Some(4),
            polling_rates_hz: vec![125, 250, 500, 1000],
            button_count: Some(6),
            onboard_profiles: Some(1),
            wireless: true,
            battery: true,
            lift_off_distances_mm: vec![],
        },
        "pulsefire-haste-2-s-wireless" => MouseLimits {
            dpi_min: None,
            dpi_max: Some(26000),
            dpi_step: None,
            max_dpi_stages: Some(4),
            polling_rates_hz: vec![125, 250, 500, 1000],
            button_count: Some(6),
            onboard_profiles: Some(1),
            wireless: true,
            battery: true,
            lift_off_distances_mm: vec![],
        },
        "pulsefire-fuse-wireless" => MouseLimits {
            dpi_min: None,
            dpi_max: Some(12000),
            dpi_step: None,
            max_dpi_stages: Some(4),
            polling_rates_hz: vec![125, 250, 500, 1000],
            button_count: Some(6),
            onboard_profiles: Some(1),
            wireless: true,
            battery: true,
            lift_off_distances_mm: vec![],
        },
        "pulsefire-saga-pro" => MouseLimits {
            dpi_min: Some(50),
            dpi_max: Some(26000),
            dpi_step: Some(50),
            max_dpi_stages: Some(4),
            polling_rates_hz: if product_id == 0x06bf {
                vec![125, 250, 500, 1000, 2000, 4000]
            } else {
                vec![125, 250, 500, 1000]
            },
            button_count: None,
            onboard_profiles: Some(1),
            wireless: product_id == 0x06bf,
            battery: product_id == 0x06bf,
            lift_off_distances_mm: vec![],
        },
        _ => MouseLimits::default(),
    }
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_seeded_model_is_unique() {
        let mut ids = MOUSE_MODELS
            .iter()
            .map(|model| model.id)
            .collect::<Vec<_>>();
        let len = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), len);
    }

    #[test]
    fn haste_wireless_exact_match_enables_native_driver() {
        let matched = model_for(0x03f0, 0x028e, "HyperX Pulsefire Haste Wireless").unwrap();
        assert!(matched.exact_hardware_match);
        assert_eq!(
            native_driver_for(0x03f0, 0x028e, "HyperX Pulsefire Haste Wireless"),
            Some(HASTE_V1_DRIVER_ID)
        );
    }

    #[test]
    fn name_only_future_variant_never_enables_native_driver() {
        let matched = model_for(0x03f0, 0xffff, "HyperX Pulsefire Haste 2 S Wireless").unwrap();
        assert!(!matched.exact_hardware_match);
        assert_eq!(
            native_driver_for(0x03f0, 0xffff, "HyperX Pulsefire Haste 2 S Wireless"),
            None
        );
    }

    #[test]
    fn name_only_matching_prefers_the_most_specific_haste_variant() {
        for (name, expected_id) in [
            (
                "HyperX Pulsefire Haste 2 Mini Wireless",
                "pulsefire-haste-2-mini-wireless",
            ),
            (
                "HyperX Pulsefire Haste 2 Core Wireless",
                "pulsefire-haste-2-core-wireless",
            ),
            (
                "HyperX Pulsefire Haste 2 S Wireless",
                "pulsefire-haste-2-s-wireless",
            ),
            (
                "HyperX Pulsefire Haste 2 Pro 4K Wireless",
                "pulsefire-haste-2-pro",
            ),
        ] {
            let matched = model_for(0x03f0, 0xffff, name).unwrap();
            assert_eq!(matched.definition.id, expected_id, "wrong model for {name}");
            assert!(!matched.exact_hardware_match);
            assert_eq!(native_driver_for(0x03f0, 0xffff, name), None);
        }
    }
}
