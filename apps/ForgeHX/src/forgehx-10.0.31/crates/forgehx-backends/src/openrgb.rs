use crate::backend::BackendError;
use forgehx_core::{
    LightingConfig, LightingControllerMetadata, LightingEffect, LightingModeMetadata,
    LightingZoneMetadata,
};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

const MAGIC: &[u8; 4] = b"ORGB";
const DEFAULT_ADDR: &str = "127.0.0.1:6742";
const CLIENT_PROTOCOL_MAX: u32 = 5;
const MAX_PACKET: usize = 8 * 1024 * 1024;
const REQUEST_CONTROLLER_COUNT: u32 = 0;
const REQUEST_CONTROLLER_DATA: u32 = 1;
const REQUEST_PROTOCOL_VERSION: u32 = 40;
const SET_CLIENT_NAME: u32 = 50;
const REQUEST_RESCAN_DEVICES: u32 = 140;
const UPDATE_LEDS: u32 = 1050;
const UPDATE_MODE: u32 = 1101;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Header {
    device: u32,
    packet_id: u32,
    size: u32,
}

#[derive(Debug, Clone)]
struct OpenRgbMode {
    index: u32,
    name: String,
    value: i32,
    flags: u32,
    speed_min: u32,
    speed_max: u32,
    brightness_min: Option<u32>,
    brightness_max: Option<u32>,
    colors_min: u32,
    colors_max: u32,
    speed: u32,
    brightness: Option<u32>,
    direction: u32,
    color_mode: u32,
    colors: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenRgbProtocolInfo {
    pub server_version: u32,
    pub negotiated_version: u32,
}

#[derive(Debug, Clone)]
pub struct OpenRgbController {
    pub metadata: LightingControllerMetadata,
    active_mode: i32,
    modes: Vec<OpenRgbMode>,
}

#[derive(Debug, Clone)]
pub struct OpenRgbClient {
    addr: SocketAddr,
    timeout: Duration,
}

impl Default for OpenRgbClient {
    fn default() -> Self {
        Self {
            addr: DEFAULT_ADDR.parse().expect("constant OpenRGB address"),
            timeout: Duration::from_millis(900),
        }
    }
}

impl OpenRgbClient {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            ..Self::default()
        }
    }

    fn connect(&self) -> Result<(TcpStream, OpenRgbProtocolInfo), BackendError> {
        let mut stream = TcpStream::connect_timeout(&self.addr, self.timeout)
            .map_err(|e| BackendError::Unavailable(e.to_string()))?;
        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(|e| BackendError::Io(e.to_string()))?;
        stream
            .set_write_timeout(Some(self.timeout))
            .map_err(|e| BackendError::Io(e.to_string()))?;
        send_packet(
            &mut stream,
            0,
            REQUEST_PROTOCOL_VERSION,
            &CLIENT_PROTOCOL_MAX.to_le_bytes(),
        )?;
        let (header, payload) = read_packet(&mut stream)?;
        if header.packet_id != REQUEST_PROTOCOL_VERSION || payload.len() != 4 {
            return Err(BackendError::Protocol(
                "invalid OpenRGB version response".into(),
            ));
        }
        let server_version = u32::from_le_bytes(payload[..4].try_into().unwrap());
        let negotiated_version = server_version.min(CLIENT_PROTOCOL_MAX);
        if negotiated_version == 0 {
            return Err(BackendError::Protocol(
                "OpenRGB protocol 0 is not supported by ForgeHX".into(),
            ));
        }
        let mut name = b"ForgeHX".to_vec();
        name.push(0);
        send_packet(&mut stream, 0, SET_CLIENT_NAME, &name)?;
        Ok((
            stream,
            OpenRgbProtocolInfo {
                server_version,
                negotiated_version,
            },
        ))
    }

    pub fn probe(&self) -> Result<OpenRgbProtocolInfo, BackendError> {
        self.connect().map(|(_, protocol)| protocol)
    }

    pub fn controller_count(&self) -> Result<u32, BackendError> {
        let (mut stream, _) = self.connect()?;
        send_packet(&mut stream, 0, REQUEST_CONTROLLER_COUNT, &[])?;
        let (header, payload) = read_packet(&mut stream)?;
        if header.packet_id != REQUEST_CONTROLLER_COUNT || payload.len() != 4 {
            return Err(BackendError::Protocol(
                "invalid OpenRGB controller-count response".into(),
            ));
        }
        Ok(u32::from_le_bytes(payload[..4].try_into().unwrap()))
    }

    pub fn controllers(&self) -> Result<Vec<OpenRgbController>, BackendError> {
        self.controllers_with_protocol()
            .map(|(controllers, _)| controllers)
    }

    pub fn controllers_with_protocol(
        &self,
    ) -> Result<(Vec<OpenRgbController>, OpenRgbProtocolInfo), BackendError> {
        let (mut stream, protocol_info) = self.connect()?;
        let protocol = protocol_info.negotiated_version;
        send_packet(&mut stream, 0, REQUEST_CONTROLLER_COUNT, &[])?;
        let (_, payload) = read_packet(&mut stream)?;
        if payload.len() != 4 {
            return Err(BackendError::Protocol(
                "invalid OpenRGB controller count".into(),
            ));
        }
        let count = u32::from_le_bytes(payload[..4].try_into().unwrap()).min(512);
        let mut controllers = Vec::new();
        for index in 0..count {
            let request = protocol.to_le_bytes();
            send_packet(&mut stream, index, REQUEST_CONTROLLER_DATA, &request)?;
            let (header, payload) = read_packet(&mut stream)?;
            if header.packet_id != REQUEST_CONTROLLER_DATA || header.device != index {
                return Err(BackendError::Protocol(
                    "mismatched OpenRGB controller response".into(),
                ));
            }
            controllers.push(parse_controller(index, protocol, &payload)?);
        }
        Ok((controllers, protocol_info))
    }

    pub fn set_static_color(
        &self,
        controller_index: u32,
        led_count: u16,
        color: [u8; 3],
    ) -> Result<(), BackendError> {
        if led_count == 0 {
            return Err(BackendError::Invalid(
                "OpenRGB controller exposes zero LEDs".into(),
            ));
        }
        let (mut stream, _) = self.connect()?;
        let mut payload = Vec::with_capacity(6 + led_count as usize * 4);
        let data_size = 6u32 + led_count as u32 * 4;
        payload.extend_from_slice(&data_size.to_le_bytes());
        payload.extend_from_slice(&led_count.to_le_bytes());
        let rgb = rgb_color(color);
        for _ in 0..led_count {
            payload.extend_from_slice(&rgb.to_le_bytes());
        }
        send_packet(&mut stream, controller_index, UPDATE_LEDS, &payload)
    }

    pub fn set_brightness(
        &self,
        controller_index: u32,
        brightness: u8,
    ) -> Result<(), BackendError> {
        let controller = self
            .controllers()?
            .into_iter()
            .find(|controller| {
                controller.metadata.backend_device_id == controller_index.to_string()
            })
            .ok_or_else(|| {
                BackendError::Invalid(format!("OpenRGB controller {controller_index} not found"))
            })?;
        let mode_index = usize::try_from(controller.active_mode)
            .ok()
            .filter(|index| *index < controller.modes.len())
            .ok_or_else(|| BackendError::Invalid("OpenRGB controller has no active mode".into()))?;
        self.update_mode(
            controller_index,
            &controller.modes[mode_index],
            brightness,
            None,
            None,
        )
    }

    pub fn set_lighting(
        &self,
        controller_index: u32,
        config: &LightingConfig,
    ) -> Result<(), BackendError> {
        let controller = self
            .controllers()?
            .into_iter()
            .find(|controller| {
                controller.metadata.backend_device_id == controller_index.to_string()
            })
            .ok_or_else(|| {
                BackendError::Invalid(format!("OpenRGB controller {controller_index} not found"))
            })?;
        if matches!(&config.effect, LightingEffect::Static) {
            let brightness = config.brightness.min(100) as u16;
            let color = config
                .color
                .map(|channel| ((channel as u16 * brightness) / 100) as u8);
            return self.set_static_color(controller_index, controller.metadata.led_count, color);
        }
        let wanted: &[&str] = match &config.effect {
            LightingEffect::Breathing => &["breath", "pulse"],
            LightingEffect::Wave => &["wave", "chase"],
            LightingEffect::Spectrum => &["spectrum", "rainbow", "cycle"],
            LightingEffect::Static => &[],
        };
        let mode = controller
            .modes
            .iter()
            .find(|mode| {
                let name = mode.name.to_ascii_lowercase();
                wanted.iter().any(|needle| name.contains(needle))
            })
            .ok_or_else(|| {
                BackendError::Invalid(format!(
                    "requested effect {:?} is not exposed by this OpenRGB controller",
                    config.effect
                ))
            })?;
        self.update_mode(
            controller_index,
            mode,
            config.brightness,
            Some(config.speed),
            Some(config.color),
        )
    }

    pub fn rescan(&self) -> Result<(), BackendError> {
        let (mut stream, protocol_info) = self.connect()?;
        let protocol = protocol_info.negotiated_version;
        if protocol < 5 {
            return Err(BackendError::Protocol(
                "OpenRGB rescan requires protocol 5".into(),
            ));
        }
        send_packet(&mut stream, 0, REQUEST_RESCAN_DEVICES, &[])
    }

    fn update_mode(
        &self,
        controller_index: u32,
        mode: &OpenRgbMode,
        brightness_pct: u8,
        speed_pct: Option<u8>,
        color: Option<[u8; 3]>,
    ) -> Result<(), BackendError> {
        let (mut stream, protocol_info) = self.connect()?;
        let protocol = protocol_info.negotiated_version;
        let mut updated = mode.clone();
        if let Some(speed) = speed_pct {
            updated.speed = scale_percent(speed, updated.speed_min, updated.speed_max);
        }
        if protocol >= 3 {
            if let (Some(min), Some(max)) = (updated.brightness_min, updated.brightness_max) {
                updated.brightness = Some(scale_percent(brightness_pct, min, max));
            }
        }
        if let Some(color) = color {
            let count = updated
                .colors
                .len()
                .max(updated.colors_min as usize)
                .min(updated.colors_max.max(updated.colors_min) as usize);
            if count > 0 {
                updated.colors = vec![rgb_color(color); count];
            }
        }
        let mut mode_data = encode_mode(&updated, protocol)?;
        let total = 8u32
            .checked_add(mode_data.len() as u32)
            .ok_or_else(|| BackendError::Invalid("OpenRGB mode packet too large".into()))?;
        let mut payload = Vec::with_capacity(total as usize);
        payload.extend_from_slice(&total.to_le_bytes());
        payload.extend_from_slice(&(updated.index as i32).to_le_bytes());
        payload.append(&mut mode_data);
        send_packet(&mut stream, controller_index, UPDATE_MODE, &payload)
    }
}

