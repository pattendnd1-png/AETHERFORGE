use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn composer_has_visible_caret_and_real_cursor_state() {
    let app = read(root().join("src/app.rs"));
    assert!(app.contains("pub composer_cursor: usize"));
    assert!(app.contains("composer_text_with_caret"));
    assert!(app.contains("\"│\""));
}

#[test]
fn composer_edits_at_caret_not_only_at_end() {
    let app = read(root().join("src/app.rs"));
    assert!(app.contains("self.composer.insert_str(self.composer_cursor, text)"));
    assert!(app.contains("pub fn delete_forward"));
    assert!(app.contains("pub fn cursor_left"));
    assert!(app.contains("pub fn cursor_right"));
    assert!(app.contains("pub fn cursor_home"));
    assert!(app.contains("pub fn cursor_end"));
}

#[test]
fn native_routes_navigation_and_delete_keys() {
    let native = read(root().join("src/native_ui.rs"));
    for key in [
        "egui::Key::ArrowLeft",
        "egui::Key::ArrowRight",
        "egui::Key::Home",
        "egui::Key::End",
        "egui::Key::Delete",
    ] {
        assert!(native.contains(key), "missing key route: {key}");
    }
}
