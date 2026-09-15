use forgehx_core::{BackendHealth, CapabilityOwner};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct BackendCandidate {
    pub owner: CapabilityOwner,
    pub health: BackendHealth,
}

#[derive(Debug, Error)]
pub enum BackendError {
    #[error("backend unavailable: {0}")]
    Unavailable(String),
    #[error("backend protocol error: {0}")]
    Protocol(String),
    #[error("ambiguous backend device match: {0}")]
    Ambiguous(String),
    #[error("invalid backend request: {0}")]
    Invalid(String),
    #[error("I/O error: {0}")]
    Io(String),
}
