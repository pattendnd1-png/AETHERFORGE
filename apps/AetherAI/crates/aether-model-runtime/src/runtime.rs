use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use aether_core::NetworkPolicy;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::download::sha256_file;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeAsset {
    pub id: String,
    pub platform: String,
    pub upstream: String,
    pub revision: String,
    pub commit: String,
    pub accelerator: String,
    pub archive: String,
    pub download_url: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub server_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeManifest {
    pub version: u32,
    pub runtimes: Vec<RuntimeAsset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeInstall {
    pub asset_id: String,
    pub executable: PathBuf,
    pub install_dir: PathBuf,
}

#[derive(Debug, Error)]
pub enum RuntimeInstallError {
    #[error("network is disabled")]
    NetworkBlocked,
    #[error("network approval is required")]
    NetworkApprovalRequired,
    #[error("runtime manifest JSON is invalid: {0}")]
    Manifest(#[from] serde_json::Error),
    #[error("no pinned runtime exists for {0}")]
    UnsupportedPlatform(String),
    #[error("runtime command failed: {0}")]
    Command(String),
    #[error("runtime I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("runtime checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    #[error("runtime archive did not contain {0}")]
    MissingServer(String),
}

impl RuntimeManifest {
    pub fn parse(json: &str) -> Result<Self, RuntimeInstallError> {
        let manifest: Self = serde_json::from_str(json)?;
        Ok(manifest)
    }

    pub fn for_current_platform(&self) -> Result<&RuntimeAsset, RuntimeInstallError> {
        let platform = current_platform_key();
        self.runtimes
            .iter()
            .find(|runtime| runtime.platform == platform)
            .ok_or_else(|| RuntimeInstallError::UnsupportedPlatform(platform.to_string()))
    }
}

pub fn current_platform_key() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x86_64"
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "windows-x86_64"
    }
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    {
        "unsupported"
    }
}

pub fn stable_runtime_path(runtime_root: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        runtime_root.join("aetherai-gguf-runtime.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        runtime_root.join("aetherai-gguf-runtime")
    }
}

pub fn install_gguf_runtime(
    manifest: &RuntimeManifest,
    runtime_root: &Path,
    policy: NetworkPolicy,
) -> Result<RuntimeInstall, RuntimeInstallError> {
    match policy {
        NetworkPolicy::Off => return Err(RuntimeInstallError::NetworkBlocked),
        NetworkPolicy::Ask => return Err(RuntimeInstallError::NetworkApprovalRequired),
        NetworkPolicy::On => {}
    }
    let asset = manifest.for_current_platform()?.clone();
    fs::create_dir_all(runtime_root)?;
    let stable = stable_runtime_path(runtime_root);
    let install_dir = runtime_root.join(&asset.id);
    if stable.is_file() && install_dir.is_dir() {
        return Ok(RuntimeInstall {
            asset_id: asset.id,
            executable: stable,
            install_dir,
        });
    }

    let downloads = runtime_root.join("downloads");
    fs::create_dir_all(&downloads)?;
    let archive = downloads.join(&asset.archive);
    let part = downloads.join(format!("{}.part", asset.archive));
    if !archive.is_file() || !sha256_file(&archive)?.eq_ignore_ascii_case(&asset.sha256) {
        let status = Command::new(curl_command())
            .args([
                "--fail",
                "--location",
                "--retry",
                "3",
                "--continue-at",
                "-",
                "--output",
            ])
            .arg(&part)
            .arg(&asset.download_url)
            .status()
            .map_err(|error| RuntimeInstallError::Command(error.to_string()))?;
        if !status.success() {
            return Err(RuntimeInstallError::Command(format!(
                "curl exited with {status}"
            )));
        }
        let actual = sha256_file(&part)?;
        if !actual.eq_ignore_ascii_case(&asset.sha256) {
            let _ = fs::remove_file(&part);
            return Err(RuntimeInstallError::ChecksumMismatch {
                expected: asset.sha256,
                actual,
            });
        }
        fs::rename(&part, &archive)?;
    }

    if install_dir.exists() {
        fs::remove_dir_all(&install_dir)?;
    }
    fs::create_dir_all(&install_dir)?;
    extract_archive(&archive, &install_dir)?;
    let server = find_named_file(&install_dir, &asset.server_name)?
        .ok_or_else(|| RuntimeInstallError::MissingServer(asset.server_name.clone()))?;
    normalize_runtime(&server, &install_dir, &stable)?;
    Ok(RuntimeInstall {
        asset_id: asset.id,
        executable: stable,
        install_dir,
    })
}

#[cfg(target_os = "windows")]
fn curl_command() -> &'static str {
    "curl.exe"
}
#[cfg(not(target_os = "windows"))]
fn curl_command() -> &'static str {
    "curl"
}

