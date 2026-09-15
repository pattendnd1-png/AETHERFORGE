use sanctuary_battlenet::{
    BridgeDiscovery, BridgeHealthReport, BridgeHealthState, NetworkBridgeReport,
    NetworkBridgeState, ProcessSnapshot, RetryPolicy, SupervisorState, discover_for_home,
    find_diablo_candidates, launch_battlenet_with_retry, launch_diablo_with_retry,
    open_official_download,
};
use sanctuary_casc::{
    ContentInventory, build_inventory, default_inventory_cache_path, inventory_is_current,
    parse_build_info, read_cached_inventory, write_cached_inventory,
};
use sanctuary_core::{InstallState, TaskEvent};
use sanctuary_diagnostics::{DiagnosticsReport, collect};
use sanctuary_install::probe_install;
use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
    time::Duration,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryAction {
    Install,
    Locate,
    Index,
    Play,
    Repair,
}

impl PrimaryAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Install => "INSTALL DIABLO III",
            Self::Locate => "LOCATE GAME",
            Self::Index => "INDEX CONTENT",
            Self::Play => "PLAY",
            Self::Repair => "REPAIR / RE-INDEX",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InventoryState {
    #[default]
    NotIndexed,
    Stale,
    Indexing,
    Indexed,
    Failed,
}

impl InventoryState {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotIndexed => "Not indexed",
            Self::Stale => "Stale",
            Self::Indexing => "Indexing",
            Self::Indexed => "Indexed",
            Self::Failed => "Index failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BridgeState {
    #[default]
    MissingClient,
    NeedsSetup,
    Installing,
    Updating,
    Verifying,
    Indexing,
    Ready,
    Starting,
    Running,
    BrokenInstall,
    Error,
}