fn scale_percent(percent: u8, min: u32, max: u32) -> u32 {
    if max <= min {
        return min;
    }
    min + ((max - min) as u64 * percent.min(100) as u64 / 100) as u32
}

fn rgb_color(color: [u8; 3]) -> u32 {
    color[0] as u32 | ((color[1] as u32) << 8) | ((color[2] as u32) << 16)
}

fn encode_header(device: u32, packet_id: u32, size: u32) -> [u8; 16] {
    let mut header = [0u8; 16];
    header[..4].copy_from_slice(MAGIC);
    header[4..8].copy_from_slice(&device.to_le_bytes());
    header[8..12].copy_from_slice(&packet_id.to_le_bytes());
    header[12..16].copy_from_slice(&size.to_le_bytes());
    header
}

fn decode_header(bytes: [u8; 16]) -> Result<Header, BackendError> {
    if &bytes[..4] != MAGIC {
        return Err(BackendError::Protocol(
            "OpenRGB packet magic mismatch".into(),
        ));
    }
    Ok(Header {
        device: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        packet_id: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        size: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
    })
}

fn send_packet(
    stream: &mut TcpStream,
    device: u32,
    packet_id: u32,
    payload: &[u8],
) -> Result<(), BackendError> {
    if payload.len() > MAX_PACKET {
        return Err(BackendError::Invalid(
            "OpenRGB payload exceeds ForgeHX limit".into(),
        ));
    }
    stream
        .write_all(&encode_header(device, packet_id, payload.len() as u32))
        .map_err(|e| BackendError::Io(e.to_string()))?;
    stream
        .write_all(payload)
        .map_err(|e| BackendError::Io(e.to_string()))?;
    stream.flush().map_err(|e| BackendError::Io(e.to_string()))
}

