mod archive;
pub use archive::{extract_zip_archive, list_zip_archive, validate_archive_entry};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    pub total_memory_bytes: u64,
    pub available_memory_bytes: u64,
    pub cpu_usage_percent: u8,
    pub gpu_usage_percent: Option<u8>,
    pub vram_total_bytes: Option<u64>,
    pub vram_available_bytes: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub name: String,
    pub executable: Option<PathBuf>,
    pub cpu_usage_percent: u8,
    pub memory_bytes: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    pub title: String,
    pub body: String,
    pub deep_link: Option<String>,
}
pub trait SystemIntegration: Send + Sync {
    fn platform_name(&self) -> &'static str;
    fn data_dir(&self) -> Result<PathBuf, SystemError>;
    fn config_dir(&self) -> Result<PathBuf, SystemError>;
    fn resource_snapshot(&self) -> Result<ResourceSnapshot, SystemError>;
    fn process_snapshot(&self) -> Result<Vec<ProcessSnapshot>, SystemError>;
    fn notify(&self, notification: Notification) -> Result<(), SystemError>;
    fn open_path(&self, path: &Path) -> Result<(), SystemError>;
    fn open_url(&self, url: &str) -> Result<(), SystemError>;
    fn pick_zip_file(&self, initial_dir: &Path) -> Result<Option<PathBuf>, SystemError>;
}
fn validate_external_https_url(url: &str) -> Result<(), SystemError> {
    if url.starts_with("https://") && !url.chars().any(char::is_whitespace) && url.len() <= 2048 {
        Ok(())
    } else {
        Err(SystemError::Platform(
            "external browser URL must use HTTPS".into(),
        ))
    }
}

#[derive(Debug, Error)]
pub enum SystemError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("system integration error: {0}")]
    Platform(String),
}
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;
pub fn current_system_integration() -> Box<dyn SystemIntegration> {
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxSystem)
    }
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsSystem)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        compile_error!("AetherAI v0.2.2 supports Linux and Windows")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_browser_urls_are_https_only() {
        assert!(validate_external_https_url("https://chatgpt.com/").is_ok());
        assert!(validate_external_https_url("https://privacy.openai.com/").is_ok());
        assert!(validate_external_https_url("http://chatgpt.com/").is_err());
        assert!(validate_external_https_url("file:///tmp/not-a-browser-login").is_err());
    }

    #[test]
    fn adapter_reports_platform_and_memory() {
        let s = current_system_integration();
        let r = s.resource_snapshot().unwrap();
        assert!(!s.platform_name().is_empty());
        assert!(r.total_memory_bytes > 0);
        assert!(r.available_memory_bytes <= r.total_memory_bytes);
    }
    #[test]
    fn data_dirs_are_nonempty() {
        let s = current_system_integration();
        assert!(!s.data_dir().unwrap().as_os_str().is_empty());
        assert!(!s.config_dir().unwrap().as_os_str().is_empty());
    }
}
