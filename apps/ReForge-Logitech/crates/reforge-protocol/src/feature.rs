use crate::{HidppResponse, ProtocolError};
use serde::{Deserialize, Serialize};

pub const ROOT: u16 = 0x0000;
pub const FEATURE_SET: u16 = 0x0001;
pub const FEATURE_INFO: u16 = 0x0002;
pub const DEVICE_FW_VERSION: u16 = 0x0003;
pub const DEVICE_UNIT_ID: u16 = 0x0004;
pub const DEVICE_NAME: u16 = 0x0005;
pub const BATTERY_STATUS: u16 = 0x1000;
pub const BATTERY_VOLTAGE: u16 = 0x1001;
pub const UNIFIED_BATTERY: u16 = 0x1004;
pub const CHARGING_CONTROL: u16 = 0x1010;
pub const BACKLIGHT: u16 = 0x1981;
pub const BACKLIGHT2: u16 = 0x1982;
pub const BACKLIGHT3: u16 = 0x1983;
pub const REPROG_CONTROLS_V4: u16 = 0x1b04;
pub const ANALOG_BUTTONS: u16 = 0x1b0c;
pub const PERSISTENT_REMAPPABLE_ACTION: u16 = 0x1c00;
pub const SMART_SHIFT: u16 = 0x2110;
pub const SMART_SHIFT_ENHANCED: u16 = 0x2111;
pub const HI_RES_SCROLLING: u16 = 0x2120;
pub const HIRES_WHEEL: u16 = 0x2121;
pub const LOWRES_WHEEL: u16 = 0x2130;
pub const THUMB_WHEEL: u16 = 0x2150;
pub const ADJUSTABLE_DPI: u16 = 0x2201;
pub const EXTENDED_ADJUSTABLE_DPI: u16 = 0x2202;
pub const POINTER_SPEED: u16 = 0x2205;
pub const FN_INVERSION: u16 = 0x40a0;
pub const NEW_FN_INVERSION: u16 = 0x40a2;
pub const GKEY: u16 = 0x8010;
pub const BRIGHTNESS_CONTROL: u16 = 0x8040;
pub const REPORT_RATE: u16 = 0x8060;
pub const EXTENDED_REPORT_RATE: u16 = 0x8061;
pub const COLOR_LED_EFFECTS: u16 = 0x8070;
pub const RGB_EFFECTS: u16 = 0x8071;
pub const PER_KEY_LIGHTING: u16 = 0x8080;
pub const PER_KEY_LIGHTING_V2: u16 = 0x8081;
pub const ONBOARD_PROFILES: u16 = 0x8100;
pub const PROFILE_MANAGEMENT: u16 = 0x8101;
pub const SIDETONE: u16 = 0x8300;
pub const EQUALIZER: u16 = 0x8310;
pub const HEADSET_OUT: u16 = 0x8320;
pub const HEADSET_VOLUME: u16 = 0x0200;
pub const HEADSET_EQ: u16 = 0x0201;
pub const HEADSET_ADVANCED_PARA_EQ: u16 = 0x020d;
pub const HEADSET_MIC_MUTE: u16 = 0x0601;
pub const HEADSET_MIC_SNR: u16 = 0x0602;
pub const HEADSET_AUDIO_SIDETONE: u16 = 0x0604;
pub const HEADSET_AI_NOISE_REDUCTION: u16 = 0x060e;
pub const HEADSET_MIC_GAIN: u16 = 0x0611;
pub const HEADSET_BATTERY_SAVER: u16 = 0x0618;
pub const HEADSET_RGB_EFFECTS: u16 = 0x0600;
pub const HEADSET_RGB_ONBOARD_EFFECTS: u16 = 0x0621;
pub const HEADSET_DO_NOT_DISTURB: u16 = 0x0631;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureEntry {
    pub index: u8,
    pub feature_id: u16,
    pub metadata: u8,
    pub version: u8,
}

pub fn parse_feature_index(response: &HidppResponse) -> Result<Option<u8>, ProtocolError> {
    let index = *response
        .params
        .first()
        .ok_or(ProtocolError::Malformed("root feature response has no feature index"))?;
    Ok((index != 0).then_some(index))
}

