use serde::{Deserialize, Serialize};

pub const AETHERAI_PROTOCOL_VERSION: u32 = 1;
pub const AETHERAI_SYSTEM_SOCKET_RELATIVE: &str = "aetherforge/aetherai/service.sock";
const MAX_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemAiRequest {
    pub protocol: u32,
    pub operation: SystemAiOperation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemAiOperation {
    Ping,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemAiResponse {
    Pong,
    Error { code: String, message: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("system AetherAI frame exceeds local IPC limit")]
    FrameTooLarge,
    #[error("system AetherAI protocol encode failed: {0}")]
    Encode(String),
    #[error("system AetherAI protocol decode failed: {0}")]
    Decode(String),
}

pub fn encode_frame<T: Serialize>(value: &T) -> Result<Vec<u8>, ProtocolError> {
    let payload =
        bincode::serialize(value).map_err(|error| ProtocolError::Encode(error.to_string()))?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge);
    }

    let length = u32::try_from(payload.len()).map_err(|_| ProtocolError::FrameTooLarge)?;
    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&length.to_le_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode_frame<T: for<'de> Deserialize<'de>>(frame: &[u8]) -> Result<T, ProtocolError> {
    if frame.len() < 4 {
        return Err(ProtocolError::Decode("missing length prefix".into()));
    }

    let declared = u32::from_le_bytes(frame[..4].try_into().expect("four-byte prefix")) as usize;
    if declared > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge);
    }
    if frame.len() != declared.saturating_add(4) {
        return Err(ProtocolError::Decode("frame length mismatch".into()));
    }

    bincode::deserialize(&frame[4..]).map_err(|error| ProtocolError::Decode(error.to_string()))
}
