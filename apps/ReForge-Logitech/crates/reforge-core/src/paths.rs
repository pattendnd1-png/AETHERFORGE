use std::{env, fs, path::PathBuf};

pub fn config_dir_from(xdg_config_home: Option<&str>, home: Option<&str>) -> PathBuf {
    if let Some(xdg) = xdg_config_home.filter(|value| !value.is_empty()) {
        return PathBuf::from(xdg).join("reforge-logitech");
    }
    if let Some(home) = home.filter(|value| !value.is_empty()) {
        return PathBuf::from(home).join(".config/reforge-logitech");
    }
    PathBuf::from(".reforge-logitech")
}

pub fn config_dir() -> PathBuf {
    config_dir_from(
        env::var("XDG_CONFIG_HOME").ok().as_deref(),
        env::var("HOME").ok().as_deref(),
    )
}

fn effective_uid() -> u32 {
    fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|contents| {
            contents.lines().find_map(|line| {
                let values = line.strip_prefix("Uid:")?;
                values.split_whitespace().next()?.parse::<u32>().ok()
            })
        })
        .unwrap_or(0)
}

pub fn runtime_dir() -> PathBuf {
    if let Ok(value) = env::var("XDG_RUNTIME_DIR")
        && !value.is_empty()
    {
        return PathBuf::from(value).join("reforge-logitech");
    }
    PathBuf::from(format!("/tmp/reforge-logitech-{}", effective_uid()))
}

pub fn runtime_socket() -> PathBuf {
    runtime_dir().join("reforge.sock")
}

pub fn profile_file() -> PathBuf {
    config_dir().join("profiles.json")
}

pub fn ui_preferences_file() -> PathBuf {
    config_dir().join("ui.json")
}

pub fn diagnostic_export_dir() -> PathBuf {
    config_dir().join("diagnostics")
}
