use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn task7b_main_guards_only_pending_managed_normal_startup() {
    let main = read(root().join("src/main.rs"));
    assert!(main.contains("pending_managed_startup"));
    assert!(main.contains("current_executable_is_managed"));
    assert!(main.contains("pending_startup_version"));
    assert!(main.contains("!diagnostic"));
    assert!(main.contains("import.is_none()"));
    assert!(main.contains("import_chatgpt.is_none()"));
}

#[test]
fn task7b_checkpoint_happens_before_aether_app_database_open() {
    let main = read(root().join("src/main.rs"));
    let checkpoint = main
        .find("create_startup_checkpoint(&dir)")
        .expect("checkpoint call");
    let app_new = main.find("AetherApp::new(&dir)").expect("app construction");
    assert!(checkpoint < app_new);
}

#[test]
fn task7b_failed_startup_recovers_pointer_without_user_data_rollback() {
    let main = read(root().join("src/main.rs"));
    let lifecycle = read(root().join("../../crates/aether-lifecycle/src/startup.rs"));
    assert!(main.contains("recover_pending_startup_failure"));
    assert!(main.contains("std::process::Command::new(&binary)"));
    assert!(lifecycle.contains("AETHERAI_USER_DATA_MUTATED=NO"));
    assert!(!lifecycle.contains("remove_dir_all"));
}

#[test]
fn task7b_first_native_frame_marks_pending_release_healthy() {
    let app = read(root().join("src/app.rs"));
    let native = read(root().join("src/native_ui.rs"));
    assert!(app.contains("startup_health_pending"));
    assert!(app.contains("native_first_frame_startup_health"));
    assert!(native.contains("self.app.native_first_frame_startup_health();"));
}

#[test]
fn task7b_health_module_has_prestartup_database_checkpoint() {
    let health = read(root().join("src/health.rs"));
    assert!(health.contains("pub fn create_startup_checkpoint"));
    assert!(health.contains("\"aetherai.db\""));
    assert!(health.contains("\"aetherai.db-wal\""));
    assert!(health.contains("\"aetherai.db-shm\""));
}
