use crate::{HidppRequest, HidppResponse, ProtocolError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrightnessRange {
    pub min: u16,
    pub max: u16,
    pub steps: u8,
    pub can_switch_off: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backlight2Info {
    pub enabled: bool,
    pub options: u8,
    pub supported: u8,
    pub effects: u16,
    pub level: u8,
    pub hands_out: u16,
    pub hands_in: u16,
    pub powered: u16,
}

impl Backlight2Info {
    pub fn mode(&self) -> u8 {
        (self.options >> 3) & 0x03
    }

    pub fn manual_supported(&self) -> bool {
        self.supported & 0x20 != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedProtocolInfo {
    pub zone_count: u8,
    pub readable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedZoneProbe {
    pub location: u16,
    pub effect_count: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedEffectProbe {
    pub zone_index: u8,
    pub effect_index: u8,
    pub effect_id: u16,
    pub capabilities: u16,
    pub period: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LedEffectParameters {
    pub color: u32,
    pub speed: u8,
    pub period_ms: u16,
    pub intensity: u8,
    pub saturation: u8,
    pub direction: u8,
    pub ramp: u8,
    pub form: u8,
}

pub fn backlight2_info_request(device: u8, feature: u8) -> Result<HidppRequest, ProtocolError> {
    HidppRequest::new(device, feature, 0, Vec::new())
}

pub fn backlight2_range_request(device: u8, feature: u8) -> Result<HidppRequest, ProtocolError> {
    HidppRequest::new(device, feature, 2, Vec::new())
}

pub fn parse_backlight2_info(response: &HidppResponse) -> Result<Backlight2Info, ProtocolError> {
    if response.params.len() < 12 {
        return Err(ProtocolError::Malformed("Backlight2 info response is too short"));
    }
    Ok(Backlight2Info {
        enabled: response.params[0] != 0,
        options: response.params[1],
        supported: response.params[2],
        effects: u16::from_le_bytes([response.params[3], response.params[4]]),
        level: response.params[5],
        hands_out: u16::from_le_bytes([response.params[6], response.params[7]]),
        hands_in: u16::from_le_bytes([response.params[8], response.params[9]]),
        powered: u16::from_le_bytes([response.params[10], response.params[11]]),
    })
}

pub fn parse_backlight2_range(response: &HidppResponse) -> Result<BrightnessRange, ProtocolError> {
    let count = response
        .params
        .first()
        .copied()
        .ok_or(ProtocolError::Malformed("Backlight2 range response is empty"))?;
    if count <= 1 {
        return Err(ProtocolError::Malformed("Backlight2 reports no adjustable brightness levels"));
    }
    Ok(BrightnessRange {
        min: 0,
        max: u16::from(count - 1),
        steps: count,
        can_switch_off: false,
    })
}

pub fn backlight2_write_request(
    device: u8,
    feature: u8,
    current: &Backlight2Info,
    enabled: bool,
    level: Option<u8>,
) -> Result<HidppRequest, ProtocolError> {
    let mut options = current.options;
    let requested_level = level.unwrap_or(current.level);
    if level.is_some() && current.manual_supported() {
        options = (options & 0x07) | (0x03 << 3);
    }
    let wire_level = if ((options >> 3) & 0x03) == 0x03 {
        requested_level
    } else {
        0
    };
    let mut payload = vec![u8::from(enabled), options, 0xff, wire_level];
    payload.extend_from_slice(&current.hands_out.to_le_bytes());
    payload.extend_from_slice(&current.hands_in.to_le_bytes());
    payload.extend_from_slice(&current.powered.to_le_bytes());
    HidppRequest::new(device, feature, 1, payload)
}

pub fn brightness_info_request(device: u8, feature: u8) -> Result<HidppRequest, ProtocolError> {
    HidppRequest::new(device, feature, 0, Vec::new())
}

pub fn brightness_read_request(device: u8, feature: u8) -> Result<HidppRequest, ProtocolError> {
    HidppRequest::new(device, feature, 1, Vec::new())
}

pub fn brightness_write_request(
    device: u8,
    feature: u8,
    value: u16,
) -> Result<HidppRequest, ProtocolError> {
    HidppRequest::new(device, feature, 2, value.to_be_bytes().to_vec())
}

pub fn brightness_power_request(
    device: u8,
    feature: u8,
    enabled: bool,
) -> Result<HidppRequest, ProtocolError> {
    HidppRequest::new(device, feature, 4, vec![u8::from(enabled)])
}

pub fn parse_brightness_range(response: &HidppResponse) -> Result<BrightnessRange, ProtocolError> {
    if response.params.len() < 6 {
        return Err(ProtocolError::Malformed("brightness info response is too short"));
    }
    Ok(BrightnessRange {
        max: u16::from_be_bytes([response.params[0], response.params[1]]),
        steps: response.params[2] & 0x0f,
        can_switch_off: response.params[3] & 0x04 != 0,
        min: u16::from_be_bytes([response.params[4], response.params[5]]),
    })
}

pub fn parse_brightness_value(response: &HidppResponse) -> Result<u16, ProtocolError> {
    if response.params.len() < 2 {
        return Err(ProtocolError::Malformed("brightness value response is too short"));
    }
    Ok(u16::from_be_bytes([response.params[0], response.params[1]]))
}

pub fn color_led_info_request(device: u8, feature: u8) -> Result<HidppRequest, ProtocolError> {
    HidppRequest::new(device, feature, 0, Vec::new())
}

pub fn rgb_info_request(device: u8, feature: u8) -> Result<HidppRequest, ProtocolError> {
    HidppRequest::new(device, feature, 0, vec![0xff, 0xff, 0x00])
}

pub fn zone_info_request(
    device: u8,
    feature: u8,
    zone_index: u8,
) -> Result<HidppRequest, ProtocolError> {
    HidppRequest::new(device, feature, 1, vec![zone_index, 0xff, 0x00])
}

pub fn rgb_zone_info_request(
    device: u8,
    feature: u8,
    zone_index: u8,
) -> Result<HidppRequest, ProtocolError> {
    HidppRequest::new(device, feature, 0, vec![zone_index, 0xff, 0x00])
}

pub fn effect_info_request(
    device: u8,
    feature: u8,
    rgb_effects: bool,
    zone_index: u8,
    effect_index: u8,
) -> Result<HidppRequest, ProtocolError> {
    HidppRequest::new(
        device,
        feature,
        if rgb_effects { 0 } else { 2 },
        vec![zone_index, effect_index, 0],
    )
}

pub fn parse_led_protocol_info(
    response: &HidppResponse,
    rgb_effects: bool,
) -> Result<LedProtocolInfo, ProtocolError> {
    let params = &response.params;
    if rgb_effects {
        if params.len() < 7 {
            return Err(ProtocolError::Malformed("RGB effects info response is too short"));
        }
        Ok(LedProtocolInfo {
            zone_count: params[2],
            readable: u16::from_be_bytes([params[5], params[6]]) & 1 != 0,
        })
    } else {
        if params.len() < 5 {
            return Err(ProtocolError::Malformed("color LED effects info response is too short"));
        }
        Ok(LedProtocolInfo {
            zone_count: params[0],
            readable: u16::from_be_bytes([params[3], params[4]]) & 1 != 0,
        })
    }
}

pub fn parse_zone_probe(
    response: &HidppResponse,
    rgb_effects: bool,
) -> Result<LedZoneProbe, ProtocolError> {
    let offset = if rgb_effects { 2 } else { 1 };
    if response.params.len() < offset + 3 {
        return Err(ProtocolError::Malformed("LED zone response is too short"));
    }
    Ok(LedZoneProbe {
        location: u16::from_be_bytes([response.params[offset], response.params[offset + 1]]),
        effect_count: response.params[offset + 2],
    })
}

pub fn parse_effect_probe(response: &HidppResponse) -> Result<LedEffectProbe, ProtocolError> {
    if response.params.len() < 8 {
        return Err(ProtocolError::Malformed("LED effect response is too short"));
    }
    Ok(LedEffectProbe {
        zone_index: response.params[0],
        effect_index: response.params[1],
        effect_id: u16::from_be_bytes([response.params[2], response.params[3]]),
        capabilities: u16::from_be_bytes([response.params[4], response.params[5]]),
        period: u16::from_be_bytes([response.params[6], response.params[7]]),
    })
}

pub fn encode_effect_parameters(effect_id: u16, values: &LedEffectParameters) -> [u8; 10] {
    let mut params = [0_u8; 10];
    let color = (values.color & 0x00ff_ffff).to_be_bytes();
    let set_color = |params: &mut [u8; 10], offset: usize| {
        params[offset..offset + 3].copy_from_slice(&color[1..4]);
    };
    match effect_id {
        0x0001 => {
            set_color(&mut params, 0);
            params[3] = values.ramp;
        }
        0x0002 => {
            set_color(&mut params, 0);
            params[3] = values.speed;
        }
        0x0003 => {
            params[5..7].copy_from_slice(&values.period_ms.to_be_bytes());
            params[7] = values.intensity;
        }
        0x0004 => {
            params[6..8].copy_from_slice(&values.period_ms.to_be_bytes());
            params[9] = values.direction;
        }
        0x000a => {
            set_color(&mut params, 0);
            params[3..5].copy_from_slice(&values.period_ms.to_be_bytes());
            params[5] = values.form;
            params[6] = values.intensity;
        }
        0x000b => {
            set_color(&mut params, 0);
            params[4..6].copy_from_slice(&values.period_ms.to_be_bytes());
        }
        0x0015 => {
            params[1] = values.saturation;
            params[6..8].copy_from_slice(&values.period_ms.to_be_bytes());
            params[8] = values.intensity;
        }
        0x0016 => {
            params[1] = values.saturation;
            params[6..8].copy_from_slice(&values.period_ms.to_be_bytes());
            params[8] = values.intensity;
            params[9] = values.direction;
        }
        0x0017 => {
            set_color(&mut params, 0);
            params[3] = values.saturation;
            params[6..8].copy_from_slice(&values.period_ms.to_be_bytes());
        }
        _ => {}
    }
    params
}

pub fn color_led_set_effect_request(
    device: u8,
    feature: u8,
    zone_index: u8,
    effect_index: u8,
    params: [u8; 10],
) -> Result<HidppRequest, ProtocolError> {
    let mut payload = vec![zone_index, effect_index];
    payload.extend_from_slice(&params);
    HidppRequest::new(device, feature, 3, payload)
}

pub fn rgb_set_effect_request(
    device: u8,
    feature: u8,
    zone_index: u8,
    effect_index: u8,
    params: [u8; 10],
) -> Result<HidppRequest, ProtocolError> {
    let mut payload = vec![zone_index, effect_index];
    payload.extend_from_slice(&params);
    payload.push(1);
    HidppRequest::new(device, feature, 1, payload)
}

pub fn color_led_control_request(
    device: u8,
    feature: u8,
    enabled: bool,
) -> Result<HidppRequest, ProtocolError> {
    HidppRequest::new(device, feature, 8, vec![u8::from(enabled)])
}

pub fn rgb_sw_control_request(
    device: u8,
    feature: u8,
    enabled: bool,
) -> Result<HidppRequest, ProtocolError> {
    let payload = if enabled {
        vec![0x01, 0x03, 0x04]
    } else {
        vec![0x01, 0x00, 0x00]
    };
    HidppRequest::new(device, feature, 5, payload)
}

pub fn onboard_profile_mode_request(
    device: u8,
    feature: u8,
    software_mode: bool,
) -> Result<HidppRequest, ProtocolError> {
    HidppRequest::new(device, feature, 1, vec![if software_mode { 0x02 } else { 0x01 }])
}

pub fn per_key_bitmap_request(
    device: u8,
    feature: u8,
    page: u8,
) -> Result<HidppRequest, ProtocolError> {
    HidppRequest::new(device, feature, 0, vec![0x00, page])
}

pub fn per_key_set_color_request(
    device: u8,
    feature: u8,
    key: u8,
    color: u32,
) -> Result<HidppRequest, ProtocolError> {
    let bytes = (color & 0x00ff_ffff).to_be_bytes();
    HidppRequest::new(device, feature, 1, vec![key, bytes[1], bytes[2], bytes[3]])
}

pub fn per_key_set_many_request(
    device: u8,
    feature: u8,
    values: &[(u8, u32)],
) -> Result<HidppRequest, ProtocolError> {
    if values.is_empty() || values.len() > 4 {
        return Err(ProtocolError::Malformed("per-key packed update must contain 1 to 4 keys"));
    }
    let mut payload = Vec::with_capacity(values.len() * 4);
    for (key, color) in values {
        let bytes = (color & 0x00ff_ffff).to_be_bytes();
        payload.extend_from_slice(&[*key, bytes[1], bytes[2], bytes[3]]);
    }
    HidppRequest::new(device, feature, 1, payload)
}

pub fn per_key_set_uniform_request(
    device: u8,
    feature: u8,
    keys: &[u8],
    color: u32,
) -> Result<HidppRequest, ProtocolError> {
    if keys.is_empty() || keys.len() > 13 {
        return Err(ProtocolError::Malformed("uniform per-key update must contain 1 to 13 keys"));
    }
    let bytes = (color & 0x00ff_ffff).to_be_bytes();
    let mut payload = vec![bytes[1], bytes[2], bytes[3]];
    payload.extend_from_slice(keys);
    HidppRequest::new(device, feature, 6, payload)
}

pub fn per_key_set_range_request(
    device: u8,
    feature: u8,
    first: u8,
    last: u8,
    color: u32,
) -> Result<HidppRequest, ProtocolError> {
    let bytes = (color & 0x00ff_ffff).to_be_bytes();
    HidppRequest::new(
        device,
        feature,
        5,
        vec![first, last, bytes[1], bytes[2], bytes[3]],
    )
}

pub fn per_key_frame_end_request(device: u8, feature: u8) -> Result<HidppRequest, ProtocolError> {
    HidppRequest::new(device, feature, 7, vec![0x00])
}

pub fn parse_per_key_bitmap_page(response: &HidppResponse) -> Result<&[u8], ProtocolError> {
    if response.params.len() < 3 {
        return Err(ProtocolError::Malformed("per-key bitmap response is too short"));
    }
    Ok(&response.params[2..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backlight2_info_decodes_little_endian_layout() {
        let response = HidppResponse {
            report_id: 0x11,
            device_index: 0xff,
            feature_index: 3,
            function_swid: 0x08,
            params: vec![1, 0x18, 0x20, 0x34, 0x12, 4, 2, 0, 3, 0, 12, 0],
        };
        let info = parse_backlight2_info(&response).unwrap();
        assert!(info.enabled);
        assert_eq!(info.effects, 0x1234);
        assert_eq!(info.level, 4);
        assert_eq!(info.hands_out, 2);
        assert_eq!(info.mode(), 3);
        assert!(info.manual_supported());
    }

    #[test]
    fn backlight2_write_preserves_durations_and_selects_manual_mode() {
        let current = Backlight2Info {
            enabled: true,
            options: 0x01,
            supported: 0x20,
            effects: 0,
            level: 2,
            hands_out: 3,
            hands_in: 4,
            powered: 5,
        };
        let request = backlight2_write_request(0xff, 4, &current, true, Some(7)).unwrap();
        assert_eq!(request.function, 1);
        assert_eq!(request.params, vec![1, 0x19, 0xff, 7, 3, 0, 4, 0, 5, 0]);
    }

    #[test]
    fn brightness_info_decodes_solaar_layout() {
        let response = HidppResponse {
            report_id: 0x11,
            device_index: 0xff,
            feature_index: 3,
            function_swid: 0x08,
            params: vec![0x00, 0x64, 0x05, 0x04, 0x00, 0x0d, 0, 0],
        };
        let range = parse_brightness_range(&response).unwrap();
        assert_eq!(range.max, 100);
        assert_eq!(range.min, 13);
        assert_eq!(range.steps, 5);
        assert!(range.can_switch_off);
    }

    #[test]
    fn static_effect_payload_places_rgb_at_start() {
        let params = encode_effect_parameters(
            0x01,
            &LedEffectParameters {
                color: 0x123456,
                ramp: 3,
                ..Default::default()
            },
        );
        assert_eq!(&params[..4], &[0x12, 0x34, 0x56, 3]);
    }

    #[test]
    fn per_key_single_color_uses_function_one() {
        let request = per_key_set_color_request(0xff, 4, 0x04, 0x112233).unwrap();
        assert_eq!(request.function, 1);
        assert_eq!(request.params, vec![0x04, 0x11, 0x22, 0x33]);
    }

    #[test]
    fn per_key_frame_end_uses_function_seven() {
        let request = per_key_frame_end_request(0xff, 4).unwrap();
        assert_eq!(request.function, 7);
        assert_eq!(request.params, vec![0]);
    }

    #[test]
    fn rgb_software_claim_matches_documented_host_control_bytes() {
        let request = rgb_sw_control_request(0xff, 4, true).unwrap();
        assert_eq!(request.function, 5);
        assert_eq!(request.params, vec![1, 3, 4]);
    }
}
