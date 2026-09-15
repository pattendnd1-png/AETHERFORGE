#[test]
fn v037_preserves_native_zero_cost_chatgpt_library_contract() {
    let app = include_str!("../src/app.rs");
    let main = include_str!("../src/main.rs");

    assert!(!app.contains("chatgpt.com"));
    assert!(!app.contains("privacy.openai.com"));
    assert!(!app.contains("open_official_url("));
    assert!(
        !main
            .lines()
            .map(str::trim)
            .any(|line| { line == concat!("mod ", "chatgpt_webview;") })
    );
    assert!(app.contains("Import ChatGPT Library"));
    assert!(app.contains("import_chatgpt_export_all"));
}