impl BridgeState {
    pub fn label(self) -> &'static str {
        match self {
            Self::MissingClient => "Battle.net missing",
            Self::NeedsSetup => "Battle.net setup required",
            Self::Installing => "Installing Diablo III",
            Self::Updating => "Updating Diablo III",
            Self::Verifying => "Verifying installation",
            Self::Indexing => "Indexing content",
            Self::Ready => "Ready",
            Self::Starting => "Starting",
            Self::Running => "Running",
            Self::BrokenInstall => "Installation needs attention",
            Self::Error => "Bridge error",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionIntent {
    Install,
    SignIn,
    OpenClient,
    Play,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparationContext {
    pub network_state: NetworkBridgeState,
    pub software_bridge_ready: bool,
    pub battlenet_running: bool,
    pub diablo_running: bool,
    pub install_state: InstallState,
    pub inventory_state: InventoryState,
    pub bridge_state: BridgeState,
    pub official_game_ready: bool,
    pub launch_cooldown_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationDecision {
    RepairNetwork,
    RepairSoftwareBridge,
    OpenDownloadPage,
    ShowBattleNetInteraction,
    WatchInstall,
    WaitForUpdate,
    VerifyGame,
    IndexContent,
    LaunchGame,
    AlreadyRunning,
    LaunchBlocked,
}

impl PreparationDecision {
    pub fn label(self) -> &'static str {
        match self {
            Self::RepairNetwork => "Network repair required",
            Self::RepairSoftwareBridge => "Software bridge repair required",
            Self::OpenDownloadPage => "Battle.net setup required",
            Self::ShowBattleNetInteraction => "Battle.net interaction required",
            Self::WatchInstall => "Waiting for Diablo III installation",
            Self::WaitForUpdate => "Waiting for Battle.net update",
            Self::VerifyGame => "Verifying installation",
            Self::IndexContent => "Indexing content",
            Self::LaunchGame => "Ready to play",
            Self::AlreadyRunning => "Diablo III is running",
            Self::LaunchBlocked => "Launch temporarily paused",
        }
    }
}

pub fn prepare_session(context: &PreparationContext, intent: SessionIntent) -> PreparationDecision {
    if context.network_state == NetworkBridgeState::Offline {
        return PreparationDecision::RepairNetwork;
    }
    if context.launch_cooldown_active {
        return PreparationDecision::LaunchBlocked;
    }

    match intent {
        SessionIntent::SignIn | SessionIntent::OpenClient => {
            if context.software_bridge_ready {
                PreparationDecision::ShowBattleNetInteraction
            } else {
                PreparationDecision::OpenDownloadPage
            }
        }
        SessionIntent::Install => {
            if !context.software_bridge_ready {
                PreparationDecision::OpenDownloadPage
            } else if context.install_state == InstallState::NotConfigured {
                PreparationDecision::ShowBattleNetInteraction
            } else {
                PreparationDecision::WatchInstall
            }
        }
        SessionIntent::Play => {
            if context.diablo_running {
                return PreparationDecision::AlreadyRunning;
            }
            if context.bridge_state == BridgeState::Updating {
                return PreparationDecision::WaitForUpdate;
            }
            if !context.software_bridge_ready {
                return PreparationDecision::RepairSoftwareBridge;
            }
            match context.install_state {
                InstallState::NotConfigured => PreparationDecision::ShowBattleNetInteraction,
                InstallState::Searching => PreparationDecision::VerifyGame,
                InstallState::FoundUnindexed | InstallState::Indexing => {
                    PreparationDecision::IndexContent
                }
                InstallState::Ready => {
                    if context.inventory_state != InventoryState::Indexed {
                        PreparationDecision::IndexContent
                    } else if context.official_game_ready {
                        PreparationDecision::LaunchGame
                    } else {
                        PreparationDecision::RepairSoftwareBridge
                    }
                }
                InstallState::NeedsRepair
                | InstallState::UnsupportedBuild
                | InstallState::Error => PreparationDecision::VerifyGame,
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProbeResult {
    pub path: PathBuf,
    pub state: InstallState,
    pub inventory_state: InventoryState,
    pub inventory: Option<ContentInventory>,
    pub diagnostics: DiagnosticsReport,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct IndexResult {
    pub path: PathBuf,
    pub inventory: ContentInventory,
    pub cache_warning: Option<String>,
}

#[derive(Debug, Clone)]
pub enum OfficialBridgeEvent {
    BridgeValidated,
    BridgeInvalidated(String),
    RecoveryStarted,
    RecoveryCompleted {
        detail: String,
    },
    RecoveryFailed(String),
    ClientLaunched,
    DownloadPageOpened,
    InstallDetected(PathBuf),
    OfficialGameStarting,
    OfficialGameStarted,
    OfficialGameExited {
        success: bool,
    },
    SessionObserved {
        state: SupervisorState,
        processes: ProcessSnapshot,
    },
    UpdateSettled(PathBuf),
    Failed(String),
    WatchFinished,
}

#[derive(Debug, Clone)]
pub enum LauncherEvent {
    Task(TaskEvent),
    ProbeComplete(Box<ProbeResult>),
    IndexComplete(Box<IndexResult>),
    IndexFailed { path: PathBuf, message: String },
    BridgeHealth(BridgeHealthReport),
    NetworkBridgeChecked(NetworkBridgeReport),
    OfficialBridge(OfficialBridgeEvent),
}

pub struct TaskBus {
    sender: Sender<LauncherEvent>,
    receiver: Receiver<LauncherEvent>,
}

impl TaskBus {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self { sender, receiver }
    }

    pub fn sender(&self) -> Sender<LauncherEvent> {
        self.sender.clone()
    }

    pub fn drain(&self) -> Vec<LauncherEvent> {
        self.receiver.try_iter().collect()
    }
}

impl Default for TaskBus {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ActivityItem {
    pub label: String,
    pub detail: String,
    pub done: bool,
    pub failed: bool,
}

#[derive(Debug, Clone)]
pub struct LauncherModel {
    pub install_state: InstallState,
    pub install_path: Option<PathBuf>,
    pub inventory_state: InventoryState,
    pub inventory: Option<ContentInventory>,
    pub diagnostics: DiagnosticsReport,
    pub bridge_state: BridgeState,
    pub bridge_detail: String,
    pub bridge_health: BridgeHealthReport,
    pub network_bridge: NetworkBridgeReport,
    pub last_bridge_recovery: Option<String>,
    pub battlenet_running: bool,
    pub agent_running: bool,
    pub diablo_running: bool,
    pub message: String,
    pub activity: Vec<ActivityItem>,
}

impl Default for LauncherModel {
    fn default() -> Self {
        Self {
            install_state: InstallState::NotConfigured,
            install_path: None,
            inventory_state: InventoryState::NotIndexed,
            inventory: None,
            diagnostics: DiagnosticsReport::minimal_for_test(),
            bridge_state: BridgeState::MissingClient,
            bridge_detail: "Battle.net bridge has not been configured yet.".into(),
            bridge_health: BridgeHealthReport::default(),
            network_bridge: NetworkBridgeReport::default(),
            last_bridge_recovery: None,
            battlenet_running: false,
            agent_running: false,
            diablo_running: false,
            message:
                "Install Diablo III through Battle.net or locate an existing installation to begin."
                    .into(),
            activity: Vec::new(),
        }
    }
}

impl LauncherModel {
    pub fn for_state(state: InstallState) -> Self {
        Self {
            install_state: state,
            ..Default::default()
        }
    }

    pub fn primary_action(&self) -> PrimaryAction {
        match self.install_state {
            InstallState::NotConfigured => PrimaryAction::Install,
            InstallState::Searching => PrimaryAction::Locate,
            InstallState::FoundUnindexed | InstallState::Indexing => PrimaryAction::Index,
            InstallState::Ready => PrimaryAction::Play,
            InstallState::NeedsRepair | InstallState::UnsupportedBuild | InstallState::Error => {
                PrimaryAction::Repair
            }
        }
    }

    pub fn play_ready(&self) -> bool {
        self.install_state == InstallState::Ready
            && self.inventory_state == InventoryState::Indexed
            && self.diagnostics.vulkan_available
    }

    pub fn apply_event(&mut self, event: LauncherEvent) {
        match event {
            LauncherEvent::ProbeComplete(result) => {
                let result = *result;
                let supervised_state = matches!(
                    self.bridge_state,
                    BridgeState::Running | BridgeState::Updating
                )
                .then_some(self.bridge_state);
                self.install_state = result.state;
                self.install_path = Some(result.path);
                self.inventory_state = result.inventory_state;
                self.inventory = result.inventory;
                self.diagnostics = result.diagnostics;
                self.message = result.message;
                let fallback_state = match (self.install_state, self.inventory_state) {
                    (InstallState::Ready, InventoryState::Indexed) => BridgeState::Ready,
                    (InstallState::FoundUnindexed, _) => BridgeState::Verifying,
                    (
                        InstallState::NeedsRepair
                        | InstallState::UnsupportedBuild
                        | InstallState::Error,
                        _,
                    ) => BridgeState::BrokenInstall,
                    _ => self.bridge_state,
                };
                self.bridge_state = supervised_state.unwrap_or(fallback_state);
                self.bridge_detail = self.bridge_state.label().into();
            }
            LauncherEvent::IndexComplete(result) => {
                let result = *result;
                let supervised_state = matches!(
                    self.bridge_state,
                    BridgeState::Running | BridgeState::Updating
                )
                .then_some(self.bridge_state);
                self.install_state = InstallState::Ready;
                self.install_path = Some(result.path);
                self.inventory_state = InventoryState::Indexed;
                self.inventory = Some(result.inventory);
                self.bridge_state = supervised_state.unwrap_or(BridgeState::Ready);
                self.bridge_detail = if self.bridge_state == BridgeState::Ready {
                    "Diablo III installation and content index are ready.".into()
                } else {
                    self.bridge_state.label().into()
                };
                self.message = result.cache_warning.map_or_else(
                    || "Content inventory indexed and cached. Native runtime is ready.".into(),
                    |warning| {
                        format!(
                            "Content inventory is ready for this session, but cache write failed: {warning}"
                        )
                    },
                );
            }
            LauncherEvent::IndexFailed { path, message } => {
                self.install_path = Some(path);
                self.install_state = InstallState::FoundUnindexed;
                self.inventory_state = InventoryState::Failed;
                self.bridge_state = BridgeState::BrokenInstall;
                self.bridge_detail = message.clone();
                self.message = message;
            }
            LauncherEvent::NetworkBridgeChecked(report) => {
                let state = report.state;
                self.network_bridge = report;
                if !matches!(
                    self.bridge_state,
                    BridgeState::Running | BridgeState::Updating
                ) && matches!(
                    state,
                    NetworkBridgeState::Degraded | NetworkBridgeState::Offline
                ) {
                    self.message = self.network_bridge.summary.clone();
                }
            }
            LauncherEvent::BridgeHealth(report) => {
                let active = matches!(
                    self.bridge_state,
                    BridgeState::Running | BridgeState::Updating
                );
                self.bridge_health = report.clone();
                if !active {
                    match report.state {
                        BridgeHealthState::Healthy => {}
                        BridgeHealthState::Recovering => {
                            self.bridge_state = BridgeState::Verifying;
                            self.bridge_detail = report.summary.clone();
                        }
                        BridgeHealthState::Degraded => {
                            self.bridge_state = if self.install_state == InstallState::NotConfigured
                            {
                                BridgeState::NeedsSetup
                            } else {
                                BridgeState::BrokenInstall
                            };
                            self.bridge_detail = report.summary.clone();
                        }
                        BridgeHealthState::Broken => {
                            self.bridge_state = if self.install_state == InstallState::NotConfigured
                            {
                                BridgeState::NeedsSetup
                            } else {
                                BridgeState::Error
                            };
                            self.bridge_detail = report.summary.clone();
                        }
                    }
                }
            }
            LauncherEvent::OfficialBridge(event) => match event {
                OfficialBridgeEvent::BridgeValidated => {
                    self.bridge_state = match self.bridge_health.state {
                        BridgeHealthState::Healthy
                            if self.install_state == InstallState::Ready
                                && self.inventory_state == InventoryState::Indexed =>
                        {
                            BridgeState::Ready
                        }
                        BridgeHealthState::Healthy | BridgeHealthState::Recovering => {
                            BridgeState::Verifying
                        }
                        BridgeHealthState::Degraded => {
                            if self.install_state == InstallState::NotConfigured {
                                BridgeState::NeedsSetup
                            } else {
                                BridgeState::BrokenInstall
                            }
                        }
                        BridgeHealthState::Broken => BridgeState::Error,
                    };
                    self.bridge_detail = "Saved Battle.net software bridge validated.".into();
                }
                OfficialBridgeEvent::BridgeInvalidated(message) => {
                    self.bridge_state = if self.install_state == InstallState::NotConfigured {
                        BridgeState::NeedsSetup
                    } else {
                        BridgeState::BrokenInstall
                    };
                    self.bridge_detail = message.clone();
                    self.message = message;
                }
                OfficialBridgeEvent::RecoveryStarted => {
                    self.last_bridge_recovery = Some("Bridge recovery started.".into());
                    self.bridge_health.state = BridgeHealthState::Recovering;
                    self.bridge_health.summary =
                        "OpenSanctuary is rediscovering the Battle.net bridge.".into();
                    if !matches!(
                        self.bridge_state,
                        BridgeState::Running | BridgeState::Updating
                    ) {
                        self.bridge_state = BridgeState::Verifying;
                        self.bridge_detail = self.bridge_health.summary.clone();
                    }
                    self.activity.push(ActivityItem {
                        label: "Repair Battle.net bridge".into(),
                        detail: "Rediscovering".into(),
                        done: false,
                        failed: false,
                    });
                }
                OfficialBridgeEvent::RecoveryCompleted { detail } => {
                    self.last_bridge_recovery = Some(detail.clone());
                    self.bridge_health.summary = detail.clone();
                    if !matches!(
                        self.bridge_state,
                        BridgeState::Running | BridgeState::Updating
                    ) {
                        self.bridge_state = match self.bridge_health.state {
                            BridgeHealthState::Healthy
                                if self.install_state == InstallState::Ready
                                    && self.inventory_state == InventoryState::Indexed =>
                            {
                                BridgeState::Ready
                            }
                            BridgeHealthState::Healthy | BridgeHealthState::Recovering => {
                                BridgeState::Verifying
                            }
                            BridgeHealthState::Degraded => {
                                if self.install_state == InstallState::NotConfigured {
                                    BridgeState::NeedsSetup
                                } else {
                                    BridgeState::BrokenInstall
                                }
                            }
                            BridgeHealthState::Broken => {
                                if self.install_state == InstallState::NotConfigured {
                                    BridgeState::NeedsSetup
                                } else if self.install_state == InstallState::Ready {
                                    BridgeState::Verifying
                                } else {
                                    BridgeState::BrokenInstall
                                }
                            }
                        };
                        self.bridge_detail = detail.clone();
                    }
                    if let Some(item) = self
                        .activity
                        .iter_mut()
                        .rev()
                        .find(|item| item.label == "Repair Battle.net bridge" && !item.done)
                    {
                        item.detail = "Complete".into();
                        item.done = true;
                    }
                    self.message = detail;
                }
                OfficialBridgeEvent::RecoveryFailed(message) => {
                    self.last_bridge_recovery = Some(message.clone());
                    if self.bridge_health.state == BridgeHealthState::Recovering {
                        self.bridge_health.state = BridgeHealthState::Broken;
                    }
                    self.bridge_health.summary = message.clone();
                    if !matches!(
                        self.bridge_state,
                        BridgeState::Running | BridgeState::Updating
                    ) {
                        self.bridge_state = match self.bridge_health.state {
                            BridgeHealthState::Degraded => {
                                if self.install_state == InstallState::NotConfigured {
                                    BridgeState::NeedsSetup
                                } else {
                                    BridgeState::BrokenInstall
                                }
                            }
                            BridgeHealthState::Healthy => BridgeState::Verifying,
                            BridgeHealthState::Recovering | BridgeHealthState::Broken => {
                                BridgeState::Error
                            }
                        };
                        self.bridge_detail = message.clone();
                    }
                    if let Some(item) = self
                        .activity
                        .iter_mut()
                        .rev()
                        .find(|item| item.label == "Repair Battle.net bridge" && !item.done)
                    {
                        item.detail = message.clone();
                        item.done = true;
                        item.failed = true;
                    }
                    self.message = message;
                }
                OfficialBridgeEvent::ClientLaunched => {
                    let installing = matches!(
                        self.bridge_state,
                        BridgeState::MissingClient
                            | BridgeState::NeedsSetup
                            | BridgeState::Installing
                            | BridgeState::Error
                    );
                    if installing {
                        self.bridge_state = BridgeState::Installing;
                    }
                    self.bridge_detail = if installing {
                        "Battle.net is running; sign in if prompted and install Diablo III.".into()
                    } else {
                        "Battle.net client launched.".into()
                    };
                    self.message =
                        "Battle.net launched. Install Diablo III there; OpenSanctuary is watching for it."
                            .into();
                }
                OfficialBridgeEvent::DownloadPageOpened => {
                    self.bridge_state = BridgeState::NeedsSetup;
                    self.bridge_detail = "Install Battle.net, then return to OpenSanctuary.".into();
                    self.message =
                        "Battle.net was not found. The official Blizzard download page was opened."
                            .into();
                }
                OfficialBridgeEvent::InstallDetected(path) => {
                    self.install_path = Some(path);
                    self.install_state = InstallState::Searching;
                    self.bridge_state = BridgeState::Verifying;
                    self.bridge_detail =
                        "Diablo III was detected; verifying local game data.".into();
                    self.message =
                        "Diablo III installation detected. Verifying local game data…".into();
                }
                OfficialBridgeEvent::OfficialGameStarting => {
                    self.bridge_state = BridgeState::Starting;
                    self.bridge_detail = "Starting Battle.net and Diablo III…".into();
                    self.message = "Starting the official Diablo III client…".into();
                }
                OfficialBridgeEvent::OfficialGameStarted => {
                    self.bridge_state = BridgeState::Running;
                    self.bridge_detail = "Diablo III is running.".into();
                    self.message = "Official Diablo III client launched from OpenSanctuary.".into();
                }
                OfficialBridgeEvent::OfficialGameExited { success } => {
                    self.bridge_state = if self.install_state == InstallState::Ready
                        && self.inventory_state == InventoryState::Indexed
                    {
                        BridgeState::Ready
                    } else {
                        BridgeState::BrokenInstall
                    };
                    self.bridge_detail = if success {
                        "Diablo III exited; installation remains ready.".into()
                    } else {
                        "Diablo III exited with an error; installation will be revalidated.".into()
                    };
                    self.message = if success {
                        "Official Diablo III client exited cleanly.".into()
                    } else {
                        "Official Diablo III client exited with an error.".into()
                    };
                }
                OfficialBridgeEvent::SessionObserved { state, processes } => {
                    self.battlenet_running = processes.battlenet_running;
                    self.agent_running = processes.agent_running;
                    self.diablo_running = processes.diablo_running;
                    match state {
                        SupervisorState::Running => {
                            self.bridge_state = BridgeState::Running;
                            self.bridge_detail = "Diablo III is already running.".into();
                        }
                        SupervisorState::Updating => {
                            self.bridge_state = BridgeState::Updating;
                            self.bridge_detail =
                                "Battle.net is changing the Diablo III installation.".into();
                            self.message =
                                "Diablo III is updating. OpenSanctuary will verify it automatically when the update settles.".into();
                        }
                        SupervisorState::Ready => {
                            if self.install_state == InstallState::Ready
                                && self.inventory_state == InventoryState::Indexed
                                && !matches!(
                                    self.bridge_state,
                                    BridgeState::Verifying
                                        | BridgeState::Indexing
                                        | BridgeState::Starting
                                )
                            {
                                self.bridge_state = BridgeState::Ready;
                                self.bridge_detail =
                                    "Diablo III installation and content index are ready.".into();
                            }
                        }
                        SupervisorState::Idle => {}
                    }
                }
                OfficialBridgeEvent::UpdateSettled(path) => {
                    self.install_path = Some(path);
                    self.install_state = InstallState::Searching;
                    self.inventory_state = InventoryState::Stale;
                    self.bridge_state = BridgeState::Verifying;
                    self.bridge_detail =
                        "Battle.net update settled; verifying Diablo III installation metadata…"
                            .into();
                    self.message =
                        "Diablo III update completed. Verifying the installation and local content index…"
                            .into();
                }
                OfficialBridgeEvent::Failed(message) => {
                    self.bridge_state = BridgeState::Error;
                    self.bridge_detail = message.clone();
                    self.message = message;
                }
                OfficialBridgeEvent::WatchFinished => {
                    if self.bridge_state == BridgeState::Installing {
                        self.bridge_state = BridgeState::NeedsSetup;
                        self.bridge_detail =
                            "Install watch ended before Diablo III became available.".into();
                    }
                }
            },
            LauncherEvent::Task(event) => self.apply_task(event),
        }
    }

    pub fn record_task(&mut self, event: TaskEvent) {
        self.apply_task(event);
    }

    fn apply_task(&mut self, event: TaskEvent) {
        match event {
            TaskEvent::Started { label } => self.activity.push(ActivityItem {
                label,
                detail: "Started".into(),
                done: false,
                failed: false,
            }),
            TaskEvent::Progress { label, fraction } => {
                if let Some(item) = self
                    .activity
                    .iter_mut()
                    .rev()
                    .find(|item| item.label == label && !item.done)
                {
                    item.detail = format!("{}%", (fraction.clamp(0.0, 1.0) * 100.0).round());
                }
            }
            TaskEvent::Finished { label } => {
                if let Some(item) = self
                    .activity
                    .iter_mut()
                    .rev()
                    .find(|item| item.label == label && !item.done)
                {
                    item.detail = "Complete".into();
                    item.done = true;
                }
            }
            TaskEvent::Failed { label, message } => {
                if let Some(item) = self
                    .activity
                    .iter_mut()
                    .rev()
                    .find(|item| item.label == label && !item.done)
                {
                    item.detail = message;
                    item.done = true;
                    item.failed = true;
                } else {
                    self.activity.push(ActivityItem {
                        label,
                        detail: message,
                        done: true,
                        failed: true,
                    });
                }
            }
        }
    }
}

pub fn spawn_probe(path: PathBuf, sender: Sender<LauncherEvent>) -> JoinHandle<()> {
    thread::spawn(move || {
        let label = "Inspect Diablo III installation".to_string();
        let _ = sender.send(LauncherEvent::Task(TaskEvent::Started {
            label: label.clone(),
        }));
        let _ = sender.send(LauncherEvent::Task(TaskEvent::Progress {
            label: label.clone(),
            fraction: 0.25,
        }));

        let result = evaluate_path(&path);
        let failed = matches!(
            result.state,
            InstallState::NeedsRepair | InstallState::UnsupportedBuild | InstallState::Error
        );
        if failed {
            let _ = sender.send(LauncherEvent::Task(TaskEvent::Failed {
                label: label.clone(),
                message: result.message.clone(),
            }));
        } else {
            let _ = sender.send(LauncherEvent::Task(TaskEvent::Progress {
                label: label.clone(),
                fraction: 1.0,
            }));
            let _ = sender.send(LauncherEvent::Task(TaskEvent::Finished { label }));
        }
        let _ = sender.send(LauncherEvent::ProbeComplete(Box::new(result)));
    })
}

pub fn spawn_index(
    path: PathBuf,
    cache_path: PathBuf,
    sender: Sender<LauncherEvent>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let label = "Index Diablo III content".to_string();
        let _ = sender.send(LauncherEvent::Task(TaskEvent::Started {
            label: label.clone(),
        }));
        let _ = sender.send(LauncherEvent::Task(TaskEvent::Progress {
            label: label.clone(),
            fraction: 0.1,
        }));

        let inventory = match build_inventory(&path) {
            Ok(inventory) => inventory,
            Err(error) => {
                let message = error.to_string();
                let _ = sender.send(LauncherEvent::Task(TaskEvent::Failed {
                    label,
                    message: message.clone(),
                }));
                let _ = sender.send(LauncherEvent::IndexFailed { path, message });
                return;
            }
        };

        let _ = sender.send(LauncherEvent::Task(TaskEvent::Progress {
            label: label.clone(),
            fraction: 0.85,
        }));
        let cache_warning = write_cached_inventory(&cache_path, &inventory)
            .err()
            .map(|error| error.to_string());
        if let Some(warning) = &cache_warning {
            let _ = sender.send(LauncherEvent::Task(TaskEvent::Failed {
                label: "Cache content inventory".into(),
                message: warning.clone(),
            }));
        }
        let _ = sender.send(LauncherEvent::Task(TaskEvent::Progress {
            label: label.clone(),
            fraction: 1.0,
        }));
        let _ = sender.send(LauncherEvent::Task(TaskEvent::Finished { label }));
        let _ = sender.send(LauncherEvent::IndexComplete(Box::new(IndexResult {
            path,
            inventory,
            cache_warning,
        })));
    })
}

pub fn spawn_official_install_bridge(
    home: PathBuf,
    battlenet_already_running: bool,
    sender: Sender<LauncherEvent>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let label = "Install Diablo III via Battle.net".to_string();
        let _ = sender.send(LauncherEvent::Task(TaskEvent::Started {
            label: label.clone(),
        }));

        let mut discovery = discover_for_home(&home);
        if let Some(path) = find_diablo_candidates(&home, &discovery).into_iter().next() {
            let _ = sender.send(LauncherEvent::Task(TaskEvent::Finished {
                label: label.clone(),
            }));
            let _ = sender.send(LauncherEvent::OfficialBridge(
                OfficialBridgeEvent::InstallDetected(path),
            ));
            return;
        }

        let mut client_launched = battlenet_already_running;
        if !client_launched {
            match discovery.launcher_exe.is_some() {
                true => match launch_battlenet_with_retry(&discovery, RetryPolicy::default()) {
                    Ok(_) => {
                        client_launched = true;
                        let _ = sender.send(LauncherEvent::OfficialBridge(
                            OfficialBridgeEvent::ClientLaunched,
                        ));
                    }
                    Err(error) => {
                        let message = error.to_string();
                        let _ = sender.send(LauncherEvent::Task(TaskEvent::Failed {
                            label: label.clone(),
                            message: message.clone(),
                        }));
                        let _ = sender.send(LauncherEvent::OfficialBridge(
                            OfficialBridgeEvent::Failed(message),
                        ));
                        return;
                    }
                },
                false => match open_official_download() {
                    Ok(_) => {
                        let _ = sender.send(LauncherEvent::OfficialBridge(
                            OfficialBridgeEvent::DownloadPageOpened,
                        ));
                    }
                    Err(error) => {
                        let message = error.to_string();
                        let _ = sender.send(LauncherEvent::Task(TaskEvent::Failed {
                            label: label.clone(),
                            message: message.clone(),
                        }));
                        let _ = sender.send(LauncherEvent::OfficialBridge(
                            OfficialBridgeEvent::Failed(message),
                        ));
                        return;
                    }
                },
            }
        }

        for _ in 0..600 {
            thread::sleep(Duration::from_secs(3));
            discovery = discover_for_home(&home);
            if !client_launched && discovery.battlenet_ready() {
                match launch_battlenet_with_retry(&discovery, RetryPolicy::default()) {
                    Ok(_) => {
                        client_launched = true;
                        let _ = sender.send(LauncherEvent::OfficialBridge(
                            OfficialBridgeEvent::ClientLaunched,
                        ));
                    }
                    Err(error) => {
                        let message = error.to_string();
                        let _ = sender.send(LauncherEvent::Task(TaskEvent::Failed {
                            label: label.clone(),
                            message: message.clone(),
                        }));
                        let _ = sender.send(LauncherEvent::OfficialBridge(
                            OfficialBridgeEvent::Failed(message),
                        ));
                        return;
                    }
                }
            }
            if let Some(path) = find_diablo_candidates(&home, &discovery).into_iter().next() {
                let _ = sender.send(LauncherEvent::Task(TaskEvent::Finished { label }));
                let _ = sender.send(LauncherEvent::OfficialBridge(
                    OfficialBridgeEvent::InstallDetected(path),
                ));
                return;
            }
        }

        let message = "Battle.net install watch timed out after 30 minutes. Use Locate Game after the installation finishes.".to_string();
        let _ = sender.send(LauncherEvent::Task(TaskEvent::Failed {
            label,
            message: message.clone(),
        }));
        let _ = sender.send(LauncherEvent::OfficialBridge(OfficialBridgeEvent::Failed(
            message,
        )));
        let _ = sender.send(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::WatchFinished,
        ));
    })
}

pub fn spawn_battlenet_client(
    discovery: BridgeDiscovery,
    sender: Sender<LauncherEvent>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let label = "Open Battle.net".to_string();
        let _ = sender.send(LauncherEvent::Task(TaskEvent::Started {
            label: label.clone(),
        }));
        match launch_battlenet_with_retry(&discovery, RetryPolicy::default()) {
            Ok(_) => {
                let _ = sender.send(LauncherEvent::Task(TaskEvent::Finished { label }));
                let _ = sender.send(LauncherEvent::OfficialBridge(
                    OfficialBridgeEvent::ClientLaunched,
                ));
            }
            Err(error) => {
                let message = error.to_string();
                let _ = sender.send(LauncherEvent::Task(TaskEvent::Failed {
                    label,
                    message: message.clone(),
                }));
                let _ = sender.send(LauncherEvent::OfficialBridge(OfficialBridgeEvent::Failed(
                    message,
                )));
            }
        }
    })
}

pub fn spawn_official_game(
    discovery: BridgeDiscovery,
    install_path: PathBuf,
    battlenet_already_running: bool,
    sender: Sender<LauncherEvent>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let label = "Official Diablo III client".to_string();
        let _ = sender.send(LauncherEvent::Task(TaskEvent::Started {
            label: label.clone(),
        }));

        let _ = sender.send(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::OfficialGameStarting,
        ));