pub fn parse_feature_set_count(response: &HidppResponse) -> Result<u8, ProtocolError> {
    response
        .params
        .first()
        .copied()
        .ok_or(ProtocolError::Malformed("feature-set count response is empty"))
}

pub fn parse_feature_set_entry(
    index: u8,
    response: &HidppResponse,
) -> Result<FeatureEntry, ProtocolError> {
    if response.params.len() < 3 {
        return Err(ProtocolError::Malformed("feature-set entry response is too short"));
    }
    Ok(FeatureEntry {
        index,
        feature_id: u16::from_be_bytes([response.params[0], response.params[1]]),
        metadata: response.params[2],
        version: response.params.get(3).copied().unwrap_or(0),
    })
}

pub fn feature_name(id: u16) -> &'static str {
    match id {
        ROOT => "Root",
        FEATURE_SET => "Feature Set",
        FEATURE_INFO => "Feature Info",
        DEVICE_FW_VERSION => "Firmware Version",
        DEVICE_UNIT_ID => "Device Unit ID",
        DEVICE_NAME => "Device Name",
        BATTERY_STATUS => "Battery Status",
        UNIFIED_BATTERY => "Unified Battery",
        BACKLIGHT => "Backlight",
        BACKLIGHT2 => "Backlight 2",
        REPROG_CONTROLS_V4 => "Reprogrammable Controls v4",
        ADJUSTABLE_DPI => "Adjustable DPI",
        EXTENDED_ADJUSTABLE_DPI => "Extended Adjustable DPI",
        POINTER_SPEED => "Pointer Speed",
        BRIGHTNESS_CONTROL => "Brightness Control",
        REPORT_RATE => "Report Rate",
        EXTENDED_REPORT_RATE => "Extended Report Rate",
        COLOR_LED_EFFECTS => "Color LED Effects",
        RGB_EFFECTS => "RGB Effects",
        PER_KEY_LIGHTING => "Per-Key Lighting",
        PER_KEY_LIGHTING_V2 => "Per-Key Lighting v2",
        ONBOARD_PROFILES => "Onboard Profiles",
        SIDETONE => "Sidetone",
        EQUALIZER => "Equalizer",
        BATTERY_VOLTAGE => "Battery Voltage",
        CHARGING_CONTROL => "Charging Control",
        BACKLIGHT3 => "Backlight 3",
        ANALOG_BUTTONS => "Analog Buttons",
        PERSISTENT_REMAPPABLE_ACTION => "Persistent Remappable Action",
        SMART_SHIFT => "SmartShift",
        SMART_SHIFT_ENHANCED => "SmartShift Enhanced",
        HI_RES_SCROLLING => "High Resolution Scrolling",
        HIRES_WHEEL => "High Resolution Wheel",
        LOWRES_WHEEL => "Low Resolution Wheel",
        THUMB_WHEEL => "Thumb Wheel",
        FN_INVERSION => "Fn Inversion",
        NEW_FN_INVERSION => "New Fn Inversion",
        GKEY => "G Keys",
        PROFILE_MANAGEMENT => "Profile Management",
        HEADSET_OUT => "Headset Output",
        HEADSET_VOLUME => "Headset Volume",
        HEADSET_EQ => "Headset EQ",
        HEADSET_ADVANCED_PARA_EQ => "Headset Advanced Parametric EQ",
        HEADSET_MIC_MUTE => "Headset Microphone Mute",
        HEADSET_MIC_SNR => "Headset Microphone SNR",
        HEADSET_AUDIO_SIDETONE => "Headset Audio Sidetone",
        HEADSET_AI_NOISE_REDUCTION => "Headset AI Noise Reduction",
        HEADSET_MIC_GAIN => "Headset Microphone Gain",
        HEADSET_BATTERY_SAVER => "Headset Battery Saver",
        HEADSET_RGB_EFFECTS => "Headset RGB Effects",
        HEADSET_RGB_ONBOARD_EFFECTS => "Headset RGB Onboard Effects",
        HEADSET_DO_NOT_DISTURB => "Headset Do Not Disturb",
        _ => "Unknown / unimplemented feature",
    }
}
