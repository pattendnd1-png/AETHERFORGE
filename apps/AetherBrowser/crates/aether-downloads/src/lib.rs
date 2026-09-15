#![forbid(unsafe_code)]
//! Download state machine with persistence through the Aether Library.

use aether_storage::{DownloadWrite, LibraryStore, StorageError, unix_now};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DownloadId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DownloadState {
    Queued,
    Active,
    Paused,
    Complete,
    Failed,
    Cancelled,
}

impl DownloadState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Complete => "complete",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownloadRecord {
    pub id: DownloadId,
    pub source_url: String,
    pub destination: String,
    pub state: DownloadState,
    pub received_bytes: u64,
    pub total_bytes: Option<u64>,
}

impl DownloadRecord {
    pub fn persist(&self, store: &LibraryStore) -> Result<(), StorageError> {
        store.record_download(DownloadWrite {
            id: self.id.0,
            source_url: &self.source_url,
            destination: &self.destination,
            state: self.state.as_str(),
            received_bytes: self.received_bytes,
            total_bytes: self.total_bytes,
            unix_seconds: unix_now(),
        })
    }

    pub fn transition(&mut self, next: DownloadState) -> Result<(), &'static str> {
        let valid = match (self.state, next) {
            (
                DownloadState::Queued,
                DownloadState::Active | DownloadState::Cancelled | DownloadState::Failed,
            )
            | (
                DownloadState::Active,
                DownloadState::Paused
                | DownloadState::Complete
                | DownloadState::Cancelled
                | DownloadState::Failed,
            )
            | (
                DownloadState::Paused,
                DownloadState::Active | DownloadState::Cancelled | DownloadState::Failed,
            ) => true,
            (current, target) if current == target => true,
            _ => false,
        };
        if valid {
            self.state = next;
            Ok(())
        } else {
            Err("invalid download state transition")
        }
    }
}