        if !battlenet_already_running {
            match launch_battlenet_with_retry(&discovery, RetryPolicy::default()) {
                Ok(_) => thread::sleep(Duration::from_millis(1500)),
                Err(error) => {
                    let message = error.to_string();
                    let _ = sender.send(LauncherEvent::Task(TaskEvent::Failed {
                        label,
                        message: message.clone(),
                    }));
                    let _ = sender.send(LauncherEvent::OfficialBridge(
                        OfficialBridgeEvent::Failed(message),
                    ));
                    return;
                }
            }
        }
        let mut child =
            match launch_diablo_with_retry(&discovery, &install_path, RetryPolicy::default()) {
                Ok(child) => child,
                Err(error) => {
                    let message = error.to_string();
                    let _ = sender.send(LauncherEvent::Task(TaskEvent::Failed {
                        label,
                        message: message.clone(),
                    }));
                    let _ = sender.send(LauncherEvent::OfficialBridge(
                        OfficialBridgeEvent::Failed(message),
                    ));
                    return;
                }
            };
        let _ = sender.send(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::OfficialGameStarted,
        ));
        match child.wait() {
            Ok(status) if status.success() => {
                let _ = sender.send(LauncherEvent::Task(TaskEvent::Finished { label }));
                let _ = sender.send(LauncherEvent::OfficialBridge(
                    OfficialBridgeEvent::OfficialGameExited { success: true },
                ));
            }
            Ok(status) => {
                let message = format!("Official Diablo III client exited with {status}");
                let _ = sender.send(LauncherEvent::Task(TaskEvent::Failed {
                    label,
                    message: message.clone(),
                }));
                let _ = sender.send(LauncherEvent::OfficialBridge(
                    OfficialBridgeEvent::OfficialGameExited { success: false },
                ));
            }
            Err(error) => {
                let message = format!("Failed while waiting for Diablo III: {error}");
                let _ = sender.send(LauncherEvent::Task(TaskEvent::Failed {
                    label,
                    message: message.clone(),
                }));
                let _ = sender.send(LauncherEvent::OfficialBridge(OfficialBridgeEvent::Failed(
                    message,
                )));
            }
        }
    })
}

