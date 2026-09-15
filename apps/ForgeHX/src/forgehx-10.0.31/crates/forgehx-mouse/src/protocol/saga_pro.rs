use crate::MouseError;
use forgehx_core::{DeviceInfo, InterfaceSource, LightingConfig, LightingEffect};
use hidapi::HidApi;
use std::ffi::CString;

pub const DRIVER_ID: &str = "hyperx-pulsefire-saga-pro-v1";
pub const HYPERX_VID: u16 = 0x03f0;
pub const SAGA_PRO_WIRED_PID: u16 = 0x04bf;
pub const SAGA_PRO_WIRELESS_PID: u16 = 0x06bf;
const PACKET_SIZE: usize = 64;
const WIRED_CONFIG_INTERFACE: i32 = 2;
const WIRELESS_CONFIG_INTERFACE: i32 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SagaBatteryStatus {
    pub percent: Option<u8>,
    pub charging: Option<bool>,
    pub full: Option<bool>,
    pub temperature_c: Option<u16>,
    pub voltage_mv: Option<u16>,
}

#[derive(Debug, Clone)]
pub struct SagaPro {
    path: String,
    wireless: bool,
}

impl SagaPro {
    pub fn from_device(device: &DeviceInfo) -> Result<Self, MouseError> {
        if device.vendor_id != HYPERX_VID
            || !matches!(
                device.product_id,
                SAGA_PRO_WIRED_PID | SAGA_PRO_WIRELESS_PID
            )
            || device.protocol.as_deref() != Some(DRIVER_ID)
        {
            return Err(MouseError::Hid(
                "device is not a registered Pulsefire Saga Pro".into(),
            ));
        }
        let path = select_config_interface(device).ok_or_else(|| {
            MouseError::Hid("Pulsefire Saga Pro configuration HID interface was not found".into())
        })?;
        Ok(Self {
            path,
            wireless: device.product_id == SAGA_PRO_WIRELESS_PID,
        })
    }

    pub fn battery_status(&self) -> Result<SagaBatteryStatus, MouseError> {
        let device = self.open()?;
        drain(&device);
        write_packet(&device, &battery_query_packet())?;
        for _ in 0..12 {
            let mut response = [0u8; PACKET_SIZE];
            match device.read_timeout(&mut response, 100) {
                Ok(length) if length >= 3 => {
                    if let Some(status) = parse_battery_status(&response[..length]) {
                        return Ok(status);
                    }
                }
                Ok(_) => {}
                Err(error) => return Err(MouseError::Hid(error.to_string())),
            }
        }
        Err(MouseError::Protocol(
            "Saga Pro battery query timed out".into(),
        ))
    }

    pub fn set_lighting(&self, config: &LightingConfig) -> Result<(), MouseError> {
        if config.effect != LightingEffect::Static {
            return Err(MouseError::Unsupported(
                "Saga Pro native lighting currently verifies static color only".into(),
            ));
        }
        let device = self.open()?;
        for packet in live_rgb_packets(config.color, config.brightness) {
            write_packet(&device, &packet)?;
        }
        Ok(())
    }

    pub fn is_wireless(&self) -> bool {
        self.wireless
    }

    fn open(&self) -> Result<hidapi::HidDevice, MouseError> {
        let api = HidApi::new().map_err(|error| MouseError::Hid(error.to_string()))?;
        let path = CString::new(self.path.as_bytes())
            .map_err(|_| MouseError::Hid("invalid hidraw path".into()))?;
        api.open_path(&path)
            .map_err(|error| MouseError::Hid(error.to_string()))
    }
}

pub fn select_config_interface(device: &DeviceInfo) -> Option<String> {
    if device.vendor_id != HYPERX_VID
        || !matches!(
            device.product_id,
            SAGA_PRO_WIRED_PID | SAGA_PRO_WIRELESS_PID
        )
    {
        return None;
    }
    let wanted = if device.product_id == SAGA_PRO_WIRELESS_PID {
        WIRELESS_CONFIG_INTERFACE
    } else {
        WIRED_CONFIG_INTERFACE
    };
    device
        .interfaces
        .iter()
        .filter(|interface| {
            interface.source == InterfaceSource::Hid
                && interface.vendor_id == Some(HYPERX_VID)
                && interface.product_id == Some(device.product_id)
        })
        .find(|interface| interface.interface_number == Some(wanted))
        .map(|interface| interface.path.clone())
}

pub fn battery_query_packet() -> [u8; PACKET_SIZE] {
    report(&[0x50, 0x02])
}

pub fn parse_battery_status(packet: &[u8]) -> Option<SagaBatteryStatus> {
    if packet.len() < 8 || packet[0] != 0x51 || packet[1] != 0x02 {
        return None;
    }
    let state = packet[3];
    Some(SagaBatteryStatus {
        percent: (packet[2] <= 100).then_some(packet[2]),
        charging: match state {
            0x00 => Some(false),
            0x01 | 0x02 => Some(true),
            _ => None,
        },
        full: match state {
            0x02 => Some(true),
            0x00 | 0x01 => Some(false),
            _ => None,
        },
        temperature_c: Some(u16::from_le_bytes([packet[4], packet[5]])),
        voltage_mv: Some(u16::from_le_bytes([packet[6], packet[7]])),
    })
}

