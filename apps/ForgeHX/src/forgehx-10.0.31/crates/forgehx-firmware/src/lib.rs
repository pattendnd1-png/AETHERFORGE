pub mod package;
pub mod registry;
pub mod staging;
pub mod transaction;

pub use package::validate_package;
pub use registry::{FirmwareAdapterDescriptor, FirmwareRegistry, UsbIdentity};
pub use staging::{FirmwareStager, StagedFirmware};
pub use transaction::{FirmwareJournal, FirmwareTransaction};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum FirmwareError {
    #[error("firmware source is not a regular file: {0}")]
    NotRegularFile(String),
    #[error("firmware staging I/O error: {0}")]
    Io(String),
    #[error("firmware adapter not found")]
    AdapterNotFound,
    #[error("firmware update is not enabled for this model")]
    UpdateNotEnabled,
    #[error("invalid firmware transaction transition: {0}")]
    InvalidTransition(String),
    #[error("firmware journal error: {0}")]
    Journal(String),
}
