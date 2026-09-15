#![forbid(unsafe_code)]
//! Servo-backed engine integration for Aether Browser.

mod live;

use aether_engine_api::{BrowserEngine, EngineError, EngineEvent, WebViewId};
use std::collections::BTreeSet;
use url::Url;

pub use live::run_live_browser;

#[derive(Debug, Default)]
pub struct ServoEngineAdapter {
    next_id: u64,
    known: BTreeSet<WebViewId>,
    events: Vec<EngineEvent>,
}

impl ServoEngineAdapter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            next_id: 0,
            known: BTreeSet::new(),
            events: Vec::new(),
        }
    }

    #[must_use]
    pub const fn backend_name(&self) -> &'static str {
        "servo"
    }

    fn require(&self, id: WebViewId) -> Result<(), EngineError> {
        self.known
            .contains(&id)
            .then_some(())
            .ok_or(EngineError::InvalidWebView(id))
    }
}

pub fn normalize_startup_url(input: &str) -> Result<String, EngineError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(EngineError::OperationFailed("startup URL is empty".into()));
    }

    let candidate = if trimmed.contains("://") || trimmed.starts_with("about:") {
        trimmed.to_owned()
    } else {
        format!("https://{trimmed}")
    };

    Url::parse(&candidate)
        .map(|url| url.to_string())
        .map_err(|error| EngineError::OperationFailed(format!("invalid startup URL: {error}")))
}

impl BrowserEngine for ServoEngineAdapter {
    fn create_webview(&mut self, initial_url: Option<&str>) -> Result<WebViewId, EngineError> {
        let normalized = initial_url.map(normalize_startup_url).transpose()?;
        self.next_id = self.next_id.saturating_add(1);
        let id = WebViewId::new(self.next_id);
        self.known.insert(id);
        self.events.push(EngineEvent::Created { id });
        if let Some(url) = normalized {
            self.events.push(EngineEvent::Navigated { id, url });
        }
        Ok(id)
    }

    fn destroy_webview(&mut self, id: WebViewId) -> Result<(), EngineError> {
        self.require(id)?;
        self.known.remove(&id);
        self.events.push(EngineEvent::Destroyed { id });
        Ok(())
    }

    fn navigate(&mut self, id: WebViewId, url: &str) -> Result<(), EngineError> {
        self.require(id)?;
        self.events.push(EngineEvent::Navigated {
            id,
            url: normalize_startup_url(url)?,
        });
        Ok(())
    }

    fn reload(&mut self, id: WebViewId) -> Result<(), EngineError> {
        self.require(id)
    }

    fn stop(&mut self, id: WebViewId) -> Result<(), EngineError> {
        self.require(id)?;
        Err(EngineError::UnsupportedCapability {
            capability: "stop_loading",
            reason: "not implemented by the Servo adapter",
        })
    }

    fn go_back(&mut self, id: WebViewId) -> Result<(), EngineError> {
        self.require(id)
    }

    fn go_forward(&mut self, id: WebViewId) -> Result<(), EngineError> {
        self.require(id)
    }

    fn set_visible(&mut self, id: WebViewId, _visible: bool) -> Result<(), EngineError> {
        self.require(id)
    }

    fn set_focused(&mut self, id: WebViewId, _focused: bool) -> Result<(), EngineError> {
        self.require(id)
    }

    fn resize(&mut self, id: WebViewId, _width: u32, _height: u32) -> Result<(), EngineError> {
        self.require(id)
    }

    fn take_events(&mut self) -> Vec<EngineEvent> {
        std::mem::take(&mut self.events)
    }
}
