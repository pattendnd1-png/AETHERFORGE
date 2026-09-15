pub mod protocol;
pub mod registry;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MouseError {
    #[error("HID access failed: {0}")]
    Hid(String),
    #[error("unsupported mouse operation: {0}")]
    Unsupported(String),
    #[error("invalid mouse configuration: {0}")]
    Invalid(String),
    #[error("mouse protocol response error: {0}")]
    Protocol(String),
}

pub use protocol::haste_v1::{PulsefireHasteWireless, DRIVER_ID as HASTE_V1_DRIVER_ID};
pub use protocol::saga_pro::{SagaPro, DRIVER_ID as SAGA_PRO_DRIVER_ID};
pub use registry::{
    model_for, native_driver_for, MouseModelDefinition, MouseModelMatch, MOUSE_MODELS,
};