pub fn evaluate_path(path: &Path) -> ProbeResult {
    evaluate_path_with_cache(path, &default_inventory_cache_path())
}

pub fn evaluate_path_with_cache(path: &Path, cache_path: &Path) -> ProbeResult {
    let diagnostics = collect(Some(path));
    let install = match probe_install(path) {
        Ok(install) => install,
        Err(error) => {
            return ProbeResult {
                path: path.to_path_buf(),
                state: InstallState::NeedsRepair,
                inventory_state: InventoryState::NotIndexed,
                inventory: None,
                diagnostics,
                message: error.to_string(),
            };
        }
    };

    let build = match parse_build_info(&install.build_info) {
        Ok(build) => build,
        Err(error) => {
            return ProbeResult {
                path: path.to_path_buf(),
                state: InstallState::NeedsRepair,
                inventory_state: InventoryState::NotIndexed,
                inventory: None,
                diagnostics,
                message: error.to_string(),
            };
        }
    };
    if build.build_key.is_none() {
        return ProbeResult {
            path: path.to_path_buf(),
            state: InstallState::UnsupportedBuild,
            inventory_state: InventoryState::NotIndexed,
            inventory: None,
            diagnostics,
            message: "The installation is readable, but no supported build key was found.".into(),
        };
    }

    match read_cached_inventory(cache_path) {
        Err(error) => ProbeResult {
            path: path.to_path_buf(),
            state: InstallState::FoundUnindexed,
            inventory_state: InventoryState::NotIndexed,
            inventory: None,
            diagnostics,
            message: format!("Inventory cache could not be read; re-index required: {error}"),
        },
        Ok(None) => ProbeResult {
            path: path.to_path_buf(),
            state: InstallState::FoundUnindexed,
            inventory_state: InventoryState::NotIndexed,
            inventory: None,
            diagnostics,
            message: "Installation located. Build a local read-only content inventory to continue."
                .into(),
        },
        Ok(Some(inventory)) => match inventory_is_current(&inventory, path) {
            Ok(true) => ProbeResult {
                path: path.to_path_buf(),
                state: InstallState::Ready,
                inventory_state: InventoryState::Indexed,
                inventory: Some(inventory),
                diagnostics,
                message: "Cached local content inventory is current.".into(),
            },
            Ok(false) => ProbeResult {
                path: path.to_path_buf(),
                state: InstallState::FoundUnindexed,
                inventory_state: InventoryState::Stale,
                inventory: Some(inventory),
                diagnostics,
                message: "The local content inventory is stale. Re-index to continue.".into(),
            },
            Err(error) => ProbeResult {
                path: path.to_path_buf(),
                state: InstallState::FoundUnindexed,
                inventory_state: InventoryState::Failed,
                inventory: Some(inventory),
                diagnostics,
                message: format!("Inventory freshness check failed: {error}"),
            },
        },
    }
}

