use sanctuary_audio as native_audio;
use sanctuary_casc::parse_build_info;
use sanctuary_core::APP_VERSION;
use sanctuary_install::probe_install;
use sanctuary_render as native_render;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsReport {
    pub opensanctuary_version: String,
    pub os: String,
    pub kernel: String,
    pub gpu_hint: Option<String>,
    pub vulkan_available: bool,
    pub pipewire_available: bool,
    pub install_path: Option<PathBuf>,
    pub install_health: String,
    pub build_key: Option<String>,
    pub index_files: Option<usize>,
    pub trace_id: String,
}

impl DiagnosticsReport {
    pub fn minimal_for_test() -> Self {
        Self {
            opensanctuary_version: APP_VERSION.into(),
            os: "test-linux".into(),
            kernel: "test-kernel".into(),
            gpu_hint: None,
            vulkan_available: false,
            pipewire_available: false,
            install_path: None,
            install_health: "not configured".into(),
            build_key: None,
            index_files: None,
            trace_id: "test-trace".into(),
        }
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn to_text(&self) -> String {
        format!(
            "OpenSanctuary {}\nTrace: {}\nOS: {}\nKernel: {}\nGPU: {}\nVulkan: {}\nPipeWire: {}\nInstall: {}\nHealth: {}\nBuild key: {}\nIndex files: {}\n",
            self.opensanctuary_version,
            self.trace_id,
            self.os,
            self.kernel,
            self.gpu_hint.as_deref().unwrap_or("unknown"),
            readiness(self.vulkan_available),
            readiness(self.pipewire_available),
            self.install_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "not configured".into()),
            self.install_health,
            self.build_key.as_deref().unwrap_or("unknown"),
            self.index_files
                .map(|count| count.to_string())
                .unwrap_or_else(|| "not indexed".into()),
        )
    }
}

pub fn collect(install: Option<&Path>) -> DiagnosticsReport {
    let (install_health, build_key) = match install {
        Some(path) => match probe_install(path) {
            Ok(probe) => match parse_build_info(&probe.build_info) {
                Ok(build) => ("installation metadata readable".into(), build.build_key),
                Err(error) => (error.to_string(), None),
            },
            Err(error) => (error.to_string(), None),
        },
        None => ("not configured".into(), None),
    };

    DiagnosticsReport {
        opensanctuary_version: APP_VERSION.into(),
        os: os_release(),
        kernel: command_stdout("uname", &["-r"]).unwrap_or_else(|| "unknown".into()),
        gpu_hint: gpu_hint(),
        vulkan_available: native_render::detect().available,
        pipewire_available: native_audio::detect().pipewire_available,
        install_path: install.map(Path::to_path_buf),
        install_health,
        build_key,
        index_files: None,
        trace_id: trace_id(),
    }
}

fn readiness(ready: bool) -> &'static str {
    if ready { "ready" } else { "unavailable" }
}

fn os_release() -> String {
    let text = fs::read_to_string("/etc/os-release").unwrap_or_default();
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
            return value.trim_matches('"').to_string();
        }
    }
    "Linux".into()
}

fn gpu_hint() -> Option<String> {
    for card in [
        "/sys/class/drm/card0/device/vendor",
        "/sys/class/drm/card1/device/vendor",
    ] {
        if let Ok(vendor) = fs::read_to_string(card) {
            let name = match vendor.trim() {
                "0x1002" => "AMD GPU",
                "0x10de" => "NVIDIA GPU",
                "0x8086" => "Intel GPU",
                other => other,
            };
            return Some(name.to_string());
        }
    }
    command_stdout(
        "sh",
        &[
            "-c",
            "command -v lspci >/dev/null && lspci | grep -Ei 'VGA|3D' | head -1",
        ],
    )
}

fn command_stdout(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn trace_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("os-{:x}-{:x}", std::process::id(), nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_exports_json_without_credentials() {
        let report = DiagnosticsReport::minimal_for_test();
        let json = report.to_json().unwrap();
        assert!(json.contains("opensanctuary_version"));
        assert!(!json.to_ascii_lowercase().contains("password"));
    }

    #[test]
    fn text_export_contains_native_capabilities() {
        let report = DiagnosticsReport::minimal_for_test();
        let text = report.to_text();
        assert!(text.contains("Vulkan:"));
        assert!(text.contains("PipeWire:"));
    }
}
