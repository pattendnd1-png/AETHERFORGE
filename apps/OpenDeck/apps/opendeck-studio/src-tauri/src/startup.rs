use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;

static NATIVE_VISIBLE: AtomicBool = AtomicBool::new(false);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StartupAck {
    release: &'static str,
    pid: u32,
    native_visible: bool,
    frontend_visible: bool,
}

fn probe_path() -> Option<PathBuf> {
    std::env::var_os("OPENDECK_STARTUP_PROBE_FILE").map(PathBuf::from)
}

fn write_probe(frontend_visible: bool) -> Result<(), String> {
    let Some(path) = probe_path() else {
        return Ok(());
    };
    let body = StartupAck {
        release: "2.0.41",
        pid: std::process::id(),
        native_visible: NATIVE_VISIBLE.load(Ordering::SeqCst),
        frontend_visible,
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(&body).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| error.to_string())
}

pub(crate) fn prepare_main_window(app: &tauri::App) -> Result<(), String> {
    if crate::qualification::enabled() {
        return Ok(());
    }
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "OpenDeck main window is missing".to_string())?;
    window.show().map_err(|error| error.to_string())?;
    write_probe(false)
}

#[tauri::command]
pub(crate) fn startup_visible_ack(app: tauri::AppHandle) -> Result<bool, String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "OpenDeck main window is missing".to_string())?;
    let visible = window.is_visible().map_err(|error| error.to_string())?;
    if !visible {
        return Err("OpenDeck main window is not visible after frontend bootstrap".into());
    }
    NATIVE_VISIBLE.store(true, Ordering::SeqCst);
    write_probe(true)?;
    Ok(true)
}