pub fn write_launch_descriptor(
    cache_dir: &Path,
    install_path: &Path,
) -> Result<PathBuf, sanctuary_core::SanctuaryError> {
    use sanctuary_core::{LaunchDescriptor, SanctuaryError};
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    let descriptor = LaunchDescriptor::new(install_path.to_path_buf());
    descriptor.validate()?;
    fs::create_dir_all(cache_dir).map_err(|source| SanctuaryError::Io {
        path: cache_dir.to_path_buf(),
        source,
    })?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let path = cache_dir.join(format!("launch-{}-{stamp}.json", std::process::id()));
    let bytes = serde_json::to_vec_pretty(&descriptor)
        .map_err(|error| SanctuaryError::InvalidLaunchDescriptor(error.to_string()))?;
    fs::write(&path, bytes).map_err(|source| SanctuaryError::Io {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

pub fn spawn_engine(
    engine_path: &Path,
    descriptor_path: &Path,
) -> Result<std::process::Child, sanctuary_core::SanctuaryError> {
    std::process::Command::new(engine_path)
        .arg("--descriptor")
        .arg(descriptor_path)
        .spawn()
        .map_err(|error| {
            sanctuary_core::SanctuaryError::EngineLaunch(format!(
                "{}: {error}",
                engine_path.display()
            ))
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticsFormat {
    Text,
    Json,
}

pub fn export_diagnostics(
    report: &DiagnosticsReport,
    path: &Path,
    format: DiagnosticsFormat,
) -> Result<(), sanctuary_core::SanctuaryError> {
    use sanctuary_core::SanctuaryError;
    use std::fs;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| SanctuaryError::Diagnostics(error.to_string()))?;
    }
    let content = match format {
        DiagnosticsFormat::Text => report.to_text(),
        DiagnosticsFormat::Json => report
            .to_json()
            .map_err(|error| SanctuaryError::Diagnostics(error.to_string()))?,
    };
    fs::write(path, content)
        .map_err(|error| SanctuaryError::Diagnostics(format!("{}: {error}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sanctuary_casc::{build_inventory, write_cached_inventory};
    use sanctuary_core::LaunchDescriptor;
    use std::fs;

    fn fixture_install() -> tempfile::TempDir {
        let td = tempfile::tempdir().unwrap();
        fs::write(
            td.path().join(".build.info"),
            "Branch!STRING:0|Build Key!HEX:16|Version!STRING:0|Product!STRING:0\nD3|0011223344556677|2.7.8.1|d3\n",
        )
        .unwrap();
        let data = td.path().join("Data/data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("00.idx"), b"index").unwrap();
        fs::write(data.join("data.000"), b"payload").unwrap();
        td
    }

    #[test]
    fn offline_network_report_does_not_overwrite_running_bridge_state() {
        let mut model = LauncherModel {
            bridge_state: BridgeState::Running,
            ..Default::default()
        };
        model.apply_event(LauncherEvent::NetworkBridgeChecked(
            NetworkBridgeReport::offline("DNS lookup failed"),
        ));
        assert_eq!(model.bridge_state, BridgeState::Running);
        assert_eq!(model.network_bridge.state, NetworkBridgeState::Offline);
    }

    #[test]
    fn current_cached_inventory_produces_ready_state() {
        let td = fixture_install();
        let cache_dir = tempfile::tempdir().unwrap();
        let cache = cache_dir.path().join("inventory-v1.json");
        let inventory = build_inventory(td.path()).unwrap();
        write_cached_inventory(&cache, &inventory).unwrap();

        let result = evaluate_path_with_cache(td.path(), &cache);
        assert_eq!(result.state, InstallState::Ready);
        assert_eq!(result.inventory_state, InventoryState::Indexed);
        assert_eq!(result.inventory.unwrap().archive_files, 1);
    }

    #[test]
    fn missing_or_stale_cache_produces_index_action() {
        let td = fixture_install();
        let cache_dir = tempfile::tempdir().unwrap();
        let cache = cache_dir.path().join("inventory-v1.json");

        let missing = evaluate_path_with_cache(td.path(), &cache);
        let mut model = LauncherModel::default();
        model.apply_event(LauncherEvent::ProbeComplete(Box::new(missing)));
        assert_eq!(model.primary_action(), PrimaryAction::Index);
        assert_eq!(model.inventory_state, InventoryState::NotIndexed);

        let inventory = build_inventory(td.path()).unwrap();
        write_cached_inventory(&cache, &inventory).unwrap();
        fs::write(td.path().join("Data/data/data.000"), b"payload-has-changed").unwrap();
        let stale = evaluate_path_with_cache(td.path(), &cache);
        model.apply_event(LauncherEvent::ProbeComplete(Box::new(stale)));
        assert_eq!(model.primary_action(), PrimaryAction::Index);
        assert_eq!(model.inventory_state, InventoryState::Stale);
    }

    #[test]
    fn completed_index_updates_model_statistics() {
        let td = fixture_install();
        let inventory = build_inventory(td.path()).unwrap();
        let mut model = LauncherModel::for_state(InstallState::FoundUnindexed);
        model.apply_event(LauncherEvent::IndexComplete(Box::new(IndexResult {
            path: td.path().to_path_buf(),
            inventory,
            cache_warning: None,
        })));
        assert_eq!(model.install_state, InstallState::Ready);
        assert_eq!(model.inventory_state, InventoryState::Indexed);
        assert_eq!(model.inventory.as_ref().unwrap().index_files, 1);
        assert_eq!(model.inventory.as_ref().unwrap().archive_files, 1);
    }

    #[test]
    fn failed_index_preserves_install_path_and_surfaces_error() {
        let td = fixture_install();
        let mut model = LauncherModel::for_state(InstallState::Indexing);
        model.apply_event(LauncherEvent::IndexFailed {
            path: td.path().to_path_buf(),
            message: "synthetic indexing failure".into(),
        });
        assert_eq!(model.install_path.as_deref(), Some(td.path()));
        assert_eq!(model.inventory_state, InventoryState::Failed);
        assert!(model.message.contains("synthetic indexing failure"));
        assert_eq!(model.primary_action(), PrimaryAction::Index);
    }

    #[test]
    fn descriptor_is_short_lived_and_valid() {
        let td = fixture_install();
        let cache = tempfile::tempdir().unwrap();
        let path = write_launch_descriptor(cache.path(), td.path()).unwrap();
        let descriptor: LaunchDescriptor =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert!(descriptor.validate().is_ok());
        assert!(path.starts_with(cache.path()));
    }

    #[test]
    fn diagnostics_export_writes_json() {
        let td = tempfile::tempdir().unwrap();
        let path = td.path().join("report.json");
        export_diagnostics(
            &DiagnosticsReport::minimal_for_test(),
            &path,
            DiagnosticsFormat::Json,
        )
        .unwrap();
        let text = fs::read_to_string(path).unwrap();
        assert!(text.contains("opensanctuary_version"));
    }

    #[test]
    fn unconfigured_install_uses_official_install_action() {
        let model = LauncherModel::default();
        assert_eq!(model.primary_action(), PrimaryAction::Install);
        assert_eq!(PrimaryAction::Install.label(), "INSTALL DIABLO III");
    }

    #[test]
    fn official_bridge_install_detection_updates_model() {
        let path = PathBuf::from("/tmp/Diablo III");
        let mut model = LauncherModel::default();
        model.apply_event(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::InstallDetected(path.clone()),
        ));
        assert_eq!(model.install_path.as_deref(), Some(path.as_path()));
        assert_eq!(model.install_state, InstallState::Searching);
        assert!(model.message.contains("detected"));
    }

    #[test]
    fn official_bridge_tracks_install_to_ready_lifecycle() {
        let path = PathBuf::from("/tmp/Diablo III");
        let mut model = LauncherModel::default();
        assert_eq!(model.bridge_state, BridgeState::MissingClient);

        model.apply_event(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::ClientLaunched,
        ));
        assert_eq!(model.bridge_state, BridgeState::Installing);

        model.apply_event(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::InstallDetected(path.clone()),
        ));
        assert_eq!(model.bridge_state, BridgeState::Verifying);

        model.install_state = InstallState::Indexing;
        model.inventory_state = InventoryState::Indexing;
        model.bridge_state = BridgeState::Indexing;
        let inventory = build_inventory(fixture_install().path()).unwrap();
        model.apply_event(LauncherEvent::IndexComplete(Box::new(IndexResult {
            path,
            inventory,
            cache_warning: None,
        })));
        assert_eq!(model.bridge_state, BridgeState::Ready);
    }

    #[test]
    fn official_game_returns_to_ready_after_exit() {
        let mut model = LauncherModel::for_state(InstallState::Ready);
        model.inventory_state = InventoryState::Indexed;
        model.bridge_state = BridgeState::Ready;

        model.apply_event(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::OfficialGameStarting,
        ));
        assert_eq!(model.bridge_state, BridgeState::Starting);

        model.apply_event(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::OfficialGameStarted,
        ));
        assert_eq!(model.bridge_state, BridgeState::Running);

        model.apply_event(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::OfficialGameExited { success: true },
        ));
        assert_eq!(model.bridge_state, BridgeState::Ready);
    }

    #[test]
    fn supervisor_observation_marks_running_game() {
        let mut model = LauncherModel::for_state(InstallState::Ready);
        model.inventory_state = InventoryState::Indexed;
        model.apply_event(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::SessionObserved {
                state: SupervisorState::Running,
                processes: ProcessSnapshot {
                    battlenet_running: true,
                    agent_running: true,
                    diablo_running: true,
                },
            },
        ));
        assert_eq!(model.bridge_state, BridgeState::Running);
        assert!(model.diablo_running);
    }

    #[test]
    fn supervisor_observation_marks_install_updating() {
        let mut model = LauncherModel::for_state(InstallState::Ready);
        model.inventory_state = InventoryState::Indexed;
        model.apply_event(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::SessionObserved {
                state: SupervisorState::Updating,
                processes: ProcessSnapshot {
                    battlenet_running: true,
                    agent_running: true,
                    diablo_running: false,
                },
            },
        ));
        assert_eq!(model.bridge_state, BridgeState::Updating);
        assert!(model.agent_running);
    }

    #[test]
    fn probe_completion_does_not_overwrite_supervised_running_state() {
        let td = fixture_install();
        let cache_dir = tempfile::tempdir().unwrap();
        let cache = cache_dir.path().join("inventory-v1.json");
        let inventory = build_inventory(td.path()).unwrap();
        write_cached_inventory(&cache, &inventory).unwrap();
        let result = evaluate_path_with_cache(td.path(), &cache);

        let mut model = LauncherModel {
            bridge_state: BridgeState::Running,
            diablo_running: true,
            ..Default::default()
        };
        model.apply_event(LauncherEvent::ProbeComplete(Box::new(result)));

        assert_eq!(model.bridge_state, BridgeState::Running);
        assert!(model.diablo_running);
    }

    #[test]
    fn settled_update_returns_to_verification_pipeline() {
        let path = PathBuf::from("/tmp/Diablo III");
        let mut model = LauncherModel::for_state(InstallState::Ready);
        model.inventory_state = InventoryState::Indexed;
        model.bridge_state = BridgeState::Updating;
        model.apply_event(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::UpdateSettled(path.clone()),
        ));
        assert_eq!(model.install_path.as_deref(), Some(path.as_path()));
        assert_eq!(model.install_state, InstallState::Searching);
        assert_eq!(model.bridge_state, BridgeState::Verifying);
    }

    #[test]
    fn invalidated_bridge_marks_broken_install_when_game_was_configured() {
        let mut model = LauncherModel::for_state(InstallState::Ready);
        model.bridge_state = BridgeState::Ready;
        model.apply_event(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::BridgeInvalidated("missing game files".into()),
        ));
        assert_eq!(model.bridge_state, BridgeState::BrokenInstall);
        assert!(model.bridge_detail.contains("missing game files"));
    }

    #[test]
    fn bridge_health_report_is_stored_without_overwriting_running_state() {
        let mut model = LauncherModel {
            bridge_state: BridgeState::Running,
            diablo_running: true,
            ..Default::default()
        };
        let report = BridgeHealthReport {
            state: BridgeHealthState::Degraded,
            summary: "game moved".into(),
            ..Default::default()
        };
        model.apply_event(LauncherEvent::BridgeHealth(report.clone()));
        assert_eq!(model.bridge_health, report);
        assert_eq!(model.bridge_state, BridgeState::Running);
    }

    #[test]
    fn recovery_started_preserves_supervised_running_state() {
        let mut model = LauncherModel {
            bridge_state: BridgeState::Running,
            diablo_running: true,
            ..Default::default()
        };
        model.apply_event(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::RecoveryStarted,
        ));
        assert_eq!(model.bridge_state, BridgeState::Running);
        assert_eq!(model.bridge_health.state, BridgeHealthState::Recovering);
        assert!(model.last_bridge_recovery.is_some());
    }

    #[test]
    fn completed_recovery_reenters_verification_when_not_running() {
        let mut model = LauncherModel {
            bridge_state: BridgeState::BrokenInstall,
            install_state: InstallState::Ready,
            ..Default::default()
        };
        model.apply_event(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::RecoveryCompleted {
                detail: "rediscovered".into(),
            },
        ));
        assert_eq!(model.bridge_state, BridgeState::Verifying);
        assert_eq!(model.last_bridge_recovery.as_deref(), Some("rediscovered"));
    }

    #[cfg(unix)]
    #[test]
    fn engine_spawner_uses_requested_native_binary() {
        let mut child =
            spawn_engine(Path::new("/bin/true"), Path::new("/tmp/descriptor.json")).unwrap();
        assert!(child.wait().unwrap().success());
    }
}

