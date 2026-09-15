use super::BridgeError;
use std::{fs, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProcessSnapshot {
    pub battlenet_running: bool,
    pub agent_running: bool,
    pub diablo_running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildFingerprint {
    pub hash: u64,
    pub byte_len: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SupervisorState {
    #[default]
    Idle,
    Ready,
    Updating,
    Running,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SessionSnapshot {
    pub processes: ProcessSnapshot,
    pub build_fingerprint: Option<BuildFingerprint>,
    pub install_changing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SessionObservation {
    pub snapshot: SessionSnapshot,
    pub state: SupervisorState,
    pub update_settled: bool,
}

#[derive(Debug, Clone)]
pub struct SessionSupervisor {
    last_fingerprint: Option<BuildFingerprint>,
    state: SupervisorState,
    stable_samples: u8,
    settle_samples: u8,
}

impl SessionSupervisor {
    pub fn new(settle_samples: u8) -> Self {
        Self {
            last_fingerprint: None,
            state: SupervisorState::Idle,
            stable_samples: 0,
            settle_samples: settle_samples.max(1),
        }
    }

    pub fn state(&self) -> SupervisorState {
        self.state
    }

    pub fn observe(&mut self, proc_root: &Path, install_path: Option<&Path>) -> SessionObservation {
        let processes = snapshot_processes(proc_root);
        let build_fingerprint = install_path.and_then(|path| build_fingerprint(path).ok());
        let fingerprint_changed = self
            .last_fingerprint
            .zip(build_fingerprint)
            .is_some_and(|(previous, current)| previous != current);
        let marker_present = install_path.is_some_and(update_marker_present);
        let updater_running = processes.battlenet_running || processes.agent_running;
        let raw_install_change = fingerprint_changed || (marker_present && updater_running);
        let was_updating = self.state == SupervisorState::Updating;
        let mut update_settled = false;

        self.state = if processes.diablo_running {
            self.stable_samples = 0;
            SupervisorState::Running
        } else if raw_install_change {
            self.stable_samples = 0;
            SupervisorState::Updating
        } else if was_updating {
            self.stable_samples = self.stable_samples.saturating_add(1);
            if self.stable_samples >= self.settle_samples {
                self.stable_samples = 0;
                update_settled = true;
                if build_fingerprint.is_some() {
                    SupervisorState::Ready
                } else {
                    SupervisorState::Idle
                }
            } else {
                SupervisorState::Updating
            }
        } else if build_fingerprint.is_some() {
            self.stable_samples = 0;
            SupervisorState::Ready
        } else {
            self.stable_samples = 0;
            SupervisorState::Idle
        };

        self.last_fingerprint = build_fingerprint;
        SessionObservation {
            snapshot: SessionSnapshot {
                processes,
                build_fingerprint,
                install_changing: raw_install_change || self.state == SupervisorState::Updating,
            },
            state: self.state,
            update_settled,
        }
    }
}

impl Default for SessionSupervisor {
    fn default() -> Self {
        Self::new(2)
    }
}

pub fn snapshot_processes(proc_root: &Path) -> ProcessSnapshot {
    let mut snapshot = ProcessSnapshot::default();
    let Ok(entries) = fs::read_dir(proc_root) else {
        return snapshot;
    };

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        if file_name.to_string_lossy().parse::<u32>().is_err() {
            continue;
        }
        let process_dir = entry.path();
        let comm = fs::read_to_string(process_dir.join("comm")).unwrap_or_default();
        let cmdline = fs::read(process_dir.join("cmdline")).unwrap_or_default();
        let cmdline = String::from_utf8_lossy(&cmdline).replace('\0', " ");
        let text = format!("{comm} {cmdline}").to_ascii_lowercase();

        snapshot.battlenet_running |= is_battlenet_process(&text);
        snapshot.agent_running |= is_agent_process(&text);
        snapshot.diablo_running |= is_diablo_process(&text);

        if snapshot.battlenet_running && snapshot.agent_running && snapshot.diablo_running {
            break;
        }
    }

    snapshot
}

pub fn build_fingerprint(install_path: &Path) -> Result<BuildFingerprint, BridgeError> {
    let path = install_path.join(".build.info");
    let bytes = fs::read(&path).map_err(|source| BridgeError::Io {
        path: path.clone(),
        source,
    })?;
    let mut hash = 0xcbf29ce484222325_u64;
    hash_bytes(&mut hash, &bytes);
    let mut byte_len = bytes.len() as u64;

    let data_root = install_path.join("Data/data");
    let mut entries = match fs::read_dir(&data_root) {
        Ok(entries) => entries
            .flatten()
            .map(|entry| entry.path())
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };
    entries.sort();
    for entry in entries {
        let Ok(metadata) = fs::metadata(&entry) else {
            continue;
        };
        hash_bytes(&mut hash, entry.as_os_str().as_encoded_bytes());
        hash_bytes(&mut hash, &metadata.len().to_le_bytes());
        byte_len = byte_len.saturating_add(metadata.len());
        if let Ok(modified) = metadata.modified()
            && let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH)
        {
            hash_bytes(&mut hash, &duration.as_nanos().to_le_bytes());
        }
    }

    Ok(BuildFingerprint { hash, byte_len })
}

fn hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = (*hash).wrapping_mul(0x100000001b3);
    }
}

fn is_battlenet_process(text: &str) -> bool {
    text.contains("battle.net.exe") || text.contains("battle.net launcher.exe")
}

fn is_agent_process(text: &str) -> bool {
    text.contains("battle.net update agent")
        || text.contains("/battle.net/agent/")
        || text.contains("\\battle.net\\agent\\")
        || (text.contains("agent.exe") && text.contains("battle.net"))
}

fn is_diablo_process(text: &str) -> bool {
    text.contains("diablo iii.exe") || text.contains("diablo iii64.exe")
}

fn update_marker_present(install_path: &Path) -> bool {
    [
        install_path.join(".build.info.tmp"),
        install_path.join(".build.info.new"),
        install_path.join(".build.info.partial"),
        install_path.join("Data/.patching"),
        install_path.join("Data/.updating"),
    ]
    .into_iter()
    .any(|path| path.exists())
}
