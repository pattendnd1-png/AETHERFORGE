use crate::{HidppRequest, HidppResponse, ProtocolError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DpiRange {
    pub sensor: u8,
    pub min: u16,
    pub max: u16,
    pub step: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DpiState {
    pub sensor: u8,
    pub current: u16,
    pub default: u16,
}

pub fn parse_sensor_count(response: &HidppResponse) -> Result<u8, ProtocolError> {
    response
        .params
        .first()
        .copied()
        .ok_or(ProtocolError::Malformed("DPI sensor-count response is empty"))
}

pub fn parse_dpi_values(response: &HidppResponse) -> Result<Vec<u16>, ProtocolError> {
    let _sensor = *response
        .params
        .first()
        .ok_or(ProtocolError::Malformed("DPI list response is empty"))?;

    let mut words = Vec::new();
    for pair in response.params[1..].chunks_exact(2) {
        let value = u16::from_be_bytes([pair[0], pair[1]]);
        if value == 0 {
            break;
        }
        words.push(value);
    }
    if words.is_empty() {
        return Err(ProtocolError::Malformed("DPI list contains no values"));
    }

    let mut values = Vec::new();
    let mut index = 0;
    while index < words.len() {
        let word = words[index];
        if word >> 13 == 0b111 {
            let step = word & 0x1fff;
            if step == 0 {
                return Err(ProtocolError::Malformed("DPI range step is zero"));
            }
            let start = *values
                .last()
                .ok_or(ProtocolError::Malformed("DPI range marker has no start value"))?;
            let end = *words
                .get(index + 1)
                .ok_or(ProtocolError::Malformed("DPI range marker has no end value"))?;
            if end >> 13 == 0b111 || end < start {
                return Err(ProtocolError::Malformed("DPI range end is invalid"));
            }

            let mut value = start.saturating_add(step);
            while value < end {
                values.push(value);
                let next = value.saturating_add(step);
                if next <= value {
                    return Err(ProtocolError::Malformed("DPI range overflowed"));
                }
                value = next;
            }
            if values.last().copied() != Some(end) {
                values.push(end);
            }
            index += 2;
        } else {
            if let Some(previous) = values.last().copied()
                && word < previous
            {
                return Err(ProtocolError::Malformed("DPI values are not ascending"));
            }
            if values.last().copied() != Some(word) {
                values.push(word);
            }
            index += 1;
        }
    }
    Ok(values)
}

pub fn parse_dpi_range(response: &HidppResponse) -> Result<DpiRange, ProtocolError> {
    let sensor = *response
        .params
        .first()
        .ok_or(ProtocolError::Malformed("DPI range response is empty"))?;
    let values = parse_dpi_values(response)?;
    let min = values[0];
    let max = *values.last().expect("non-empty DPI list checked above");
    let step = values
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .filter(|gap| *gap > 0)
        .min()
        .unwrap_or(1);
    Ok(DpiRange {
        sensor,
        min,
        max,
        step,
    })
}

pub fn validate_dpi_value(values: &[u16], dpi: u16) -> Result<(), ProtocolError> {
    if values.binary_search(&dpi).is_err() {
        let range = match (values.first(), values.last()) {
            (Some(min), Some(max)) => format!("{min}..={max}"),
            _ => "an empty supported-DPI list".to_owned(),
        };
        return Err(ProtocolError::InvalidDpi(format!(
            "{dpi} is not one of the device-supported values ({range})"
        )));
    }
    Ok(())
}

pub fn parse_dpi_state(response: &HidppResponse) -> Result<DpiState, ProtocolError> {
    if response.params.len() < 5 {
        return Err(ProtocolError::Malformed("DPI state response is too short"));
    }
    Ok(DpiState {
        sensor: response.params[0],
        current: u16::from_be_bytes([response.params[1], response.params[2]]),
        default: u16::from_be_bytes([response.params[3], response.params[4]]),
    })
}

pub fn validate_dpi(range: &DpiRange, dpi: u16) -> Result<(), ProtocolError> {
    if dpi < range.min || dpi > range.max {
        return Err(ProtocolError::InvalidDpi(format!(
            "{dpi} is outside {}..={}",
            range.min, range.max
        )));
    }
    if range.step > 1 && (dpi - range.min) % range.step != 0 {
        return Err(ProtocolError::InvalidDpi(format!(
            "{dpi} does not align to {} DPI steps from {}",
            range.step, range.min
        )));
    }
    Ok(())
}

pub fn build_set_dpi_request(
    device_index: u8,
    feature_index: u8,
    sensor: u8,
    dpi: u16,
) -> Result<HidppRequest, ProtocolError> {
    let [high, low] = dpi.to_be_bytes();
    HidppRequest::new(device_index, feature_index, 0x3, vec![sensor, high, low])
}
