//! Read-only bridge between AetherFiles and the system-level AetherAI service.
//!
//! AetherAI remains the single system model/runtime authority. This crate
//! contains no model loader, network client, shell/process launcher, file
//! mutation API, or restore API.

mod capability;
mod client;
mod protocol;

pub use capability::{
    CapabilityError, FileMetadata, FileSearchCandidate, FileSearchProvider, FileSearchRequest,
    IndexFileSearchProvider, SearchableText,
};
pub use client::{SystemAetherAiClient, system_aetherai_socket_path};
pub use protocol::{
    AETHERAI_PROTOCOL_VERSION, AETHERAI_SYSTEM_SOCKET_RELATIVE, ProtocolError, SystemAiOperation,
    SystemAiRequest, SystemAiResponse, decode_frame, encode_frame,
};