fn read_packet(stream: &mut TcpStream) -> Result<(Header, Vec<u8>), BackendError> {
    let mut header = [0u8; 16];
    stream
        .read_exact(&mut header)
        .map_err(|e| BackendError::Protocol(format!("OpenRGB response header: {e}")))?;
    let header = decode_header(header)?;
    let size = header.size as usize;
    if size > MAX_PACKET {
        return Err(BackendError::Protocol(
            "OpenRGB packet exceeds ForgeHX limit".into(),
        ));
    }
    let mut payload = vec![0u8; size];
    stream
        .read_exact(&mut payload)
        .map_err(|e| BackendError::Protocol(format!("OpenRGB response payload: {e}")))?;
    Ok((header, payload))
}

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}
impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], BackendError> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| BackendError::Protocol("OpenRGB cursor overflow".into()))?;
        if end > self.data.len() {
            return Err(BackendError::Protocol(
                "truncated OpenRGB controller data".into(),
            ));
        }
        let value = &self.data[self.pos..end];
        self.pos = end;
        Ok(value)
    }
    fn u16(&mut self) -> Result<u16, BackendError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, BackendError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, BackendError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn string(&mut self) -> Result<String, BackendError> {
        let len = self.u16()? as usize;
        if len > 65535 {
            return Err(BackendError::Protocol("OpenRGB string too large".into()));
        }
        let bytes = self.take(len)?;
        let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
        Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
    }
    fn skip(&mut self, n: usize) -> Result<(), BackendError> {
        self.take(n).map(|_| ())
    }
}

