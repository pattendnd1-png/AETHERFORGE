#![forbid(unsafe_code)]
//! Engine-neutral browser contracts shared by the Aether Browser shell and engine adapters.

use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WebViewId(u64);

impl WebViewId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EngineError {
    UnsupportedCapability {
        capability: &'static str,
        reason: &'static str,
    },
    InvalidWebView(WebViewId),
    OperationFailed(String),
}

impl Display for EngineError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedCapability { capability, reason } => {
                write!(
                    formatter,
                    "engine capability {capability} is unavailable: {reason}"
                )
            }
            Self::InvalidWebView(id) => write!(formatter, "invalid webview {}", id.get()),
            Self::OperationFailed(message) => formatter.write_str(message),
        }
    }
}

impl Error for EngineError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EngineCommand {
    Create {
        id: WebViewId,
        initial_url: Option<String>,
    },
    Destroy {
        id: WebViewId,
    },
    Navigate {
        id: WebViewId,
        url: String,
    },
    Reload {
        id: WebViewId,
    },
    Stop {
        id: WebViewId,
    },
    Back {
        id: WebViewId,
    },
    Forward {
        id: WebViewId,
    },
    SetVisible {
        id: WebViewId,
        visible: bool,
    },
    SetFocused {
        id: WebViewId,
        focused: bool,
    },
    Resize {
        id: WebViewId,
        width: u32,
        height: u32,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EngineEvent {
    Created {
        id: WebViewId,
    },
    Destroyed {
        id: WebViewId,
    },
    Navigated {
        id: WebViewId,
        url: String,
    },
    TitleChanged {
        id: WebViewId,
        title: String,
    },
    LoadProgress {
        id: WebViewId,
        percent: u8,
    },
    UnsupportedCapability {
        id: Option<WebViewId>,
        capability: String,
    },
}

pub trait BrowserEngine {
    fn create_webview(&mut self, initial_url: Option<&str>) -> Result<WebViewId, EngineError>;
    fn destroy_webview(&mut self, id: WebViewId) -> Result<(), EngineError>;
    fn navigate(&mut self, id: WebViewId, url: &str) -> Result<(), EngineError>;
    fn reload(&mut self, id: WebViewId) -> Result<(), EngineError>;
    fn stop(&mut self, id: WebViewId) -> Result<(), EngineError>;
    fn go_back(&mut self, id: WebViewId) -> Result<(), EngineError>;
    fn go_forward(&mut self, id: WebViewId) -> Result<(), EngineError>;
    fn set_visible(&mut self, id: WebViewId, visible: bool) -> Result<(), EngineError>;
    fn set_focused(&mut self, id: WebViewId, focused: bool) -> Result<(), EngineError>;
    fn resize(&mut self, id: WebViewId, width: u32, height: u32) -> Result<(), EngineError>;
    fn take_events(&mut self) -> Vec<EngineEvent>;
}
