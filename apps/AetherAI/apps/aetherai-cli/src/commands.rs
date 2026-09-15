use aether_core::ProcessingPreset;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Preset(ProcessingPreset),
    Workspace(String),
    Models,
    Model(String),
    Settings,
    Quit,
}
pub fn parse_command(input: &str) -> Option<Command> {
    let t = input.trim();
    match t {
        "/help" => return Some(Command::Help),
        "/models" => return Some(Command::Models),
        "/settings" => return Some(Command::Settings),
        "/quit" => return Some(Command::Quit),
        _ => {}
    }
    if let Some(v) = t.strip_prefix("/preset ") {
        let p = match v.trim().to_ascii_lowercase().as_str() {
            "fast" => ProcessingPreset::Fast,
            "balanced" => ProcessingPreset::Balanced,
            "deep" => ProcessingPreset::Deep,
            "maximum" => ProcessingPreset::Maximum,
            "custom" => ProcessingPreset::Custom,
            _ => return Some(Command::Help),
        };
        return Some(Command::Preset(p));
    }
    if let Some(v) = t.strip_prefix("/workspace ") {
        let v = v.trim();
        return Some(if v.is_empty() {
            Command::Help
        } else {
            Command::Workspace(v.into())
        });
    }
    if let Some(v) = t.strip_prefix("/model ") {
        let v = v.trim();
        return Some(if v.is_empty() {
            Command::Help
        } else {
            Command::Model(v.into())
        });
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_models() {
        assert_eq!(parse_command("/models"), Some(Command::Models));
    }
    #[test]
    fn parses_model() {
        assert_eq!(
            parse_command("/model local-qwen"),
            Some(Command::Model("local-qwen".into()))
        );
    }
    #[test]
    fn parses_preset() {
        assert_eq!(
            parse_command("/preset deep"),
            Some(Command::Preset(ProcessingPreset::Deep))
        );
    }
    #[test]
    fn text_is_not_command() {
        assert_eq!(parse_command("hello"), None);
    }
}