fn parse_controller(
    index: u32,
    protocol: u32,
    data: &[u8],
) -> Result<OpenRgbController, BackendError> {
    let mut c = Cursor::new(data);
    let declared = c.u32()? as usize;
    if declared > data.len() || declared < 4 {
        return Err(BackendError::Protocol(
            "invalid OpenRGB controller data size".into(),
        ));
    }
    let _device_type = c.i32()?;
    let name = c.string()?;
    let vendor = if protocol >= 1 {
        c.string()?
    } else {
        String::new()
    };
    let description = c.string()?;
    let version = c.string()?;
    let serial = c.string()?;
    let location = c.string()?;
    let num_modes = c.u16()?.min(256);
    let active_mode = c.i32()?;
    let mut modes = Vec::new();
    for mode_index in 0..num_modes {
        let name = c.string()?;
        let value = c.i32()?;
        let flags = c.u32()?;
        let speed_min = c.u32()?;
        let speed_max = c.u32()?;
        let (brightness_min, brightness_max) = if protocol >= 3 {
            (Some(c.u32()?), Some(c.u32()?))
        } else {
            (None, None)
        };
        let colors_min = c.u32()?;
        let colors_max = c.u32()?;
        let speed = c.u32()?;
        let brightness = if protocol >= 3 { Some(c.u32()?) } else { None };
        let direction = c.u32()?;
        let color_mode = c.u32()?;
        let num_colors = c.u16()?.min(1024);
        let mut colors = Vec::with_capacity(num_colors as usize);
        for _ in 0..num_colors {
            colors.push(c.u32()?);
        }
        modes.push(OpenRgbMode {
            index: mode_index as u32,
            name,
            value,
            flags,
            speed_min,
            speed_max,
            brightness_min,
            brightness_max,
            colors_min,
            colors_max,
            speed,
            brightness,
            direction,
            color_mode,
            colors,
        });
    }
    let num_zones = c.u16()?.min(1024);
    let mut zones = Vec::new();
    for zone_index in 0..num_zones {
        let zone_name = c.string()?;
        let _zone_type = c.i32()?;
        let _min = c.u32()?;
        let _max = c.u32()?;
        let count = c.u32()?;
        let matrix_len = c.u16()? as usize;
        c.skip(matrix_len)?;
        if protocol >= 4 {
            let segments = c.u16()?.min(4096);
            for _ in 0..segments {
                let _ = c.string()?;
                c.skip(12)?;
            }
        }
        if protocol >= 5 {
            let _flags = c.u32()?;
        }
        zones.push(LightingZoneMetadata {
            index: zone_index as u32,
            name: zone_name,
            led_count: count,
        });
    }
    let num_leds = c.u16()?;
    for _ in 0..num_leds {
        let _ = c.string()?;
        let _ = c.u32()?;
    }
    let num_colors = c.u16()?;
    c.skip(num_colors as usize * 4)?;
    if protocol >= 5 {
        let alt = c.u16()?;
        for _ in 0..alt {
            let _ = c.string()?;
        }
        let _flags = c.u32()?;
    }
    let metadata = LightingControllerMetadata {
        backend_device_id: index.to_string(),
        name,
        vendor,
        description,
        version,
        serial,
        location,
        led_count: num_leds,
        modes: modes
            .iter()
            .map(|mode| LightingModeMetadata {
                index: mode.index,
                name: mode.name.clone(),
                speed_min: mode.speed_min,
                speed_max: mode.speed_max,
                brightness_min: mode.brightness_min,
                brightness_max: mode.brightness_max,
            })
            .collect(),
        zones,
        protocol_version: protocol,
    };
    Ok(OpenRgbController {
        metadata,
        active_mode,
        modes,
    })
}

