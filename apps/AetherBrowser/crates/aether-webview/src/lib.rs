#![forbid(unsafe_code)]
//! Browser-facing ownership and lifecycle control for engine webviews.

use aether_engine_api::{BrowserEngine, EngineError, EngineEvent, WebViewId};

#[derive(Debug)]
pub struct WebViewController<E> {
    engine: E,
}

impl<E> WebViewController<E>
where
    E: BrowserEngine,
{
    #[must_use]
    pub const fn new(engine: E) -> Self {
        Self { engine }
    }

    pub fn create(&mut self, initial_url: Option<&str>) -> Result<WebViewId, EngineError> {
        self.engine.create_webview(initial_url)
    }

    pub fn destroy(&mut self, id: WebViewId) -> Result<(), EngineError> {
        self.engine.destroy_webview(id)
    }

    pub fn navigate(&mut self, id: WebViewId, url: &str) -> Result<(), EngineError> {
        self.engine.navigate(id, url)
    }

    pub fn reload(&mut self, id: WebViewId) -> Result<(), EngineError> {
        self.engine.reload(id)
    }

    pub fn stop(&mut self, id: WebViewId) -> Result<(), EngineError> {
        self.engine.stop(id)
    }

    pub fn go_back(&mut self, id: WebViewId) -> Result<(), EngineError> {
        self.engine.go_back(id)
    }

    pub fn go_forward(&mut self, id: WebViewId) -> Result<(), EngineError> {
        self.engine.go_forward(id)
    }

    pub fn set_visible(&mut self, id: WebViewId, visible: bool) -> Result<(), EngineError> {
        self.engine.set_visible(id, visible)
    }

    pub fn set_focused(&mut self, id: WebViewId, focused: bool) -> Result<(), EngineError> {
        self.engine.set_focused(id, focused)
    }

    pub fn resize(&mut self, id: WebViewId, width: u32, height: u32) -> Result<(), EngineError> {
        self.engine.resize(id, width, height)
    }

    pub fn take_events(&mut self) -> Vec<EngineEvent> {
        self.engine.take_events()
    }

    #[must_use]
    pub fn engine(&self) -> &E {
        &self.engine
    }

    pub fn engine_mut(&mut self) -> &mut E {
        &mut self.engine
    }

    #[must_use]
    pub fn into_engine(self) -> E {
        self.engine
    }
}
