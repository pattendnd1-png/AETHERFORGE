#[test]
fn terminal_destination_is_native_and_shared_with_aether_terminal() {
    let app = include_str!("../src/app.rs");
    let main = include_str!("../src/main.rs");
    let cargo = include_str!("../Cargo.toml");
    let ui = include_str!("../../../crates/aether-ui/src/lib.rs");

    assert!(main.contains("mod terminal;"));
    assert!(ui.contains("Terminal,"));
    assert!(app.contains("DesktopPage::Terminal"));
    assert!(app.contains("render_terminal"));
    assert!(cargo.contains("aether-terminal-core"));
    assert!(cargo.contains("aether-terminal-ui"));
    assert!(cargo.contains("aether-terminal-settings"));
}

#[test]
fn terminal_integration_does_not_embed_konsole() {
    let app = include_str!("../src/app.rs");
    let main = include_str!("../src/main.rs");
    assert!(!app.to_ascii_lowercase().contains("konsole"));
    assert!(!main.to_ascii_lowercase().contains("konsole"));
}