#[cfg(test)]
mod bridge_repair_regression_tests {
    use super::*;

    #[test]
    fn recovery_completion_preserves_degraded_bridge_health() {
        let mut model = LauncherModel::default();
        let report = BridgeHealthReport {
            state: BridgeHealthState::Degraded,
            runner_ready: true,
            prefix_ready: true,
            launcher_ready: true,
            diablo_install_ready: false,
            diablo_exe_ready: false,
            summary: "Battle.net bridge repaired; Diablo III is not installed yet.".into(),
            recovery_detail: None,
        };

        model.apply_event(LauncherEvent::BridgeHealth(report));
        model.apply_event(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::RecoveryCompleted {
                detail: "Battle.net software bridge repaired.".into(),
            },
        ));

        assert_eq!(model.bridge_health.state, BridgeHealthState::Degraded);
        assert_eq!(model.bridge_state, BridgeState::NeedsSetup);
    }
}

#[cfg(test)]
mod session_preparation_tests {
    use super::*;

    fn context() -> PreparationContext {
        PreparationContext {
            network_state: NetworkBridgeState::Online,
            software_bridge_ready: true,
            battlenet_running: false,
            diablo_running: false,
            install_state: InstallState::Ready,
            inventory_state: InventoryState::Indexed,
            bridge_state: BridgeState::Ready,
            official_game_ready: true,
            launch_cooldown_active: false,
        }
    }