fn encode_mode(mode: &OpenRgbMode, protocol: u32) -> Result<Vec<u8>, BackendError> {
    let mut out = Vec::new();
    let mut name = mode.name.as_bytes().to_vec();
    name.push(0);
    let name_len = u16::try_from(name.len())
        .map_err(|_| BackendError::Invalid("OpenRGB mode name too long".into()))?;
    out.extend_from_slice(&name_len.to_le_bytes());
    out.extend_from_slice(&name);
    out.extend_from_slice(&mode.value.to_le_bytes());
    out.extend_from_slice(&mode.flags.to_le_bytes());
    out.extend_from_slice(&mode.speed_min.to_le_bytes());
    out.extend_from_slice(&mode.speed_max.to_le_bytes());
    if protocol >= 3 {
        out.extend_from_slice(&mode.brightness_min.unwrap_or(0).to_le_bytes());
        out.extend_from_slice(&mode.brightness_max.unwrap_or(100).to_le_bytes());
    }
    out.extend_from_slice(&mode.colors_min.to_le_bytes());
    out.extend_from_slice(&mode.colors_max.to_le_bytes());
    out.extend_from_slice(&mode.speed.to_le_bytes());
    if protocol >= 3 {
        out.extend_from_slice(&mode.brightness.unwrap_or(100).to_le_bytes());
    }
    out.extend_from_slice(&mode.direction.to_le_bytes());
    out.extend_from_slice(&mode.color_mode.to_le_bytes());
    let count = u16::try_from(mode.colors.len())
        .map_err(|_| BackendError::Invalid("too many OpenRGB mode colors".into()))?;
    out.extend_from_slice(&count.to_le_bytes());
    for color in &mode.colors {
        out.extend_from_slice(&color.to_le_bytes());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_round_trip_is_16_byte_little_endian_orgb() {
        let bytes = encode_header(7, UPDATE_LEDS, 44);
        assert_eq!(&bytes[..4], b"ORGB");
        let decoded = decode_header(bytes).unwrap();
        assert_eq!(
            decoded,
            Header {
                device: 7,
                packet_id: UPDATE_LEDS,
                size: 44
            }
        );
    }

    #[test]
    fn protocol_is_clamped_to_supported_parser_version() {
        assert_eq!(9u32.min(CLIENT_PROTOCOL_MAX), 5);
        assert_eq!(3u32.min(CLIENT_PROTOCOL_MAX), 3);
    }

    #[test]
    fn newer_server_is_compatible_through_negotiation() {
        let info = OpenRgbProtocolInfo {
            server_version: 6,
            negotiated_version: 5,
        };
        assert_eq!(info.server_version, 6);
        assert_eq!(info.negotiated_version, 5);
    }

    #[test]
    fn openrgb_rgbcolor_uses_red_low_byte() {
        assert_eq!(rgb_color([0x11, 0x22, 0x33]), 0x00332211);
    }
}
