use std::{
    fs,
    path::{Path, PathBuf},
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read(p: impl AsRef<Path>) -> String {
    fs::read_to_string(p).unwrap_or_default()
}
#[test]
fn settings_has_lifecycle_controls() {
    let a = read(root().join("src/app.rs"));
    for s in [
        "APPLICATION LIFECYCLE",
        "\"Check Updates\"",
        "\"Prepare Update\"",
        "\"Restart & Update\"",
        "\"Roll Back\"",
        "UpdateSettingsAction",
    ] {
        assert!(a.contains(s), "missing {s}");
    }
}
#[test]
fn app_spawns_only_lifecycle_selected_binary() {
    let a = read(root().join("src/app.rs"));
    assert!(a.contains("self.update.restart_and_update()"));
    assert!(a.contains("self.update.rollback()"));
    assert!(a.contains("std::process::Command::new(&binary)"));
    assert!(a.contains("self.should_quit = true"));
}
#[test]
fn restart_surface_uses_verified_gate() {
    let a = read(root().join("src/app.rs"));
    assert!(a.contains("self.update.can_prepare()"));
    assert!(a.contains("self.update.can_restart_and_update()"));
}
#[test]
fn update_transport_is_local_native() {
    let u = read(root().join("src/update.rs"));
    for bad in ["http://", "https://", "reqwest", "webbrowser"] {
        assert!(!u.contains(bad), "forbidden {bad}");
    }
}
