use serde::{Deserialize, Serialize};
use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Child, Command},
    thread,
    time::Duration,
};
use thiserror::Error;

mod account;
mod health;
mod network;
mod supervisor;
mod window_host;
pub use account::{
    AccountError, AccountProfile, DesktopSessionState, OAUTH_CLIENT_ID_ENV, OFFICIAL_ACCOUNT_URL,
    account_profile_path, account_state_from_processes, default_account_profile_path,
    load_account_profile, oauth_client_id, official_account_url, save_account_profile,
};
pub use health::{
    BridgeHealthReport, BridgeHealthState, FailureTracker, RecoveryOutcome, evaluate_bridge_health,
    quarantine_bridge_record, recover_bridge,
};
pub use network::{
    NetworkBridgeReport, NetworkBridgeState, NetworkCheck, NetworkCheckKind, network_probe_hosts,
    probe_network_bridge,
};
pub use supervisor::{
    BuildFingerprint, ProcessSnapshot, SessionObservation, SessionSnapshot, SessionSupervisor,
    SupervisorState, build_fingerprint, snapshot_processes,
};
pub use window_host::{
    BattleNetWindowHost, HostRect, HostStatus, WindowHostMode, detect_window_host_mode,
    window_matches_battlenet,
};

pub const OFFICIAL_DOWNLOAD_URL: &str = "https://download.battle.net/en-us/desktop";

const BRIDGE_RECORD_SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BridgeRecord {
    pub schema_version: u32,
    pub launcher_exe: PathBuf,
    pub runner: PathBuf,
    pub wine_prefix: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diablo_install: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diablo_exe: Option<PathBuf>,
}

impl BridgeRecord {
    pub fn from_discovery(discovery: &BridgeDiscovery) -> Option<Self> {
        let launcher_exe = discovery.launcher_exe.clone()?;
        let runner = discovery.runner.clone()?;
        let wine_prefix = discovery.wine_prefix.clone()?;
        let diablo_install = discovery
            .diablo_install
            .as_ref()
            .filter(|path| looks_like_diablo_install(path))
            .cloned();
        let diablo_exe = diablo_install.as_deref().and_then(|install| {
            discovery
                .diablo_exe
                .as_ref()
                .filter(|path| path.is_file())
                .cloned()
                .or_else(|| find_diablo_executable(install))
        });

        Some(Self {
            schema_version: BRIDGE_RECORD_SCHEMA,
            launcher_exe,
            runner,
            wine_prefix,
            diablo_install,
            diablo_exe,
        })
    }

    pub fn validated(&self) -> Result<BridgeDiscovery, BridgeError> {
        if self.schema_version != BRIDGE_RECORD_SCHEMA {
            return Err(BridgeError::UnsupportedRecordSchema(self.schema_version));
        }
        require_file(&self.runner, "Wine runner")?;
        require_dir(&self.wine_prefix, "Wine prefix")?;
        require_file(&self.launcher_exe, "Battle.net launcher")?;

        let diablo_install = self
            .diablo_install
            .as_ref()
            .filter(|path| looks_like_diablo_install(path))
            .cloned();
        let diablo_exe = diablo_install.as_deref().and_then(|install| {
            self.diablo_exe
                .as_ref()
                .filter(|path| path.is_file())
                .cloned()
                .or_else(|| find_diablo_executable(install))
        });

        Ok(BridgeDiscovery {
            launcher_exe: Some(self.launcher_exe.clone()),
            runner: Some(self.runner.clone()),
            wine_prefix: Some(self.wine_prefix.clone()),
            diablo_install,
            diablo_exe,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    attempts: u8,
    delay: Duration,
}

impl RetryPolicy {
    pub fn new(attempts: u8, delay: Duration) -> Self {
        Self {
            attempts: attempts.max(1),
            delay,
        }
    }

    pub fn attempts(self) -> u8 {
        self.attempts
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(3, Duration::from_millis(600))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BridgeDiscovery {
    pub launcher_exe: Option<PathBuf>,
    pub runner: Option<PathBuf>,
    pub wine_prefix: Option<PathBuf>,
    pub diablo_install: Option<PathBuf>,
    pub diablo_exe: Option<PathBuf>,
}

impl BridgeDiscovery {
    pub fn battlenet_ready(&self) -> bool {
        self.launcher_exe.is_some() && self.runner.is_some() && self.wine_prefix.is_some()
    }

    pub fn official_game_ready(&self) -> bool {
        self.battlenet_ready() && self.diablo_exe.is_some()
    }

    pub fn official_game_ready_for(&self, install_path: &Path) -> bool {
        self.battlenet_ready()
            && (self.diablo_exe.as_ref().is_some_and(|path| path.is_file())
                || find_diablo_executable(install_path).is_some())
    }

    pub fn runtime_prefix(&self) -> Option<&Path> {
        self.wine_prefix.as_deref()
    }
}

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("Battle.net is not installed in a discovered Wine prefix")]
    MissingLauncher,
    #[error(
        "no Wine runner was found; set OPENSANCTUARY_WINE to the Wine binary used for Battle.net"
    )]
    MissingRunner,
    #[error("the Battle.net Wine prefix could not be determined")]
    MissingPrefix,
    #[error("Diablo III.exe was not found under {0}")]
    MissingDiabloExecutable(PathBuf),
    #[error("unsupported Battle.net bridge record schema {0}")]
    UnsupportedRecordSchema(u32),
    #[error("saved Battle.net bridge record is stale: {0}")]
    StaleRecord(String),
    #[error("failed to decode Battle.net bridge record: {0}")]
    RecordDecode(String),
    #[error("failed to encode Battle.net bridge record: {0}")]
    RecordEncode(String),
    #[error("bridge record I/O failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to start {program}: {source}")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },
}

impl BridgeError {
    fn is_retryable(&self) -> bool {
        matches!(self, Self::Spawn { .. })
    }
}

pub fn bridge_record_path(home: &Path, xdg_config_home: Option<&OsStr>) -> PathBuf {
    xdg_config_home.map_or_else(
        || home.join(".config/opensanctuary/battlenet.toml"),
        |root| PathBuf::from(root).join("opensanctuary/battlenet.toml"),
    )
}

pub fn default_bridge_record_path() -> PathBuf {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    bridge_record_path(&home, env::var_os("XDG_CONFIG_HOME").as_deref())
}

pub fn load_bridge_record(path: &Path) -> Result<Option<BridgeRecord>, BridgeError> {
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).map_err(|source| BridgeError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&text)
        .map(Some)
        .map_err(|error| BridgeError::RecordDecode(error.to_string()))
}