pub fn live_rgb_packets(color: [u8; 3], brightness: u8) -> [[u8; PACKET_SIZE]; 3] {
    let brightness = brightness.min(100);
    let intensity = ((u16::from(brightness) * 255 + 50) / 100) as u8;
    [
        report(&[0x44, 0x01, 0x01, 0x00]),
        report(&[0x44, 0x02, 0x00, 0x00, color[0], color[1], color[2]]),
        report(&[0x40, 0x01, 0x00, 0x00, intensity]),
    ]
}

pub fn dpi_to_code(dpi: u16) -> Result<u16, MouseError> {
    if !(50..=26000).contains(&dpi) || !dpi.is_multiple_of(50) {
        return Err(MouseError::Invalid(
            "Saga Pro DPI must be 50..=26000 in 50-DPI steps".into(),
        ));
    }
    Ok(dpi / 50 - 1)
}

pub fn polling_rate_code(rate_hz: u16, wireless: bool) -> Result<u8, MouseError> {
    // Verified Saga relationship: rate_hz = 8000 / interval_code.
    let code = match rate_hz {
        125 => 0x40,
        250 => 0x20,
        500 => 0x10,
        1000 => 0x08,
        2000 if wireless => 0x04,
        4000 if wireless => 0x02,
        _ => {
            return Err(MouseError::Invalid(
                "unsupported Saga Pro polling rate for this transport".into(),
            ))
        }
    };
    Ok(code)
}

pub fn build_dpi_packet(
    dpis: &[u16],
    active_stage: usize,
    polling_hz: u16,
    wireless: bool,
    profile: bool,
) -> Result<[u8; PACKET_SIZE], MouseError> {
    if dpis.len() != 4 || active_stage >= 4 {
        return Err(MouseError::Invalid(
            "Saga Pro requires exactly four DPI stages and active stage 0..3".into(),
        ));
    }
    let colors = [
        [0xff, 0x00, 0x00],
        [0x00, 0x00, 0xff],
        [0xff, 0xff, 0x00],
        [0x00, 0xff, 0x00],
    ];
    let mut prefix = vec![
        0x32,
        0x01,
        if profile { 0x01 } else { 0x00 },
        0x00,
        polling_rate_code(polling_hz, wireless)?,
        0x0f,
        active_stage as u8,
    ];
    for (dpi, color) in dpis.iter().copied().zip(colors) {
        let code = dpi_to_code(dpi)?;
        prefix.extend_from_slice(&[
            (code & 0xff) as u8,
            (code >> 8) as u8,
            color[0],
            color[1],
            color[2],
        ]);
    }
    Ok(report(&prefix))
}

fn report(prefix: &[u8]) -> [u8; PACKET_SIZE] {
    let mut packet = [0u8; PACKET_SIZE];
    let len = prefix.len().min(PACKET_SIZE);
    packet[..len].copy_from_slice(&prefix[..len]);
    packet
}

fn write_packet(device: &hidapi::HidDevice, packet: &[u8; PACKET_SIZE]) -> Result<(), MouseError> {
    let written = device
        .write(packet)
        .map_err(|error| MouseError::Hid(error.to_string()))?;
    if written != PACKET_SIZE {
        return Err(MouseError::Hid(format!(
            "short Pulsefire Saga Pro HID write: {written}/{PACKET_SIZE}"
        )));
    }
    Ok(())
}

fn drain(device: &hidapi::HidDevice) {
    loop {
        let mut buf = [0u8; PACKET_SIZE];
        match device.read_timeout(&mut buf, 0) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polling_codes_follow_verified_interval_relationship() {
        assert_eq!(polling_rate_code(1000, false).unwrap(), 0x08);
        assert_eq!(polling_rate_code(4000, true).unwrap(), 0x02);
        assert!(polling_rate_code(4000, false).is_err());
    }

    #[test]
    fn dpi_packet_has_verified_profile_prefix() {
        let packet = build_dpi_packet(&[400, 800, 1600, 3200], 1, 1000, false, true).unwrap();
        assert_eq!(&packet[..7], &[0x32, 0x01, 0x01, 0x00, 0x08, 0x0f, 0x01]);
        assert_eq!(&packet[7..9], &[0x07, 0x00]);
    }

    #[test]
    fn battery_parser_decodes_status_fields() {
        let status = parse_battery_status(&[0x51, 0x02, 77, 0x01, 31, 0, 0x74, 0x0e]).unwrap();
        assert_eq!(status.percent, Some(77));
        assert_eq!(status.charging, Some(true));
        assert_eq!(status.temperature_c, Some(31));
        assert_eq!(status.voltage_mv, Some(3700));
    }
}
