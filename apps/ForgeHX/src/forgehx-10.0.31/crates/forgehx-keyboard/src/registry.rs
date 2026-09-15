#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardProtocolFamily {
    AlloyLegacy,
    AlloyOrigins,
    AlloyRise,
    Origins2,
    Eve,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardLightingTopology {
    None,
    Zone(u16),
    PerKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardLimits {
    pub lighting: KeyboardLightingTopology,
    pub polling_rates_hz: Vec<u16>,
    pub onboard_profiles: Option<u8>,
    pub wireless: bool,
    pub battery: bool,
    pub hall_effect: bool,
    pub rapid_trigger: bool,
}

impl Default for KeyboardLimits {
    fn default() -> Self {
        Self {
            lighting: KeyboardLightingTopology::None,
            polling_rates_hz: Vec::new(),
            onboard_profiles: None,
            wireless: false,
            battery: false,
            hall_effect: false,
            rapid_trigger: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyboardModelDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub protocol_family: KeyboardProtocolFamily,
    pub exact_ids: &'static [(u16, u16)],
    pub native_driver: Option<&'static str>,
    pub complete: bool,
}

#[derive(Debug, Clone)]
pub struct KeyboardModelMatch {
    pub definition: &'static KeyboardModelDefinition,
    pub exact_hardware_match: bool,
    pub limits: KeyboardLimits,
}

const ALLOY_ORIGINS_IDS: &[(u16, u16)] = &[(0x0951, 0x16e5), (0x03f0, 0x0591)];
const ALLOY_ORIGINS_CORE_IDS: &[(u16, u16)] = &[(0x0951, 0x16e6), (0x03f0, 0x098f)];
const ALLOY_ORIGINS_60_IDS: &[(u16, u16)] = &[(0x0951, 0x1734), (0x03f0, 0x0c8e)];
const ALLOY_ORIGINS_65_IDS: &[(u16, u16)] = &[(0x03f0, 0x038f)];
const ALLOY_ELITE_RGB_IDS: &[(u16, u16)] = &[(0x0951, 0x16be)];
const ALLOY_ELITE_2_IDS: &[(u16, u16)] = &[(0x0951, 0x1711), (0x03f0, 0x058f)];
const ALLOY_FPS_RGB_IDS: &[(u16, u16)] = &[(0x0951, 0x16dc)];

pub const KEYBOARD_MODELS: &[KeyboardModelDefinition] = &[
    KeyboardModelDefinition {
        id: "alloy-rise-75-wireless",
        name: "HyperX Alloy Rise 75 Wireless",
        aliases: &["Alloy Rise 75 Wireless"],
        protocol_family: KeyboardProtocolFamily::AlloyRise,
        exact_ids: &[],
        native_driver: None,
        complete: false,
    },
    KeyboardModelDefinition {
        id: "alloy-rise-75",
        name: "HyperX Alloy Rise 75",
        aliases: &["Alloy Rise 75"],
        protocol_family: KeyboardProtocolFamily::AlloyRise,
        exact_ids: &[],
        native_driver: None,
        complete: false,
    },
    KeyboardModelDefinition {
        id: "alloy-rise",
        name: "HyperX Alloy Rise",
        aliases: &["Alloy Rise"],
        protocol_family: KeyboardProtocolFamily::AlloyRise,
        exact_ids: &[],
        native_driver: None,
        complete: false,
    },
    KeyboardModelDefinition {
        id: "origins-2-pro-65",
        name: "HyperX Origins 2 Pro 65",
        aliases: &["Origins 2 Pro 65"],
        protocol_family: KeyboardProtocolFamily::Origins2,
        exact_ids: &[],
        native_driver: None,
        complete: false,
    },
    KeyboardModelDefinition {
        id: "origins-2-65",
        name: "HyperX Origins 2 65",
        aliases: &["Origins 2 65"],
        protocol_family: KeyboardProtocolFamily::Origins2,
        exact_ids: &[],
        native_driver: None,
        complete: false,
    },
    KeyboardModelDefinition {
        id: "origins-2-1800",
        name: "HyperX Origins 2 1800",
        aliases: &["Origins 2 1800"],
        protocol_family: KeyboardProtocolFamily::Origins2,
        exact_ids: &[],
        native_driver: None,
        complete: false,
    },
    KeyboardModelDefinition {
        id: "eve-1800",
        name: "HyperX Eve 1800",
        aliases: &["Eve 1800"],
        protocol_family: KeyboardProtocolFamily::Eve,
        exact_ids: &[],
        native_driver: None,
        complete: false,
    },
    KeyboardModelDefinition {
        id: "alloy-elite-rgb",
        name: "HyperX Alloy Elite RGB",
        aliases: &["Alloy Elite RGB"],
        protocol_family: KeyboardProtocolFamily::AlloyLegacy,
        exact_ids: ALLOY_ELITE_RGB_IDS,
        native_driver: None,
        complete: false,
    },
    KeyboardModelDefinition {
        id: "alloy-fps-rgb",
        name: "HyperX Alloy FPS RGB",
        aliases: &["Alloy FPS RGB"],
        protocol_family: KeyboardProtocolFamily::AlloyLegacy,
        exact_ids: ALLOY_FPS_RGB_IDS,
        native_driver: None,
        complete: false,
    },
    KeyboardModelDefinition {
        id: "alloy-origins-core",
        name: "HyperX Alloy Origins Core",
        aliases: &["Alloy Origins Core"],
        protocol_family: KeyboardProtocolFamily::AlloyOrigins,
        exact_ids: ALLOY_ORIGINS_CORE_IDS,
        native_driver: None,
        complete: false,
    },
    KeyboardModelDefinition {
        id: "alloy-origins-60",
        name: "HyperX Alloy Origins 60",
        aliases: &["Alloy Origins 60"],
        protocol_family: KeyboardProtocolFamily::AlloyOrigins,
        exact_ids: ALLOY_ORIGINS_60_IDS,
        native_driver: None,
        complete: false,
    },
    KeyboardModelDefinition {
        id: "alloy-origins-65",
        name: "HyperX Alloy Origins 65",
        aliases: &["Alloy Origins 65"],
        protocol_family: KeyboardProtocolFamily::AlloyOrigins,
        exact_ids: ALLOY_ORIGINS_65_IDS,
        native_driver: None,
        complete: false,
    },
    KeyboardModelDefinition {
        id: "alloy-origins",
        name: "HyperX Alloy Origins",
        aliases: &["Alloy Origins"],
        protocol_family: KeyboardProtocolFamily::AlloyOrigins,
        exact_ids: ALLOY_ORIGINS_IDS,
        native_driver: None,
        complete: false,
    },
    KeyboardModelDefinition {
        id: "alloy-elite-2",
        name: "HyperX Alloy Elite 2",
        aliases: &["Alloy Elite 2"],
        protocol_family: KeyboardProtocolFamily::AlloyLegacy,
        exact_ids: ALLOY_ELITE_2_IDS,
        native_driver: None,
        complete: false,
    },
    KeyboardModelDefinition {
        id: "alloy-mkw100",
        name: "HyperX Alloy MKW100",
        aliases: &["Alloy MKW100", "MKW100"],
        protocol_family: KeyboardProtocolFamily::AlloyLegacy,
        exact_ids: &[],
        native_driver: None,
        complete: false,
    },
];

pub fn model_for(vendor_id: u16, product_id: u16, name: &str) -> Option<KeyboardModelMatch> {
    let normalized = normalize(name);
    let by_name = KEYBOARD_MODELS
        .iter()
        .filter_map(|model| name_match_score(model, &normalized).map(|score| (score, model)))
        .max_by_key(|(score, _)| *score)
        .map(|(_, model)| model);
    let definition = by_name.or_else(|| {
        KEYBOARD_MODELS
            .iter()
            .find(|model| model.exact_ids.contains(&(vendor_id, product_id)))
    })?;
    let exact_hardware_match = definition.exact_ids.contains(&(vendor_id, product_id));
    Some(KeyboardModelMatch {
        definition,
        exact_hardware_match,
        limits: limits_for(definition.id),
    })
}

fn name_match_score(model: &KeyboardModelDefinition, normalized_name: &str) -> Option<(u8, usize)> {
    std::iter::once(model.name)
        .chain(model.aliases.iter().copied())
        .filter_map(|candidate| {
            let candidate = normalize(candidate);
            if candidate.is_empty() {
                None
            } else if normalized_name == candidate {
                Some((2, candidate.len()))
            } else if normalized_name.contains(&candidate) {
                Some((1, candidate.len()))
            } else {
                None
            }
        })
        .max()
}

fn limits_for(id: &str) -> KeyboardLimits {
    match id {
        "alloy-rise" => KeyboardLimits {
            lighting: KeyboardLightingTopology::PerKey,
            polling_rates_hz: vec![1000, 2000, 4000, 8000],
            onboard_profiles: Some(10),
            ..KeyboardLimits::default()
        },
        "alloy-rise-75" => KeyboardLimits {
            lighting: KeyboardLightingTopology::PerKey,
            polling_rates_hz: vec![1000, 2000, 4000, 8000],
            onboard_profiles: Some(10),
            ..KeyboardLimits::default()
        },
        "alloy-rise-75-wireless" => KeyboardLimits {
            lighting: KeyboardLightingTopology::PerKey,
            onboard_profiles: Some(10),
            wireless: true,
            battery: true,
            ..KeyboardLimits::default()
        },
        "origins-2-pro-65" => KeyboardLimits {
            lighting: KeyboardLightingTopology::PerKey,
            polling_rates_hz: vec![1000, 2000, 4000, 8000],
            hall_effect: true,
            rapid_trigger: true,
            ..KeyboardLimits::default()
        },
        "origins-2-65" | "origins-2-1800" => KeyboardLimits {
            lighting: KeyboardLightingTopology::PerKey,
            polling_rates_hz: vec![1000, 2000, 4000, 8000],
            ..KeyboardLimits::default()
        },
        "eve-1800" => KeyboardLimits {
            lighting: KeyboardLightingTopology::Zone(10),
            ..KeyboardLimits::default()
        },
        "alloy-elite-rgb" | "alloy-fps-rgb" | "alloy-origins" | "alloy-origins-core"
        | "alloy-elite-2" | "alloy-origins-60" | "alloy-origins-65" | "alloy-mkw100" => {
            KeyboardLimits {
                lighting: KeyboardLightingTopology::PerKey,
                ..KeyboardLimits::default()
            }
        }
        _ => KeyboardLimits::default(),
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
        let mut ids = KEYBOARD_MODELS
            .iter()
            .map(|model| model.id)
            .collect::<Vec<_>>();
        let len = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), len);
    }

    #[test]
    fn exact_origins_65_is_an_exact_match() {
        let matched = model_for(0x03f0, 0x038f, "HyperX Alloy Origins 65").unwrap();
        assert_eq!(matched.definition.id, "alloy-origins-65");
        assert!(matched.exact_hardware_match);
    }

    #[test]
    fn rise_75_wireless_name_beats_shorter_rise_alias() {
        let matched = model_for(0x03f0, 0xffff, "HyperX Alloy Rise 75 Wireless").unwrap();
        assert_eq!(matched.definition.id, "alloy-rise-75-wireless");
        assert!(!matched.exact_hardware_match);
    }

    #[test]
    fn origins_2_pro_name_beats_origins_2_65() {
        let matched = model_for(0x03f0, 0xffff, "HyperX Origins 2 Pro 65").unwrap();
        assert_eq!(matched.definition.id, "origins-2-pro-65");
    }

    #[test]
    fn name_only_models_never_have_a_native_driver() {
        let matched = model_for(0x03f0, 0xffff, "HyperX Origins 2 Pro 65").unwrap();
        assert!(matched.definition.native_driver.is_none());
        assert!(!matched.exact_hardware_match);
    }
}