pub fn save_bridge_record(path: &Path, record: &BridgeRecord) -> Result<(), BridgeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| BridgeError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let text = toml::to_string_pretty(record)
        .map_err(|error| BridgeError::RecordEncode(error.to_string()))?;
    let temporary = path.with_extension("toml.tmp");
    fs::write(&temporary, text).map_err(|source| BridgeError::Io {
        path: temporary.clone(),
        source,
    })?;
    fs::rename(&temporary, path).map_err(|source| BridgeError::Io {
        path: path.to_path_buf(),
        source,
    })
}

pub fn clear_bridge_record(path: &Path) -> Result<(), BridgeError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(BridgeError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

pub fn merge_saved_discovery(
    record: Option<&BridgeRecord>,
    mut discovered: BridgeDiscovery,
) -> BridgeDiscovery {
    let Some(record) = record.filter(|record| record.schema_version == BRIDGE_RECORD_SCHEMA) else {
        return discovered;
    };

    if record.runner.is_file() {
        discovered.runner = Some(record.runner.clone());
    }
    if record.wine_prefix.is_dir() {
        discovered.wine_prefix = Some(record.wine_prefix.clone());
    }
    if record.launcher_exe.is_file() {
        discovered.launcher_exe = Some(record.launcher_exe.clone());
    }
    if let Some(install) = record
        .diablo_install
        .as_ref()
        .filter(|path| looks_like_diablo_install(path))
    {
        discovered.diablo_install = Some(install.clone());
        discovered.diablo_exe = record
            .diablo_exe
            .as_ref()
            .filter(|path| path.is_file())
            .cloned()
            .or_else(|| find_diablo_executable(install));
    }
    discovered
}

pub fn bridge_record_for_install(
    discovery: &BridgeDiscovery,
    install_path: &Path,
) -> Option<BridgeRecord> {
    let mut complete = discovery.clone();
    complete.diablo_install = Some(install_path.to_path_buf());
    complete.diablo_exe = find_diablo_executable(install_path);
    BridgeRecord::from_discovery(&complete)
}

pub fn discover() -> BridgeDiscovery {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    discover_for_home(&home)
}

pub fn discover_for_home(home: &Path) -> BridgeDiscovery {
    discover_with_environment(
        home,
        env::var_os("PATH").as_deref(),
        env::var_os("OPENSANCTUARY_BATTLENET_EXE").as_deref(),
        env::var_os("OPENSANCTUARY_WINE").as_deref(),
        env::var_os("WINEPREFIX").as_deref(),
    )
}

pub fn discover_for_home_with_record(
    home: &Path,
    record: Option<&BridgeRecord>,
) -> BridgeDiscovery {
    let mut discovery = merge_saved_discovery(record, discover_for_home(home));
    if discovery
        .diablo_install
        .as_deref()
        .is_some_and(looks_like_diablo_install)
        && discovery
            .diablo_exe
            .as_ref()
            .is_some_and(|path| path.is_file())
    {
        return discovery;
    }

    if let Some(path) = find_diablo_candidates(home, &discovery).into_iter().next() {
        discovery.diablo_install = Some(path.clone());
        discovery.diablo_exe = find_diablo_executable(&path);
    }
    discovery
}

pub fn discover_with_environment(
    home: &Path,
    path_env: Option<&OsStr>,
    launcher_override: Option<&OsStr>,
    runner_override: Option<&OsStr>,
    prefix_override: Option<&OsStr>,
) -> BridgeDiscovery {
    let explicit_launcher = launcher_override
        .map(PathBuf::from)
        .filter(|path| path.is_file());
    let explicit_runner = runner_override
        .map(PathBuf::from)
        .filter(|path| path.is_file());
    let explicit_prefix = prefix_override
        .map(PathBuf::from)
        .filter(|path| path.is_dir());

    let mut prefixes = candidate_prefixes(home, explicit_prefix.as_deref());
    if let Some(prefix) = explicit_launcher
        .as_deref()
        .and_then(prefix_for_windows_path)
    {
        push_unique(&mut prefixes, prefix);
    }

    let launcher_exe = explicit_launcher.or_else(|| {
        prefixes
            .iter()
            .find_map(|prefix| find_battlenet_launcher(prefix))
    });
    let wine_prefix = explicit_prefix
        .or_else(|| launcher_exe.as_deref().and_then(prefix_for_windows_path))
        .or_else(|| {
            prefixes
                .iter()
                .find(|prefix| find_battlenet_launcher(prefix).is_some())
                .cloned()
        });
    let runner = explicit_runner.or_else(|| find_runner_in_path(path_env));

    let mut discovery = BridgeDiscovery {
        launcher_exe,
        runner,
        wine_prefix,
        diablo_install: None,
        diablo_exe: None,
    };
    let candidates = find_diablo_candidates(home, &discovery);
    discovery.diablo_install = candidates.first().cloned();
    discovery.diablo_exe = discovery
        .diablo_install
        .as_deref()
        .and_then(find_diablo_executable);
    discovery
}

pub fn find_diablo_candidates(home: &Path, discovery: &BridgeDiscovery) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = &discovery.diablo_install {
        push_unique(&mut candidates, path.clone());
    }

    for path in [
        home.join("Games/Diablo III"),
        home.join("games/Diablo III"),
        home.join(".local/share/opensanctuary/imports/Diablo III"),
    ] {
        if looks_like_diablo_install(&path) {
            push_unique(&mut candidates, path);
        }
    }

    let mut prefixes = candidate_prefixes(home, discovery.wine_prefix.as_deref());
    if let Some(prefix) = discovery
        .launcher_exe
        .as_deref()
        .and_then(prefix_for_windows_path)
    {
        push_unique(&mut prefixes, prefix);
    }
    for prefix in prefixes {
        for path in diablo_paths_in_prefix(&prefix) {
            if looks_like_diablo_install(&path) {
                push_unique(&mut candidates, path);
            }
        }
    }
    candidates
}

