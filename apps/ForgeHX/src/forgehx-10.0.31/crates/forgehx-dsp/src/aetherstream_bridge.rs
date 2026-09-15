use crate::FRAME_SAMPLES;
use std::os::unix::net::UnixDatagram;
use std::path::PathBuf;

pub const BRIDGE_MAGIC: [u8; 8] = *b"AFXHXM01";
pub const BRIDGE_VERSION: u16 = 1;
pub const BRIDGE_RATE: u32 = 48_000;
pub const BRIDGE_CHANNELS: u16 = 1;
pub const BRIDGE_HEADER_BYTES: usize = 32;
pub const BRIDGE_PACKET_BYTES: usize =
    BRIDGE_HEADER_BYTES + FRAME_SAMPLES * std::mem::size_of::<f32>();
pub const AETHERSTREAM_SYSTEM_SOURCE_NAME: &str = "aetherstream.system.microphone";

#[derive(Debug)]
pub struct AetherStreamMicBridge {
    socket: UnixDatagram,
    target: PathBuf,
    sequence: u64,
}

impl AetherStreamMicBridge {
    pub fn new() -> Result<Self, String> {
        let socket = UnixDatagram::unbound().map_err(|error| {
            format!("cannot create ForgeHX→AetherStream mic bridge socket: {error}")
        })?;
        socket.set_nonblocking(true).map_err(|error| {
            format!("cannot make ForgeHX→AetherStream bridge nonblocking: {error}")
        })?;
        Ok(Self {
            socket,
            target: bridge_socket_path(),
            sequence: 0,
        })
    }

    pub fn send_processed_frame(&mut self, samples: &[f32; FRAME_SAMPLES]) {
        let mut packet = [0_u8; BRIDGE_PACKET_BYTES];
        packet[..8].copy_from_slice(&BRIDGE_MAGIC);
        packet[8..10].copy_from_slice(&BRIDGE_VERSION.to_le_bytes());
        packet[12..16].copy_from_slice(&BRIDGE_RATE.to_le_bytes());
        packet[16..18].copy_from_slice(&BRIDGE_CHANNELS.to_le_bytes());
        packet[18..20].copy_from_slice(&(FRAME_SAMPLES as u16).to_le_bytes());
        packet[20..28].copy_from_slice(&self.sequence.to_le_bytes());
        for (index, sample) in samples.iter().enumerate() {
            let start = BRIDGE_HEADER_BYTES + index * std::mem::size_of::<f32>();
            packet[start..start + 4].copy_from_slice(&sample.to_le_bytes());
        }
        let _ = self.socket.send_to(&packet, &self.target);
        self.sequence = self.sequence.wrapping_add(1);
    }
}

pub fn bridge_socket_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        PathBuf::from(dir).join("aetherstream/forgehx-mic.sock")
    } else {
        PathBuf::from(format!(
            "/tmp/aetherstream-{}/forgehx-mic.sock",
            std::env::var("UID").unwrap_or_else(|_| "user".into())
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_packet_contract_is_fixed_10ms_48k_mono() {
        assert_eq!(BRIDGE_MAGIC, *b"AFXHXM01");
        assert_eq!(BRIDGE_VERSION, 1);
        assert_eq!(BRIDGE_RATE, 48_000);
        assert_eq!(BRIDGE_CHANNELS, 1);
        assert_eq!(FRAME_SAMPLES, 480);
        assert_eq!(BRIDGE_PACKET_BYTES, 1_952);
    }
}
