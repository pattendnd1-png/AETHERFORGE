use crate::protocol::{
    AETHERAI_PROTOCOL_VERSION, AETHERAI_SYSTEM_SOCKET_RELATIVE, ProtocolError, SystemAiOperation,
    SystemAiRequest, SystemAiResponse, decode_frame, encode_frame,
};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

const MAX_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum SystemAiClientError {
    #[error("system AetherAI is unavailable: {0}")]
    Unavailable(String),
    #[error("system AetherAI protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("system AetherAI I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("system AetherAI returned an error: {code}: {message}")]
    Remote { code: String, message: String },
    #[error("system AetherAI returned an unexpected response")]
    UnexpectedResponse,
}

#[derive(Debug, Clone)]
pub struct SystemAetherAiClient {
    socket_path: PathBuf,
}

impl SystemAetherAiClient {
    pub fn canonical() -> Self {
        Self::with_socket_path(system_aetherai_socket_path())
    }

    pub fn with_socket_path(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub async fn ping(&self) -> Result<(), SystemAiClientError> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|error| SystemAiClientError::Unavailable(error.to_string()))?;

        let request = SystemAiRequest {
            protocol: AETHERAI_PROTOCOL_VERSION,
            operation: SystemAiOperation::Ping,
        };
        let frame = encode_frame(&request)?;
        stream.write_all(&frame).await?;
        stream.flush().await?;

        let mut prefix = [0_u8; 4];
        stream.read_exact(&mut prefix).await?;
        let length = u32::from_le_bytes(prefix) as usize;
        if length > MAX_FRAME_BYTES {
            return Err(SystemAiClientError::Protocol(ProtocolError::FrameTooLarge));
        }

        let mut payload = vec![0_u8; length];
        stream.read_exact(&mut payload).await?;
        let mut response_frame = Vec::with_capacity(4 + length);
        response_frame.extend_from_slice(&prefix);
        response_frame.extend_from_slice(&payload);

        match decode_frame::<SystemAiResponse>(&response_frame)? {
            SystemAiResponse::Pong => Ok(()),
            SystemAiResponse::Error { code, message } => {
                Err(SystemAiClientError::Remote { code, message })
            }
        }
    }
}

pub fn system_aetherai_socket_path() -> PathBuf {
    if let Some(runtime_dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir).join(AETHERAI_SYSTEM_SOCKET_RELATIVE);
    }

    let uid = std::env::var("UID").unwrap_or_else(|_| "user".into());
    PathBuf::from(format!("/tmp/aetherforge-{uid}/aetherai/service.sock"))
}