pub fn launch_battlenet(discovery: &BridgeDiscovery) -> Result<Child, BridgeError> {
    let launcher = discovery
        .launcher_exe
        .as_deref()
        .ok_or(BridgeError::MissingLauncher)?;
    let runner = discovery
        .runner
        .as_deref()
        .ok_or(BridgeError::MissingRunner)?;
    let prefix = discovery
        .wine_prefix
        .as_deref()
        .ok_or(BridgeError::MissingPrefix)?;

    Command::new(runner)
        .env("WINEPREFIX", prefix)
        .arg(launcher)
        .spawn()
        .map_err(|source| BridgeError::Spawn {
            program: runner.display().to_string(),
            source,
        })
}

pub fn launch_battlenet_with_retry(
    discovery: &BridgeDiscovery,
    policy: RetryPolicy,
) -> Result<Child, BridgeError> {
    retry_spawn(policy, || launch_battlenet(discovery))
}

pub fn launch_diablo(
    discovery: &BridgeDiscovery,
    install_path: &Path,
) -> Result<Child, BridgeError> {
    let runner = discovery
        .runner
        .as_deref()
        .ok_or(BridgeError::MissingRunner)?;
    let prefix = discovery
        .wine_prefix
        .as_deref()
        .ok_or(BridgeError::MissingPrefix)?;
    let executable = discovery
        .diablo_exe
        .as_deref()
        .filter(|path| path.is_file())
        .map(Path::to_path_buf)
        .or_else(|| find_diablo_executable(install_path))
        .ok_or_else(|| BridgeError::MissingDiabloExecutable(install_path.to_path_buf()))?;

    Command::new(runner)
        .env("WINEPREFIX", prefix)
        .arg(&executable)
        .current_dir(install_path)
        .spawn()
        .map_err(|source| BridgeError::Spawn {
            program: executable.display().to_string(),
            source,
        })
}

