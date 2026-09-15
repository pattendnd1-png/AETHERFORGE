use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

pub const APP_NAME: &str = "OpenSanctuary";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InstallState {
    #[default]
    NotConfigured,
    Searching,
    FoundUnindexed,
    Indexing,
    Ready,
    NeedsRepair,
    UnsupportedBuild,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallHealth {
    pub state: InstallState,
    pub detail: String,
}

impl InstallHealth {
    pub fn new(state: InstallState, detail: impl Into<String>) -> Self {
        Self {
            state,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RuntimeCapabilities {
    pub vulkan_available: bool,
    pub pipewire_available: bool,
    pub gpu_hint: Option<String>,
    pub audio_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub install_path: Option<PathBuf>,
    pub reduced_motion: bool,
    pub selected_game: String,
    pub show_desktop_notifications: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            install_path: None,
            reduced_motion: false,
            selected_game: "diablo-iii".into(),
            show_desktop_notifications: true,
        }
    }
}

impl AppConfig {
    pub fn load_from(path: &Path) -> Result<Self, SanctuaryError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path).map_err(|source| SanctuaryError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        toml::from_str(&text).map_err(|source| SanctuaryError::ConfigDecode(source.to_string()))
    }

    pub fn save_to(&self, path: &Path) -> Result<(), SanctuaryError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| SanctuaryError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let text = toml::to_string_pretty(self)
            .map_err(|source| SanctuaryError::ConfigEncode(source.to_string()))?;
        fs::write(path, text).map_err(|source| SanctuaryError::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}

pub fn xdg_config_path() -> PathBuf {
    if let Some(root) = env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(root).join("opensanctuary/config.toml");
    }
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".config/opensanctuary/config.toml")
}

pub fn xdg_cache_dir() -> PathBuf {
    if let Some(root) = env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(root).join("opensanctuary");
    }
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".cache/opensanctuary")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchDescriptor {
    pub schema_version: u32,
    pub install_path: PathBuf,
    pub created_by: String,
}

impl LaunchDescriptor {
    pub fn new(install_path: PathBuf) -> Self {
        Self {
            schema_version: 1,
            install_path,
            created_by: format!("{APP_NAME} {APP_VERSION}"),
        }
    }

    pub fn validate(&self) -> Result<(), SanctuaryError> {
        if self.schema_version != 1 {
            return Err(SanctuaryError::InvalidLaunchDescriptor(format!(
                "unsupported schema {}",
                self.schema_version
            )));
        }
        if !self.install_path.is_dir() {
            return Err(SanctuaryError::InstallationNotFound(
                self.install_path.clone(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskEvent {
    Started { label: String },
    Progress { label: String, fraction: f32 },
    Finished { label: String },
    Failed { label: String, message: String },
}

#[derive(Debug, Error)]
pub enum SanctuaryError {
    #[error("Diablo III installation was not found at {0}")]
    InstallationNotFound(PathBuf),
    #[error("content store is unreadable: {0}")]
    ContentStoreUnreadable(String),
    #[error("unsupported Diablo III build: {0}")]
    UnsupportedBuild(String),
    #[error("Vulkan is unavailable: {0}")]
    VulkanUnavailable(String),
    #[error("invalid launch descriptor: {0}")]
    InvalidLaunchDescriptor(String),
    #[error("configuration decode failed: {0}")]
    ConfigDecode(String),
    #[error("configuration encode failed: {0}")]
    ConfigEncode(String),
    #[error("I/O failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("engine launch failed: {0}")]
    EngineLaunch(String),
    #[error("diagnostics export failed: {0}")]
    Diagnostics(String),
    #[error("content inventory cache failed: {0}")]
    Cache(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let cfg = AppConfig {
            reduced_motion: true,
            ..Default::default()
        };
        cfg.save_to(&path).unwrap();
        assert_eq!(AppConfig::load_from(&path).unwrap(), cfg);
    }

    #[test]
    fn descriptor_rejects_nonexistent_install() {
        let descriptor = LaunchDescriptor::new(PathBuf::from("/definitely/missing"));
        assert!(matches!(
            descriptor.validate(),
            Err(SanctuaryError::InstallationNotFound(_))
        ));
    }

    #[test]
    fn absent_config_uses_safe_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = AppConfig::load_from(&dir.path().join("missing.toml")).unwrap();
        assert_eq!(cfg.selected_game, "diablo-iii");
        assert!(cfg.install_path.is_none());
    }
}
