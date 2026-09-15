use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn task7a_activity_page_exposes_health_checks_and_actions() {
    let app = read(root().join("src/app.rs"));
    assert!(app.contains("SYSTEM HEALTH"));
    assert!(app.contains("\"Recheck\""));
    assert!(app.contains("\"Repair\""));
    assert!(app.contains("\"Roll Back\""));
    assert!(app.contains("\"Open Logs\""));
    assert!(app.contains("\"Export Diagnostic\""));
    assert!(app.contains("health_action_at"));
}

#[test]
fn task7a_app_health_uses_live_storage_models_updater_and_activity_state() {
    let app = read(root().join("src/app.rs"));
    assert!(app.contains("HealthPanelState::evaluate"));
    assert!(app.contains("self.store"));
    assert!(app.contains("self.models.len()"));
    assert!(app.contains("self.providers.len()"));
    assert!(app.contains("self.update.summary()"));
}

#[test]
fn task7a_diagnostic_export_is_native_local_file_output() {
    let app = read(root().join("src/app.rs"));
    let health = read(root().join("src/health.rs"));
    assert!(app.contains("export_health_diagnostic"));
    assert!(app.contains("diagnostic_path_in"));
    assert!(!health.contains("reqwest"));
    assert!(!health.contains("ureq"));
    assert!(!health.contains("webbrowser"));
    assert!(!health.contains("http://"));
    assert!(!health.contains("https://"));
}

#[test]
fn task7a_health_contract_covers_all_required_domains() {
    let health = read(root().join("src/health.rs"));
    for key in [
        "VERSION",
        "RENDERER_SCHEMA",
        "DATABASE_MIGRATION",
        "LOCAL_MODEL_RUNTIME",
        "STORAGE",
        "RETRIEVAL_INDEX",
        "NATIVE_CHATGPT_LIBRARY",
        "UPDATER",
        "RELEASE_INTEGRITY",
        "PERMISSIONS",
        "BACKGROUND_ACTIVITY",
    ] {
        assert!(health.contains(key), "missing health key {key}");
    }
}
