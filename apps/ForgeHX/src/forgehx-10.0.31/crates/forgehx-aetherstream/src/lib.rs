use forgehx_core::{OutputDspProfile, PRESERVED_AUDIO_ENDPOINT};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use uuid::Uuid;

const IPC_PROTOCOL_VERSION: u16 = 1;
const MAX_FRAME: usize = 8 * 1024 * 1024;
pub const DEFAULT_OUTPUT_DSP_TARGET: &str = "@DEFAULT_AUDIO_SINK@";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcEnvelope<T> {
    pub protocol_version: u16,
    pub message_id: Uuid,
    pub payload: T,
}

impl<T> IpcEnvelope<T> {
    fn new(payload: T) -> Self {
        Self {
            protocol_version: IPC_PROTOCOL_VERSION,
            message_id: Uuid::new_v4(),
            payload,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputDspRuntimeRequest {
    Apply {
        device_id: String,
        profile: OutputDspProfile,
    },
    Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputDspRuntimeResponse {
    Applied {
        device_id: String,
        generation: u64,
    },
    Status {
        live: bool,
        active_device_id: Option<String>,
        generation: u64,
    },
    Rejected {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputDspRuntimeStatus {
    pub live: bool,
    pub active_device_id: Option<String>,
    pub generation: u64,
}

#[derive(Default)]
pub struct OutputDspClient;

impl OutputDspClient {
    #[must_use]
    pub fn socket_path() -> PathBuf {
        if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
            PathBuf::from(dir).join("aetherstream/output-dsp.sock")
        } else {
            PathBuf::from(format!(
                "/tmp/aetherstream-{}/output-dsp.sock",
                std::env::var("UID").unwrap_or_else(|_| "user".into())
            ))
        }
    }

    pub fn status(&self) -> Result<OutputDspRuntimeStatus, String> {
        match self.exchange(OutputDspRuntimeRequest::Status)? {
            OutputDspRuntimeResponse::Status {
                live,
                active_device_id,
                generation,
            } => Ok(OutputDspRuntimeStatus {
                live,
                active_device_id,
                generation,
            }),
            OutputDspRuntimeResponse::Rejected { reason } => Err(reason),
            OutputDspRuntimeResponse::Applied { .. } => {
                Err("AetherStream returned apply reply to status request".into())
            }
        }
    }

    pub fn apply(&self, profile: OutputDspProfile) -> Result<u64, String> {
        profile.validate(48_000.0, 2).map_err(|e| e.to_string())?;
        // Routing is deliberately outside this client. The current user-owned endpoint
        // remains PRESERVED_AUDIO_ENDPOINT; AetherStream processes the existing default sink.
        let _routing_lock = PRESERVED_AUDIO_ENDPOINT;
        match self.exchange(OutputDspRuntimeRequest::Apply {
            device_id: DEFAULT_OUTPUT_DSP_TARGET.into(),
            profile,
        })? {
            OutputDspRuntimeResponse::Applied { generation, .. } => Ok(generation),
            OutputDspRuntimeResponse::Rejected { reason } => Err(reason),
            OutputDspRuntimeResponse::Status { .. } => {
                Err("AetherStream returned status reply to apply request".into())
            }
        }
    }

    fn exchange(
        &self,
        request: OutputDspRuntimeRequest,
    ) -> Result<OutputDspRuntimeResponse, String> {
        let mut stream = UnixStream::connect(Self::socket_path())
            .map_err(|e| format!("connect AetherStream output DSP: {e}"))?;
        write_frame(&mut stream, &IpcEnvelope::new(request))?;
        let reply: IpcEnvelope<OutputDspRuntimeResponse> = read_frame(&mut stream)?;
        if reply.protocol_version != IPC_PROTOCOL_VERSION {
            return Err(format!(
                "AetherStream output-DSP protocol mismatch: {}",
                reply.protocol_version
            ));
        }
        Ok(reply.payload)
    }
}

fn write_frame<T: Serialize>(stream: &mut UnixStream, value: &T) -> Result<(), String> {
    let payload = postcard::to_stdvec(value).map_err(|e| e.to_string())?;
    if payload.len() > MAX_FRAME {
        return Err("AetherStream output-DSP frame too large".into());
    }
    let len =
        u32::try_from(payload.len()).map_err(|_| "AetherStream output-DSP frame too large")?;
    stream
        .write_all(&u32::to_be_bytes(len))
        .map_err(|e| e.to_string())?;
    stream.write_all(&payload).map_err(|e| e.to_string())?;
    Ok(())
}

fn read_frame<T: DeserializeOwned>(stream: &mut UnixStream) -> Result<T, String> {
    let mut prefix = [0u8; 4];
    stream.read_exact(&mut prefix).map_err(|e| e.to_string())?;
    let len = u32::from_be_bytes(prefix) as usize;
    if len == 0 || len > MAX_FRAME {
        return Err("invalid AetherStream output-DSP frame length".into());
    }
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload).map_err(|e| e.to_string())?;
    postcard::from_bytes(&payload).map_err(|e| e.to_string())
}
