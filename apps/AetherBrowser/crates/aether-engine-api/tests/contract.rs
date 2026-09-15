use aether_engine_api::{BrowserEngine, EngineCommand, EngineError, EngineEvent, WebViewId};

#[derive(Default)]
struct RecordingEngine {
    next_id: u64,
    commands: Vec<EngineCommand>,
    events: Vec<EngineEvent>,
}

impl BrowserEngine for RecordingEngine {
    fn create_webview(&mut self, initial_url: Option<&str>) -> Result<WebViewId, EngineError> {
        self.next_id += 1;
        let id = WebViewId::new(self.next_id);
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
        std::mem::take(&mut self.events)
    }
}

#[test]
fn engine_contract_represents_browser_lifecycle_commands() {
    let mut engine = RecordingEngine::default();
    let id = engine.create_webview(Some("https://example.com")).unwrap();
    engine.navigate(id, "https://servo.org").unwrap();
    engine.go_back(id).unwrap();
    engine.go_forward(id).unwrap();
    engine.reload(id).unwrap();
    engine.stop(id).unwrap();
    engine.set_visible(id, false).unwrap();
    engine.set_focused(id, true).unwrap();
    engine.resize(id, 1920, 1080).unwrap();
    engine.destroy_webview(id).unwrap();

    assert_eq!(engine.commands.len(), 10);
    assert_eq!(
        engine.commands[0],
        EngineCommand::Create {
            id,
            initial_url: Some("https://example.com".to_owned()),
        }
    );
    assert_eq!(
        engine.commands[1],
        EngineCommand::Navigate {
            id,
            url: "https://servo.org".to_owned(),
        }
    );
    assert_eq!(engine.commands[9], EngineCommand::Destroy { id });
}