fn extract_archive(archive: &Path, destination: &Path) -> Result<(), RuntimeInstallError> {
    #[cfg(target_os = "windows")]
    {
        let archive = archive
            .to_str()
            .ok_or_else(|| RuntimeInstallError::Command("non-UTF-8 runtime archive path".into()))?;
        let destination = destination.to_str().ok_or_else(|| {
            RuntimeInstallError::Command("non-UTF-8 runtime destination path".into())
        })?;
        let script = format!(
            "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
            archive.replace('\'', "''"),
            destination.replace('\'', "''")
        );
        let status = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .status()
            .map_err(|error| RuntimeInstallError::Command(error.to_string()))?;
        if !status.success() {
            return Err(RuntimeInstallError::Command(format!(
                "PowerShell Expand-Archive exited with {status}"
            )));
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let status = Command::new("tar")
            .arg("-xzf")
            .arg(archive)
            .arg("-C")
            .arg(destination)
            .status()
            .map_err(|error| RuntimeInstallError::Command(error.to_string()))?;
        if !status.success() {
            return Err(RuntimeInstallError::Command(format!(
                "tar exited with {status}"
            )));
        }
    }
    Ok(())
}

fn find_named_file(root: &Path, name: &str) -> io::Result<Option<PathBuf>> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case(name))
            {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
}

#[cfg(not(target_os = "windows"))]
fn normalize_runtime(
    server: &Path,
    install_dir: &Path,
    stable: &Path,
) -> Result<(), RuntimeInstallError> {
    let mut library_dirs = Vec::new();
    collect_library_dirs(install_dir, &mut library_dirs)?;
    library_dirs.sort();
    library_dirs.dedup();
    let library_path = library_dirs
        .iter()
        .map(|path| shell_quote(path))
        .collect::<Vec<_>>()
        .join(":");
    let mut file = File::create(stable)?;
    writeln!(file, "#!/usr/bin/env bash")?;
    writeln!(file, "set -euo pipefail")?;
    if !library_path.is_empty() {
        writeln!(
            file,
            "export LD_LIBRARY_PATH={library_path}${{LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}}"
        )?;
    }
    writeln!(file, "exec {} \"$@\"", shell_quote(server))?;
    drop(file);
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(stable)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(stable, permissions)?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn collect_library_dirs(root: &Path, output: &mut Vec<PathBuf>) -> io::Result<()> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.contains(".so"))
            {
                if let Some(parent) = path.parent() {
                    output.push(parent.to_path_buf());
                }
            }
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn shell_quote(path: &Path) -> String {
    let text = path.to_string_lossy();
    format!("'{}'", text.replace('\'', "'\\''"))
}

#[cfg(target_os = "windows")]
fn normalize_runtime(
    server: &Path,
    install_dir: &Path,
    stable: &Path,
) -> Result<(), RuntimeInstallError> {
    fs::copy(server, stable)?;
    copy_dlls(install_dir, stable.parent().unwrap_or(install_dir))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn copy_dlls(root: &Path, destination: &Path) -> io::Result<()> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("dll"))
            {
                if let Some(name) = path.file_name() {
                    fs::copy(&path, destination.join(name))?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn manifest() -> RuntimeManifest {
        RuntimeManifest::parse(include_str!(
            "../../../resources/inference/runtime-manifest.json"
        ))
        .unwrap()
    }

    #[test]
    fn current_platform_has_a_pinned_runtime() {
        let manifest = manifest();
        let asset = manifest.for_current_platform().unwrap();
        assert_eq!(asset.revision, "b10649");
        assert_eq!(asset.accelerator, "vulkan");
        assert_eq!(asset.sha256.len(), 64);
        assert!(
            asset
                .download_url
                .starts_with("https://github.com/ggml-org/llama.cpp/")
        );
    }

    #[test]
    fn ask_blocks_runtime_install_before_network_io() {
        let dir = tempdir().unwrap();
        let result = install_gguf_runtime(&manifest(), dir.path(), NetworkPolicy::Ask);
        assert!(matches!(
            result,
            Err(RuntimeInstallError::NetworkApprovalRequired)
        ));
    }

    #[test]
    fn off_blocks_runtime_install_before_network_io() {
        let dir = tempdir().unwrap();
        let result = install_gguf_runtime(&manifest(), dir.path(), NetworkPolicy::Off);
        assert!(matches!(result, Err(RuntimeInstallError::NetworkBlocked)));
    }
}
