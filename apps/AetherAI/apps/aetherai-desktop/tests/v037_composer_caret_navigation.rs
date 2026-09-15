use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn composer_has_cursor_navigation_and_visible_caret() {
    let app = read(root().join("src/app.rs"));
    let native = read(root().join("src/native_ui.rs"));
    for marker in [
        "pub composer_cursor: usize",
        "pub fn delete_forward",
        "pub fn cursor_left",
        "pub fn cursor_right",
        "pub fn cursor_home",
        "pub fn cursor_end",
        "composer_text_with_caret",
        "\"│\"",
    ] {
        assert!(app.contains(marker), "missing {marker}");
    }
    for marker in [
        "egui::Key::ArrowLeft",
        "egui::Key::ArrowRight",
        "egui::Key::Home",
        "egui::Key::End",
        "egui::Key::Delete",
    ] {
        assert!(native.contains(marker), "missing {marker}");
    }
}