    #[test]
    fn preparation_repairs_offline_network_first() {
        let mut ctx = context();
        ctx.network_state = NetworkBridgeState::Offline;
        assert_eq!(
            prepare_session(&ctx, SessionIntent::Play),
            PreparationDecision::RepairNetwork
        );
    }

    #[test]
    fn preparation_routes_missing_client_to_download() {
        let mut ctx = context();
        ctx.software_bridge_ready = false;
        ctx.install_state = InstallState::NotConfigured;
        assert_eq!(
            prepare_session(&ctx, SessionIntent::Install),
            PreparationDecision::OpenDownloadPage
        );
    }

    #[test]
    fn preparation_surfaces_blizzard_interaction_for_sign_in() {
        assert_eq!(
            prepare_session(&context(), SessionIntent::SignIn),
            PreparationDecision::ShowBattleNetInteraction
        );
    }

    #[test]
    fn preparation_surfaces_blizzard_interaction_for_install() {
        let mut ctx = context();
        ctx.install_state = InstallState::NotConfigured;
        ctx.official_game_ready = false;
        assert_eq!(
            prepare_session(&ctx, SessionIntent::Install),
            PreparationDecision::ShowBattleNetInteraction
        );
    }

    #[test]
    fn preparation_detects_already_running_game() {
        let mut ctx = context();
        ctx.diablo_running = true;
        assert_eq!(
            prepare_session(&ctx, SessionIntent::Play),
            PreparationDecision::AlreadyRunning
        );
    }

