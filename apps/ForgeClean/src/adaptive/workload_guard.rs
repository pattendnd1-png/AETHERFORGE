use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProtectedReason {
    Streaming,
    Gaming,
    Build,
    Benchmark,
}

pub trait ProtectionState {
    fn protected_reason(&self) -> Option<ProtectedReason>;
}

#[derive(Debug, Clone, Default)]
pub struct WorkloadGuard {
    forced: Option<ProtectedReason>,
}

impl WorkloadGuard {
    pub fn with_forced_reason(reason: ProtectedReason) -> Self {
        Self {
            forced: Some(reason),
        }
    }

    fn scan_proc(&self) -> Option<ProtectedReason> {
        let Ok(entries) = fs::read_dir("/proc") else {
            return None;
        };
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name();
            if !name.to_string_lossy().bytes().all(|b| b.is_ascii_digit()) {
                continue;
            }
            let dir = entry.path();
            let comm = fs::read_to_string(dir.join("comm"))
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();
            let cmdline = fs::read(dir.join("cmdline"))
                .map(|b| {
                    String::from_utf8_lossy(&b)
                        .replace('\0', " ")
                        .to_ascii_lowercase()
                })
                .unwrap_or_default();

            if comm == "obs" || comm == "obs-studio" || cmdline.contains("/obs-studio") {
                return Some(ProtectedReason::Streaming);
            }
            if comm == "gamescope" || cmdline.contains("/steamapps/common/") {
                return Some(ProtectedReason::Gaming);
            }
            if comm == "cargo" || comm == "rustc" || comm == "ninja" || comm == "make" {
                return Some(ProtectedReason::Build);
            }
        }
        None
    }

    fn lock_reason(&self) -> Option<ProtectedReason> {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")?;
        let root = Path::new(&runtime).join("aetherforge");
        if root.join("benchmark.lock").exists() {
            return Some(ProtectedReason::Benchmark);
        }
        if root.join("build.lock").exists() {
            return Some(ProtectedReason::Build);
        }
        None
    }
}

impl ProtectionState for WorkloadGuard {
    fn protected_reason(&self) -> Option<ProtectedReason> {
        self.forced
            .or_else(|| self.lock_reason())
            .or_else(|| self.scan_proc())
    }
}
