pub mod backend;
pub mod match_device;
pub mod openrgb;
pub mod ratbag;
pub mod selector;
pub mod services;

pub use backend::{BackendCandidate, BackendError};
pub use match_device::{match_external_device, ExternalIdentity, MatchError};
pub use openrgb::{OpenRgbClient, OpenRgbController, OpenRgbProtocolInfo};
pub use ratbag::{RatbagClient, RatbagDevice};
pub use selector::{select_owner, select_owner_with_health};
pub use services::BackendServiceManager;
