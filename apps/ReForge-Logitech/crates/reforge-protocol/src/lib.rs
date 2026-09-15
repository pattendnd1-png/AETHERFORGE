use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

pub mod dpi;
pub mod feature;
pub mod lighting;

pub const SOFTWARE_ID: u8 = 0x08;
pub const REPORT_ID_SHORT: u8 = 0x10;
pub const REPORT_ID_LONG: u8 = 0x11;
pub const SHORT_REPORT_LEN: usize = 7;
pub const LONG_REPORT_LEN: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    InvalidFunction(u8),
    PayloadTooLarge(usize),
    InvalidReportId(u8),
    ReportTooShort(usize),
    Malformed(&'static str),
    InvalidDpi(String),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFunction(value) => write!(f, "invalid HID++ function nibble {value:#x}"),
            Self::PayloadTooLarge(size) => write!(f, "HID++ payload is too large: {size} bytes"),
            Self::InvalidReportId(id) => write!(f, "unsupported HID++ report id {id:#04x}"),
            Self::ReportTooShort(size) => write!(f, "HID++ report is too short: {size} bytes"),
            Self::Malformed(message) => write!(f, "malformed HID++ response: {message}"),
            Self::InvalidDpi(message) => write!(f, "invalid DPI: {message}"),
        }
    }
}

impl Error for ProtocolError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HidppRequest {
    pub device_index: u8,
    pub feature_index: u8,
    pub function: u8,
    pub params: Vec<u8>,
}

impl HidppRequest {
    pub fn new(
        device_index: u8,
        feature_index: u8,
        function: u8,
        params: Vec<u8>,
    ) -> Result<Self, ProtocolError> {
        if function > 0x0f {
            return Err(ProtocolError::InvalidFunction(function));
        }
        if params.len() > LONG_REPORT_LEN - 4 {
            return Err(ProtocolError::PayloadTooLarge(params.len()));
        }
        Ok(Self {
            device_index,
            feature_index,
            function,
            params,
        })
    }

    pub fn function_swid(&self) -> u8 {
        (self.function << 4) | SOFTWARE_ID
    }

    pub fn encode(&self) -> Vec<u8> {
        let len = if self.params.len() <= SHORT_REPORT_LEN - 4 {
            SHORT_REPORT_LEN
        } else {
            LONG_REPORT_LEN
        };
        let mut report = vec![0_u8; len];
        report[0] = if len == SHORT_REPORT_LEN {
            REPORT_ID_SHORT
        } else {
            REPORT_ID_LONG
        };
        report[1] = self.device_index;
        report[2] = self.feature_index;
        report[3] = self.function_swid();
        report[4..4 + self.params.len()].copy_from_slice(&self.params);
        report
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HidppResponse {
    pub report_id: u8,
    pub device_index: u8,
    pub feature_index: u8,
    pub function_swid: u8,
    pub params: Vec<u8>,
}

impl HidppResponse {
    pub fn parse(raw: &[u8]) -> Result<Self, ProtocolError> {
        if raw.len() < SHORT_REPORT_LEN {
            return Err(ProtocolError::ReportTooShort(raw.len()));
        }
        let expected_len = match raw[0] {
            REPORT_ID_SHORT => SHORT_REPORT_LEN,
            REPORT_ID_LONG => LONG_REPORT_LEN,
            other => return Err(ProtocolError::InvalidReportId(other)),
        };
        if raw.len() < expected_len {
            return Err(ProtocolError::ReportTooShort(raw.len()));
        }
        Ok(Self {
            report_id: raw[0],
            device_index: raw[1],
            feature_index: raw[2],
            function_swid: raw[3],
            params: raw[4..expected_len].to_vec(),
        })
    }

    pub fn function(&self) -> u8 {
        self.function_swid >> 4
    }

    pub fn software_id(&self) -> u8 {
        self.function_swid & 0x0f
    }

    pub fn is_hidpp_error(&self) -> bool {
        self.feature_index == 0x8f
    }
}
