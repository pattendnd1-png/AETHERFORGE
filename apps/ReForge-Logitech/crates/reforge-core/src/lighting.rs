use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0 };
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255 };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn packed(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }

    pub const fn from_packed(value: u32) -> Self {
        Self {
            r: ((value >> 16) & 0xff) as u8,
            g: ((value >> 8) & 0xff) as u8,
            b: (value & 0xff) as u8,
        }
    }

    pub fn scale(self, intensity: u8) -> Self {
        let factor = u16::from(intensity.min(100));
        Self {
            r: ((u16::from(self.r) * factor) / 100) as u8,
            g: ((u16::from(self.g) * factor) / 100) as u8,
            b: ((u16::from(self.b) * factor) / 100) as u8,
        }
    }

    pub fn blend(self, other: Self, amount: u8) -> Self {
        let amount = u32::from(amount);
        let inverse = 255_u32.saturating_sub(amount);
        Self {
            r: ((u32::from(self.r) * inverse + u32::from(other.r) * amount) / 255) as u8,
            g: ((u32::from(self.g) * inverse + u32::from(other.g) * amount) / 255) as u8,
            b: ((u32::from(self.b) * inverse + u32::from(other.b) * amount) / 255) as u8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LightingEffect {
    Off,
    #[default]
    Static,
    Breathing,
    ColorCycle,
    Wave,
    Ripple,
    Gradient,
    ScreenReactive,
    AudioReactive,
}

impl LightingEffect {
    pub const fn is_host_driven(self) -> bool {
        matches!(
            self,
            Self::Gradient | Self::ScreenReactive | Self::AudioReactive
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrightnessInfo {
    pub min: u16,
    pub max: u16,
    pub current: u16,
    pub can_switch_off: bool,
    pub steps: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightingZone {
    pub index: u8,
    pub location: u16,
    pub name: String,
    pub effect_ids: Vec<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LightingCapabilities {
    pub brightness: Option<BrightnessInfo>,
    #[serde(default)]
    pub backlight_v2: bool,
    pub color_led_effects: bool,
    pub rgb_effects: bool,
    pub per_key_v2: bool,
    pub supported_keys: Vec<u8>,
    pub zones: Vec<LightingZone>,
}

impl LightingCapabilities {
    pub fn supported(&self) -> bool {
        self.brightness.is_some()
            || self.backlight_v2
            || self.color_led_effects
            || self.rgb_effects
            || self.per_key_v2
    }

    pub fn supports_effect(&self, effect: LightingEffect) -> bool {
        match effect {
            LightingEffect::Off | LightingEffect::Static => {
                self.backlight_v2 || self.per_key_v2 || self.color_led_effects || self.rgb_effects
            },
            LightingEffect::Gradient
            | LightingEffect::ScreenReactive
            | LightingEffect::AudioReactive => self.per_key_v2 || !self.zones.is_empty(),
            LightingEffect::Breathing => self.zones.iter().any(|zone| {
                zone.effect_ids.contains(&0x000a) || zone.effect_ids.contains(&0x0002)
            }),
            LightingEffect::ColorCycle => self.zones.iter().any(|zone| {
                zone.effect_ids.contains(&0x0015) || zone.effect_ids.contains(&0x0003)
            }),
            LightingEffect::Wave => self.zones.iter().any(|zone| {
                zone.effect_ids.contains(&0x0016) || zone.effect_ids.contains(&0x0004)
            }),
            LightingEffect::Ripple => self.zones.iter().any(|zone| {
                zone.effect_ids.contains(&0x0017) || zone.effect_ids.contains(&0x000b)
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LightingState {
    pub enabled: bool,
    pub effect: LightingEffect,
    pub primary: RgbColor,
    pub secondary: RgbColor,
    pub brightness: Option<u16>,
    pub period_ms: u16,
    pub intensity: u8,
    pub direction: u8,
    #[serde(default)]
    pub per_key: BTreeMap<u8, RgbColor>,
}

impl Default for LightingState {
    fn default() -> Self {
        Self {
            enabled: true,
            effect: LightingEffect::Static,
            primary: RgbColor::new(0x00, 0x9d, 0xff),
            secondary: RgbColor::new(0x8a, 0x2b, 0xe2),
            brightness: None,
            period_ms: 3000,
            intensity: 100,
            direction: 1,
            per_key: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct IntegrationStatus {
    pub screen_capture: Option<String>,
    pub audio_capture: Option<String>,
    pub process_watcher: bool,
    pub active_auto_profile: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_color_round_trips() {
        let color = RgbColor::new(0x12, 0x34, 0x56);
        assert_eq!(RgbColor::from_packed(color.packed()), color);
    }

    #[test]
    fn brightness_scaling_clamps_to_one_hundred_percent() {
        let color = RgbColor::new(200, 100, 50);
        assert_eq!(color.scale(50), RgbColor::new(100, 50, 25));
        assert_eq!(color.scale(200), color);
    }

    #[test]
    fn host_driven_effects_are_explicit() {
        assert!(LightingEffect::Gradient.is_host_driven());
        assert!(LightingEffect::ScreenReactive.is_host_driven());
        assert!(LightingEffect::AudioReactive.is_host_driven());
        assert!(!LightingEffect::Wave.is_host_driven());
    }

    #[test]
    fn monochrome_backlight_supports_on_off_without_color_effects() {
        let caps = LightingCapabilities {
            backlight_v2: true,
            ..Default::default()
        };
        assert!(caps.supports_effect(LightingEffect::Off));
        assert!(caps.supports_effect(LightingEffect::Static));
        assert!(!caps.supports_effect(LightingEffect::Wave));
    }

    #[test]
    fn effect_support_uses_device_advertised_ids() {
        let caps = LightingCapabilities {
            zones: vec![LightingZone {
                index: 0,
                location: 1,
                name: "Primary".into(),
                effect_ids: vec![0x01, 0x0a, 0x16],
            }],
            ..Default::default()
        };
        assert!(caps.supports_effect(LightingEffect::Breathing));
        assert!(caps.supports_effect(LightingEffect::Wave));
        assert!(!caps.supports_effect(LightingEffect::Ripple));
    }
}
