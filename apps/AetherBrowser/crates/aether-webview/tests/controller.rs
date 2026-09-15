use aether_engine_api::{BrowserEngine, EngineCommand, EngineError, EngineEvent, WebViewId};
use aether_webview::WebViewController;

#[derive(Default)]
struct RecordingEngine {
    commands: Vec<EngineCommand>,
}

impl BrowserEngine for RecordingEngine {
    fn create_webview(&mut self, initial_url: Option<&str>) -> Result<WebViewId, EngineError> {
        let id = WebViewId::new(7);
        self.commands.push(EngineCommand::Create {
            id,
            initial_url: initial_url.map(str::to_owned),
        });
        Ok(id)
    }
    fn destroy_webview(&mut self, id: WebViewId) -> Result<(), EngineError> {
        self.commands.push(EngineCommand::Destroy { id });
        Ok(())
    }
    fn navigate(&mut self, id: WebViewId, url: &str) -> Result<(), EngineError> {
        self.commands.push(EngineCommand::Navigate {
            id,
            url: url.to_owned(),
        });
        Ok(())
    }
    fn reload(&mut self, id: WebViewId) -> Result<(), EngineError> {
        self.commands.push(EngineCommand::Reload { id });
        Ok(())
    }
    fn stop(&mut self, id: WebViewId) -> Result<(), EngineError> {
        self.commands.push(EngineCommand::Stop { id });
        Ok(())
    }
    fn go_back(&mut self, id: WebViewId) -> Result<(), EngineError> {
        self.commands.push(EngineCommand::Back { id });
        Ok(())
    }
    fn go_forward(&mut self, id: WebViewId) -> Result<(), EngineError> {
        self.commands.push(EngineCommand::Forward { id });
        Ok(())
    }
    fn set_visible(&mut self, id: WebViewId, visible: bool) -> Result<(), EngineError> {
        self.commands
            .push(EngineCommand::SetVisible { id, visible });
        Ok(())
    }
    fn set_focused(&mut self, id: WebViewId, focused: bool) -> Result<(), EngineError> {
        self.commands
            .push(EngineCommand::SetFocused { id, focused });
        Ok(())
    }
    fn resize(&mut self, id: WebViewId, width: u32, height: u32) -> Result<(), EngineError> {
        self.commands
            .push(EngineCommand::Resize { id, width, height });
        Ok(())
    }
    fn take_events(&mut self) -> Vec<EngineEvent> {
        Vec::new()
    }
}

#[test]
fn controller_drives_engine_without_engine_specific_types() {
    let mut controller = WebViewController::new(RecordingEngine::default());
    let id = controller.create(Some("https://example.com")).unwrap();
    controller.navigate(id, "https://servo.org").unwrap();
    controller.reload(id).unwrap();
    controller.destroy(id).unwrap();

    let engine = controller.into_engine();
    assert_eq!(engine.commands.len(), 4);
}
