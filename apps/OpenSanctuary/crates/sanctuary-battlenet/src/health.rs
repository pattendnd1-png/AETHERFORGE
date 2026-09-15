use super::{BridgeDiscovery, BridgeError, BridgeRecord, discover_for_home_with_record};
use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BridgeHealthState {
    Healthy,
    Degraded,
    Recovering,
    #[default]
    Broken,
}

impl BridgeHealthState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Healthy => "Healthy",
            Self::Degraded => "Degraded",
            Self::Recovering => "Recovering",
            Self::Broken => "Broken",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeHealthReport {
    pub state: BridgeHealthState,
    pub runner_ready: bool,
    pub prefix_ready: bool,
    pub launcher_ready: bool,
    pub diablo_install_ready: bool,
    pub diablo_exe_ready: bool,
    pub summary: String,
    pub recovery_detail: Option<String>,
}

impl Default for BridgeHealthReport {
    fn default() -> Self {
        Self {
            state: BridgeHealthState::Broken,
            runner_ready: false,
            prefix_ready: false,
            launcher_ready: false,
            diablo_install_ready: false,
            diablo_exe_ready: false,
            summary: "Battle.net bridge has not been validated yet.".into(),
            recovery_detail: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryOutcome {
    pub discovery: BridgeDiscovery,
    pub record: Option<BridgeRecord>,
    pub report: BridgeHealthReport,
    pub recovered: bool,
    pub detail: String,
}

pub fn evaluate_bridge_health(discovery: &BridgeDiscovery) -> BridgeHealthReport {
    let runner_ready = discovery.runner.as_ref().is_some_and(|path| path.is_file());
    let prefix_ready = discovery
        .wine_prefix
        .as_ref()
        .is_some_and(|path| path.is_dir());
    let launcher_ready = discovery
        .launcher_exe
        .as_ref()
        .is_some_and(|path| path.is_file());
    let diablo_install_ready = discovery.diablo_install.as_ref().is_some_and(|path| {
        path.is_dir() && path.join(".build.info").is_file() && path.join("Data").is_dir()
    });
    let diablo_exe_ready = discovery
        .diablo_exe
        .as_ref()
        .is_some_and(|path| path.is_file());

    let client_ready = runner_ready && prefix_ready && launcher_ready;
    let game_ready = diablo_install_ready && diablo_exe_ready;
    let state = if client_ready && game_ready {
        BridgeHealthState::Healthy
    } else if client_ready {
        BridgeHealthState::Degraded
    } else {
        BridgeHealthState::Broken
    };
    let summary = match state {
        BridgeHealthState::Healthy => "Battle.net bridge and Diablo III paths are healthy.",
        BridgeHealthState::Degraded => {
            "Battle.net software bridge is healthy; Diablo III still needs installation or validation."
        }
        BridgeHealthState::Recovering => "OpenSanctuary is rediscovering the Battle.net bridge.",
        BridgeHealthState::Broken => {
            "Battle.net bridge is incomplete; setup or repair is required."
        }
    }
    .into();

    BridgeHealthReport {
        state,
        runner_ready,
        prefix_ready,
        launcher_ready,
        diablo_install_ready,
        diablo_exe_ready,
        summary,
        recovery_detail: None,
    }
}

pub fn recover_bridge(home: &Path, saved_record: Option<&BridgeRecord>) -> RecoveryOutcome {
    let discovery = discover_for_home_with_record(home, saved_record);
    let mut report = evaluate_bridge_health(&discovery);
    let record = BridgeRecord::from_discovery(&discovery);
    let recovered = report.state != BridgeHealthState::Broken && record.is_some();
    let detail = if report.state == BridgeHealthState::Healthy && recovered {
        "Battle.net software bridge and Diablo III installation were rediscovered and validated."
            .to_string()
    } else if report.state == BridgeHealthState::Degraded && recovered {
        "Battle.net software bridge was repaired; Diablo III still needs installation or validation."
            .to_string()
    } else {
        "Battle.net bridge rediscovery did not find a complete usable client configuration."
            .to_string()
    };
    report.recovery_detail = Some(detail.clone());
    RecoveryOutcome {
        discovery,
        record,
        report,
        recovered,
        detail,
    }
}

pub fn quarantine_bridge_record(path: &Path) -> Result<Option<PathBuf>, BridgeError> {
    if !path.exists() {
        return Ok(None);
    }
    let mut target = path.with_extension("toml.invalid");
    let mut suffix = 1_u32;
    while target.exists() {
        target = path.with_extension(format!("toml.invalid.{suffix}"));
        suffix += 1;
    }
    fs::rename(path, &target).map_err(|source| BridgeError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(Some(target))
}

#[derive(Debug, Clone)]
pub struct FailureTracker {
    failures: VecDeque<Instant>,
    threshold: usize,
    window: Duration,
    cooldown: Duration,
    cooldown_until: Option<Instant>,
}

impl FailureTracker {
    pub fn new(threshold: usize, window: Duration, cooldown: Duration) -> Self {
        Self {
            failures: VecDeque::new(),
            threshold: threshold.max(1),
            window,
            cooldown,
            cooldown_until: None,
        }
    }

    pub fn can_attempt(&mut self, now: Instant) -> bool {
        if self.cooldown_until.is_some_and(|until| now >= until) {
            self.cooldown_until = None;
            self.failures.clear();
        }
        self.prune(now);
        self.cooldown_until.is_none()
    }

    pub fn record_failure(&mut self, now: Instant) {
        self.prune(now);
        self.failures.push_back(now);
        if self.failures.len() >= self.threshold {
            self.cooldown_until = Some(now + self.cooldown);
        }
    }

    pub fn record_success(&mut self) {
        self.failures.clear();
        self.cooldown_until = None;
    }

    pub fn cooldown_remaining(&self, now: Instant) -> Option<Duration> {
        self.cooldown_until
            .and_then(|until| until.checked_duration_since(now))
    }

    pub fn recent_failures(&self) -> usize {
        self.failures.len()
    }

    fn prune(&mut self, now: Instant) {
        while self
            .failures
            .front()
            .is_some_and(|failure| now.saturating_duration_since(*failure) > self.window)
        {
            self.failures.pop_front();
        }
    }
}

impl Default for FailureTracker {
    fn default() -> Self {
        Self::new(3, Duration::from_secs(60), Duration::from_secs(30))
    }
}