    #[test]
    fn preparation_waits_for_active_update() {
        let mut ctx = context();
        ctx.bridge_state = BridgeState::Updating;
        assert_eq!(
            prepare_session(&ctx, SessionIntent::Play),
            PreparationDecision::WaitForUpdate
        );
    }

    #[test]
    fn preparation_reenters_verification_for_searching_install() {
        let mut ctx = context();
        ctx.install_state = InstallState::Searching;
        ctx.official_game_ready = false;
        assert_eq!(
            prepare_session(&ctx, SessionIntent::Play),
            PreparationDecision::VerifyGame
        );
    }

    #[test]
    fn preparation_indexes_found_install_before_launch() {
        let mut ctx = context();
        ctx.install_state = InstallState::FoundUnindexed;
        ctx.inventory_state = InventoryState::NotIndexed;
        ctx.official_game_ready = false;
        assert_eq!(
            prepare_session(&ctx, SessionIntent::Play),
            PreparationDecision::IndexContent
        );
    }

    #[test]
    fn preparation_launches_ready_game() {
        assert_eq!(
            prepare_session(&context(), SessionIntent::Play),
            PreparationDecision::LaunchGame
        );
    }

    #[test]
    fn preparation_labels_are_player_facing() {
        assert_eq!(PreparationDecision::LaunchGame.label(), "Ready to play");
        assert_eq!(
            PreparationDecision::WaitForUpdate.label(),
            "Waiting for Battle.net update"
        );
        assert_eq!(
            PreparationDecision::VerifyGame.label(),
            "Verifying installation"
        );
        assert_eq!(
            PreparationDecision::IndexContent.label(),
            "Indexing content"
        );
    }

    #[test]
    fn preparation_honors_bridge_failure_cooldown() {
        let mut ctx = context();
        ctx.launch_cooldown_active = true;
        assert_eq!(
            prepare_session(&ctx, SessionIntent::Play),
            PreparationDecision::LaunchBlocked
        );
    }
}