pub fn launch_diablo_with_retry(
    discovery: &BridgeDiscovery,
    install_path: &Path,
    policy: RetryPolicy,
) -> Result<Child, BridgeError> {
    retry_spawn(policy, || launch_diablo(discovery, install_path))
}

pub fn open_url(url: &str) -> Result<Child, BridgeError> {
    Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map_err(|source| BridgeError::Spawn {
            program: "xdg-open".into(),
            source,
        })
}

pub fn open_official_download() -> Result<Child, BridgeError> {
    open_url(OFFICIAL_DOWNLOAD_URL)
}

pub fn open_official_account() -> Result<Child, BridgeError> {
    open_url(official_account_url())
}

fn retry_spawn<F>(policy: RetryPolicy, mut spawn: F) -> Result<Child, BridgeError>
where
    F: FnMut() -> Result<Child, BridgeError>,
{
    let attempts = policy.attempts.max(1);
    for attempt in 1..=attempts {
        match spawn() {
            Ok(child) => return Ok(child),
            Err(error) if error.is_retryable() && attempt < attempts => {
                thread::sleep(policy.delay);
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("retry loop always returns on its final attempt")
}

fn require_file(path: &Path, label: &str) -> Result<(), BridgeError> {
    if path.is_file() {
        Ok(())
    } else {
        Err(BridgeError::StaleRecord(format!(
            "{label} is missing at {}",
            path.display()
        )))
    }
}

fn require_dir(path: &Path, label: &str) -> Result<(), BridgeError> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(BridgeError::StaleRecord(format!(
            "{label} is missing at {}",
            path.display()
        )))
    }
}

fn candidate_prefixes(home: &Path, explicit: Option<&Path>) -> Vec<PathBuf> {
    let mut prefixes = Vec::new();
    if let Some(prefix) = explicit.filter(|path| path.is_dir()) {
        push_unique(&mut prefixes, prefix.to_path_buf());
    }
    for prefix in [
        home.join(".wine"),
        home.join("Games/battlenet"),
        home.join("Games/battle-net"),
        home.join("games/battlenet"),
    ] {
        if prefix.is_dir() {
            push_unique(&mut prefixes, prefix);
        }
    }
    append_child_directories(&mut prefixes, &home.join(".local/share/bottles/bottles"));
    append_child_directories(
        &mut prefixes,
        &home.join(".var/app/com.usebottles.bottles/data/bottles/bottles"),
    );
    prefixes
}

fn append_child_directories(output: &mut Vec<PathBuf>, root: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            push_unique(output, path);
        }
    }
}

fn find_runner_in_path(path_env: Option<&OsStr>) -> Option<PathBuf> {
    let path_env = path_env?;
    for directory in env::split_paths(path_env) {
        for name in ["wine", "wine64"] {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn find_battlenet_launcher(prefix: &Path) -> Option<PathBuf> {
    battlenet_paths_in_prefix(prefix)
        .into_iter()
        .find(|path| path.is_file())
}

fn battlenet_paths_in_prefix(prefix: &Path) -> [PathBuf; 4] {
    [
        prefix.join("drive_c/Program Files (x86)/Battle.net/Battle.net Launcher.exe"),
        prefix.join("drive_c/Program Files (x86)/Battle.net/Battle.net.exe"),
        prefix.join("drive_c/Program Files/Battle.net/Battle.net Launcher.exe"),
        prefix.join("drive_c/Program Files/Battle.net/Battle.net.exe"),
    ]
}

fn diablo_paths_in_prefix(prefix: &Path) -> [PathBuf; 2] {
    [
        prefix.join("drive_c/Program Files (x86)/Diablo III"),
        prefix.join("drive_c/Program Files/Diablo III"),
    ]
}

fn find_diablo_executable(install_path: &Path) -> Option<PathBuf> {
    [
        install_path.join("Diablo III.exe"),
        install_path.join("Diablo III Launcher.exe"),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

fn looks_like_diablo_install(path: &Path) -> bool {
    path.join(".build.info").is_file() && path.join("Data").is_dir()
}

fn prefix_for_windows_path(path: &Path) -> Option<PathBuf> {
    path.ancestors().find_map(|ancestor| {
        if ancestor.file_name() == Some(OsStr::new("drive_c")) {
            ancestor.parent().map(Path::to_path_buf)
        } else {
            None
        }
    })
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.contains(&path) {
        paths.push(path);
    }
}
