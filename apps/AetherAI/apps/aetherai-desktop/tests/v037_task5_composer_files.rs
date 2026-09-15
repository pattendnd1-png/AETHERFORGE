use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn task5_composer_has_native_attachment_state_and_picker_modes() {
    let app = read(root().join("src/app.rs"));
    assert!(app.contains("pub enum AttachmentKind"));
    assert!(app.contains("pub struct ComposerAttachment"));
    assert!(app.contains("pub struct FileSurfaceEntry"));
    assert!(app.contains("pub composer_attachments: Vec<ComposerAttachment>"));
    assert!(app.contains("pub files_root: PathBuf"));
    assert!(app.contains("pub files_entries: Vec<FileSurfaceEntry>"));
    assert!(app.contains("pub file_pick_mode: Option<AttachmentKind>"));
}
#[test]
fn task5_chat_has_file_folder_attach_and_attachment_chips() {
    let app = read(root().join("src/app.rs"));
    assert!(app.contains("\"+ FILE\""));
    assert!(app.contains("\"+ FOLDER\""));
    assert!(app.contains("ATTACHED"));
    assert!(app.contains("begin_file_picker(AttachmentKind::File)"));
    assert!(app.contains("begin_file_picker(AttachmentKind::Folder)"));
    assert!(app.contains("remove_attachment"));
}
#[test]
fn task5_files_page_is_a_native_browser_and_picker() {
    let app = read(root().join("src/app.rs"));
    assert!(app.contains("DesktopPage::Files => self.render_files"));
    assert!(app.contains("fn render_files("));
    assert!(app.contains("scan_files_surface"));
    assert!(app.contains("ATTACH THIS FOLDER"));
    assert!(app.contains("\"UP\""));
    assert!(app.contains("\"REFRESH\""));
}
#[test]
fn task5_drag_drop_is_wired_through_egui_031_raw_input() {
    let native = read(root().join("src/native_ui.rs"));
    assert!(native.contains("dropped_files"));
    assert!(native.contains("hovered_files"));
    assert!(native.contains("native_attach_dropped_paths"));
    assert!(native.contains("DROP FILES OR FOLDERS TO ATTACH"));
}
#[test]
fn task5_attachment_context_is_bounded_and_local() {
    let app = read(root().join("src/app.rs"));
    assert!(app.contains("MAX_ATTACHMENT_PREVIEW_BYTES"));
    assert!(app.contains("MAX_FOLDER_PREVIEW_FILES"));
    assert!(app.contains("MAX_FOLDER_PREVIEW_BYTES"));
    assert!(app.contains("build_attachment_context"));
    assert!(app.contains("read_text_preview"));
}
#[test]
fn task5_normal_composer_stays_local_and_terminal_optional() {
    let app = read(root().join("src/app.rs"));
    let native = read(root().join("src/native_ui.rs"));
    assert!(!native.contains(".ensure_session("));
    assert!(app.contains("AetherAI attachments (local-only context)"));
}
