mod account_page;
mod battlenet_page;
mod chrome;
mod diagnostics;
mod downloads;
mod game_page;
mod notifications;
mod theme;
mod ui_motion;
mod widgets;

use account_page::{AccountPageAction, AccountPageView, render_account_page};
use battlenet_page::{BattleNetPageAction, BattleNetPageView, render_battlenet_page};
use chrome::{
    GameStripAction, GameStripView, TopBarAction, TopBarView, render_game_strip, render_top_bar,
};
use diagnostics::{AdvancedDiagnosticsView, render_advanced_diagnostics};
use downloads::{DownloadsAction, DownloadsView, render_downloads_tray};
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Vec2};
use game_page::{
    GamePageAction, GamePageView, GameReadiness, render_game_hero, render_game_status,
};
use notifications::{notification_entries, render_notifications};
use sanctuary_battlenet::{
    AccountProfile, BattleNetWindowHost, BridgeDiscovery, BridgeHealthState, DesktopSessionState,
    FailureTracker, HostRect, NetworkBridgeState, SessionObservation, SessionSupervisor,
    SupervisorState, account_state_from_processes, bridge_record_for_install, clear_bridge_record,
    default_account_profile_path, default_bridge_record_path, discover_for_home_with_record,
    evaluate_bridge_health, load_account_profile, load_bridge_record, oauth_client_id,
    open_official_account, probe_network_bridge, quarantine_bridge_record, recover_bridge,
    save_account_profile, save_bridge_record,
};
use sanctuary_casc::{StorageKind, default_inventory_cache_path};
use sanctuary_core::{AppConfig, InstallState, TaskEvent, xdg_cache_dir, xdg_config_path};
use sanctuary_install::discover_candidates;
use sanctuary_launcher::{
    BridgeState, DiagnosticsFormat, InventoryState, LauncherEvent, LauncherModel,
    OfficialBridgeEvent, PreparationContext, PreparationDecision, PrimaryAction, SessionIntent,
    TaskBus, export_diagnostics, prepare_session, spawn_battlenet_client, spawn_engine,
    spawn_index, spawn_official_game, spawn_official_install_bridge, spawn_probe,
    write_launch_descriptor,
};
use std::{
    path::{Path, PathBuf},
    process::Child,
    thread,
    time::{Duration, Instant},
};
use theme::configure_style;
use ui_motion::{CarouselState, transition_progress};
use widgets::{
    activity_notification_count, activity_row, activity_tray_summary, compact_fingerprint,
    compact_news_card, drawer_message, format_bytes, home_last_played_card,
    home_last_played_placeholder, inventory_badge, kind_rich_text, latest_feature_card,
    library_card, news_card, paint_featured_accent, paint_hero, paint_mini_diablo_icon,
    state_caption, story_strip_card, subnav_button,
};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OpenSanctuary")
            .with_inner_size([1500.0, 920.0])
            .with_min_inner_size([1180.0, 720.0]),
        renderer: eframe::Renderer::Wgpu,
        persist_window: true,
        ..Default::default()
    };
    eframe::run_native(
        "org.opensanctuary.Launcher",
        options,
        Box::new(|cc| Ok(Box::new(LauncherApp::new(cc)))),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Games,
    BattleNet,
    Account,
    Shop,
    Overview,
    Content,
    Activity,
    Settings,
    Diagnostics,
}

impl Page {
    fn is_game_page(self) -> bool {
        matches!(
            self,
            Self::Overview | Self::Content | Self::Settings | Self::Diagnostics
        )
    }

    fn is_launcher_page(self) -> bool {
        matches!(
            self,
            Self::Home | Self::Games | Self::BattleNet | Self::Account | Self::Shop
        )
    }

    fn home_nav_active(self) -> bool {
        self == Self::Home
    }

    fn games_nav_active(self) -> bool {
        self == Self::Games || self.is_game_page()
    }

    fn battlenet_nav_active(self) -> bool {
        matches!(self, Self::BattleNet | Self::Account)
    }

    fn shop_nav_active(self) -> bool {
        self == Self::Shop
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ContentFilter {
    All,
    Index,
    Archive,
    Config,
    Other,
}

impl ContentFilter {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Index => "Indexes",
            Self::Archive => "Archives",
            Self::Config => "Config",
            Self::Other => "Other",
        }
    }

    fn matches(self, kind: StorageKind) -> bool {
        match self {
            Self::All => true,
            Self::Index => kind == StorageKind::Index,
            Self::Archive => kind == StorageKind::Archive,
            Self::Config => kind == StorageKind::Config,
            Self::Other => kind == StorageKind::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UtilityDrawer {
    Notifications,
    Account,
}

fn toggle_utility_drawer(
    current: Option<UtilityDrawer>,
    requested: UtilityDrawer,
) -> Option<UtilityDrawer> {
    if current == Some(requested) {
        None
    } else {
        Some(requested)
    }
}

fn preparation_surfaces_battlenet(decision: PreparationDecision, intent: SessionIntent) -> bool {
    matches!(
        (decision, intent),
        (PreparationDecision::ShowBattleNetInteraction, _)
            | (PreparationDecision::OpenDownloadPage, _)
            | (PreparationDecision::WatchInstall, SessionIntent::Install)
    )
}

struct LauncherApp {
    config: AppConfig,
    model: LauncherModel,
    bus: TaskBus,
    page: Page,
    locate_open: bool,
    locate_text: String,
    activity_open: bool,
    utility_open: Option<UtilityDrawer>,
    engine_child: Option<Child>,
    engine_descriptor: Option<PathBuf>,
    last_export: Option<PathBuf>,
    content_search: String,
    content_filter: ContentFilter,
    launcher_search: String,
    featured_carousel: CarouselState,
    featured_transition_started: f64,
    bridge_record_path: PathBuf,
    account_profile_path: PathBuf,
    account_profile: AccountProfile,
    battle_net_window_host: BattleNetWindowHost,
    official_discovery: BridgeDiscovery,
    official_install_active: bool,
    official_game_active: bool,
    auto_index_after_probe: bool,
    session_supervisor: SessionSupervisor,
    last_supervisor_poll: Instant,
    last_session_observation: Option<SessionObservation>,
    bridge_failures: FailureTracker,
    network_probe_in_flight: bool,
    network_repair_active: bool,
    last_network_probe: Instant,
}

impl LauncherApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_style(&cc.egui_ctx);
        let config = AppConfig::load_from(&xdg_config_path()).unwrap_or_default();
        let configured_install = config.install_path.clone();
        let bridge_record_path = default_bridge_record_path();
        let account_profile_path = default_account_profile_path();
        let (account_profile, account_profile_error) =
            match load_account_profile(&account_profile_path) {
                Ok(profile) => (profile, None),
                Err(error) => (AccountProfile::default(), Some(error.to_string())),
            };
        let battle_net_window_host = BattleNetWindowHost::detect();
        let (saved_record, bridge_record_error) = match load_bridge_record(&bridge_record_path) {
            Ok(record) => (record, None),
            Err(error) => {
                let quarantine = quarantine_bridge_record(&bridge_record_path)
                    .ok()
                    .flatten()
                    .map(|path| format!("; quarantined as {}", path.display()))
                    .unwrap_or_default();
                (None, Some(format!("{error}{quarantine}")))
            }
        };
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let official_discovery = discover_for_home_with_record(&home, saved_record.as_ref());
        let discovered_install = official_discovery.diablo_install.clone();
        let saved_validation = saved_record
            .as_ref()
            .map(|record| record.validated().map_err(|error| error.to_string()));
        let saved_install = saved_validation
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .and_then(|discovery| discovery.diablo_install.clone());

        let mut app = Self {
            locate_text: configured_install
                .as_ref()
                .or(saved_install.as_ref())
                .or(discovered_install.as_ref())
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            config,
            model: LauncherModel::default(),
            bus: TaskBus::new(),
            page: Page::Overview,
            locate_open: false,
            activity_open: false,
            utility_open: None,
            engine_child: None,
            engine_descriptor: None,
            last_export: None,
            content_search: String::new(),
            content_filter: ContentFilter::All,
            launcher_search: String::new(),
            featured_carousel: CarouselState::new(3),
            featured_transition_started: 0.0,
            bridge_record_path,
            account_profile_path,
            account_profile,
            battle_net_window_host,
            official_discovery,
            official_install_active: false,
            official_game_active: false,
            auto_index_after_probe: false,
            session_supervisor: SessionSupervisor::default(),
            last_supervisor_poll: Instant::now() - Duration::from_secs(2),
            last_session_observation: None,
            bridge_failures: FailureTracker::default(),
            network_probe_in_flight: false,
            network_repair_active: false,
            last_network_probe: Instant::now() - Duration::from_secs(60),
        };
        app.model
            .apply_event(LauncherEvent::BridgeHealth(evaluate_bridge_health(
                &app.official_discovery,
            )));
        if let Some(error) = account_profile_error {
            app.model.message = format!("Account hint profile was ignored: {error}");
        }
        if let Some(error) = bridge_record_error {
            if app.official_discovery.official_game_ready() {
                app.model.apply_event(LauncherEvent::OfficialBridge(
                    OfficialBridgeEvent::RecoveryStarted,
                ));
                app.model.apply_event(LauncherEvent::OfficialBridge(
                    OfficialBridgeEvent::RecoveryCompleted {
                        detail: format!(
                            "Invalid bridge metadata was quarantined and rediscovered: {error}"
                        ),
                    },
                ));
            } else {
                app.model.bridge_state = if app.official_discovery.battlenet_ready() {
                    BridgeState::NeedsSetup
                } else {
                    BridgeState::MissingClient
                };
                app.model.bridge_detail = format!("Saved bridge record ignored: {error}");
            }
        } else if let Some(result) = saved_validation {
            match result {
                Ok(_) => app.model.apply_event(LauncherEvent::OfficialBridge(
                    OfficialBridgeEvent::BridgeValidated,
                )),
                Err(error) => {
                    if app.official_discovery.official_game_ready() {
                        app.model.apply_event(LauncherEvent::OfficialBridge(
                            OfficialBridgeEvent::RecoveryStarted,
                        ));
                        app.model.apply_event(LauncherEvent::OfficialBridge(
                            OfficialBridgeEvent::RecoveryCompleted {
                                detail: format!(
                                    "Saved bridge was stale and was rediscovered: {error}"
                                ),
                            },
                        ));
                    } else {
                        app.model.apply_event(LauncherEvent::OfficialBridge(
                            OfficialBridgeEvent::BridgeInvalidated(error),
                        ));
                    }
                }
            }
        } else if app.official_discovery.battlenet_ready() {
            app.model.bridge_state = BridgeState::NeedsSetup;
            app.model.bridge_detail =
                "Battle.net is available; Diablo III has not been validated yet.".into();
        }

        if let Some(path) = configured_install.or(saved_install).or(discovered_install) {
            app.auto_index_after_probe = true;
            app.start_probe(path);
        }
        app.start_network_probe(false);
        app
    }

    fn start_probe(&mut self, path: PathBuf) {
        self.model.install_state = InstallState::Searching;
        self.model.bridge_state = BridgeState::Verifying;
        self.model.bridge_detail = "Verifying Diablo III installation metadata…".into();
        self.model.install_path = Some(path.clone());
        self.model.message = "Inspecting installation metadata and local inventory cache…".into();
        spawn_probe(path, self.bus.sender());
    }

    fn start_index(&mut self, path: PathBuf) {
        self.model.install_state = InstallState::Indexing;
        self.model.bridge_state = BridgeState::Indexing;
        self.model.bridge_detail = "Building the local Diablo III content index…".into();
        self.model.inventory_state = InventoryState::Indexing;
        self.model.install_path = Some(path.clone());
        self.model.message = "Building a read-only local inventory of Diablo III content…".into();
        spawn_index(path, default_inventory_cache_path(), self.bus.sender());
    }

    fn process_events(&mut self) {
        for event in self.bus.drain() {
            let completed_path = match &event {
                LauncherEvent::ProbeComplete(result) => Some(result.path.clone()),
                LauncherEvent::IndexComplete(result) => Some(result.path.clone()),
                LauncherEvent::IndexFailed { path, .. } => Some(path.clone()),
                LauncherEvent::OfficialBridge(OfficialBridgeEvent::InstallDetected(path)) => {
                    Some(path.clone())
                }
                LauncherEvent::Task(_)
                | LauncherEvent::BridgeHealth(_)
                | LauncherEvent::NetworkBridgeChecked(_)
                | LauncherEvent::OfficialBridge(_) => None,
            };
            let detected_install = match &event {
                LauncherEvent::OfficialBridge(OfficialBridgeEvent::InstallDetected(path)) => {
                    Some(path.clone())
                }
                _ => None,
            };
            let settled_update = match &event {
                LauncherEvent::OfficialBridge(OfficialBridgeEvent::UpdateSettled(path)) => {
                    Some(path.clone())
                }
                _ => None,
            };
            let official_game_exited = matches!(
                &event,
                LauncherEvent::OfficialBridge(OfficialBridgeEvent::OfficialGameExited { .. })
            );
            let auto_index_path = match &event {
                LauncherEvent::ProbeComplete(result)
                    if self.auto_index_after_probe
                        && result.state == InstallState::FoundUnindexed =>
                {
                    Some(result.path.clone())
                }
                LauncherEvent::ProbeComplete(_) if self.auto_index_after_probe => {
                    self.auto_index_after_probe = false;
                    None
                }
                _ => None,
            };
            match &event {
                LauncherEvent::NetworkBridgeChecked(report) => {
                    self.network_probe_in_flight = false;
                    self.last_network_probe = Instant::now();
                    if self.network_repair_active {
                        self.network_repair_active = false;
                        if report.state == NetworkBridgeState::Online {
                            self.bridge_failures.record_success();
                            self.session_supervisor = SessionSupervisor::default();
                            self.last_session_observation = None;
                            self.refresh_official_discovery();
                            self.model.record_task(TaskEvent::Finished {
                                label: "Repair network bridge".into(),
                            });
                            self.model.message =
                                "Battle.net network bridge repaired and session state refreshed."
                                    .into();
                        } else {
                            self.model.record_task(TaskEvent::Failed {
                                label: "Repair network bridge".into(),
                                message: report.summary.clone(),
                            });
                        }
                    }
                }
                LauncherEvent::OfficialBridge(OfficialBridgeEvent::InstallDetected(_))
                | LauncherEvent::OfficialBridge(OfficialBridgeEvent::WatchFinished) => {
                    self.official_install_active = false;
                    self.refresh_official_discovery();
                }
                LauncherEvent::OfficialBridge(OfficialBridgeEvent::ClientLaunched)
                | LauncherEvent::OfficialBridge(OfficialBridgeEvent::OfficialGameStarted)
                | LauncherEvent::OfficialBridge(OfficialBridgeEvent::RecoveryCompleted {
                    ..
                }) => {
                    self.bridge_failures.record_success();
                    if matches!(
                        &event,
                        LauncherEvent::OfficialBridge(OfficialBridgeEvent::OfficialGameStarted)
                    ) {
                        self.official_game_active = true;
                    }
                }
                LauncherEvent::OfficialBridge(OfficialBridgeEvent::OfficialGameExited {
                    ..
                }) => {
                    self.official_game_active = false;
                }
                LauncherEvent::OfficialBridge(OfficialBridgeEvent::SessionObserved {
                    state,
                    ..
                }) => {
                    self.official_game_active = *state == SupervisorState::Running;
                }
                LauncherEvent::OfficialBridge(OfficialBridgeEvent::Failed(_))
                | LauncherEvent::OfficialBridge(OfficialBridgeEvent::RecoveryFailed(_)) => {
                    self.bridge_failures.record_failure(Instant::now());
                    self.official_install_active = false;
                    self.official_game_active = false;
                }
                _ => {}
            }
            self.model.apply_event(event);
            if let Some(path) = completed_path.as_ref() {
                self.config.install_path = Some(path.clone());
                let _ = self.config.save_to(&xdg_config_path());
                self.persist_official_bridge_for(path);
            }
            if let Some(path) = detected_install {
                self.auto_index_after_probe = true;
                self.start_probe(path);
            }
            if let Some(path) = settled_update {
                self.auto_index_after_probe = true;
                self.start_probe(path);
            }
            if let Some(path) = auto_index_path {
                self.auto_index_after_probe = false;
                self.start_index(path);
            }
            if official_game_exited {
                self.revalidate_official_bridge();
            }
        }
    }

    fn start_network_probe(&mut self, repair: bool) {
        if self.network_probe_in_flight {
            if repair {
                self.model.message = "Network bridge check is already running.".into();
            }
            return;
        }
        self.network_probe_in_flight = true;
        self.network_repair_active = repair;
        self.last_network_probe = Instant::now();
        if repair {
            self.page = Page::BattleNet;
            self.model.record_task(TaskEvent::Started {
                label: "Repair network bridge".into(),
            });
            self.model.message =
                "Repairing the Battle.net network bridge: DNS, TCP/443, and TLS…".into();
        }
        let sender = self.bus.sender();
        thread::spawn(move || {
            let report = probe_network_bridge();
            let _ = sender.send(LauncherEvent::NetworkBridgeChecked(report));
        });
    }

    fn poll_network_bridge(&mut self) {
        if !self.network_probe_in_flight
            && self.last_network_probe.elapsed() >= Duration::from_secs(30)
        {
            self.start_network_probe(false);
        }
    }

    fn repair_network_bridge(&mut self) {
        self.start_network_probe(true);
    }

    fn poll_session_supervisor(&mut self) {
        if self.last_supervisor_poll.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.last_supervisor_poll = Instant::now();
        let install_path = self
            .model
            .install_path
            .clone()
            .or_else(|| self.config.install_path.clone());
        let observation = self
            .session_supervisor
            .observe(Path::new("/proc"), install_path.as_deref());

        if self.last_session_observation != Some(observation) {
            let _ = self.bus.sender().send(LauncherEvent::OfficialBridge(
                OfficialBridgeEvent::SessionObserved {
                    state: observation.state,
                    processes: observation.snapshot.processes,
                },
            ));
            self.last_session_observation = Some(observation);
        }

        if observation.update_settled
            && let Some(path) = install_path
        {
            let _ = self.bus.sender().send(LauncherEvent::OfficialBridge(
                OfficialBridgeEvent::UpdateSettled(path),
            ));
        }
    }

    fn repair_official_bridge(&mut self) {
        self.model.apply_event(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::RecoveryStarted,
        ));

        let saved_hint = match load_bridge_record(&self.bridge_record_path) {
            Ok(record) => {
                if let Err(error) = clear_bridge_record(&self.bridge_record_path) {
                    self.model.apply_event(LauncherEvent::OfficialBridge(
                        OfficialBridgeEvent::RecoveryFailed(format!(
                            "Could not reset the saved bridge record: {error}"
                        )),
                    ));
                    return;
                }
                record
            }
            Err(error) => match quarantine_bridge_record(&self.bridge_record_path) {
                Ok(_) => {
                    self.model.last_bridge_recovery = Some(format!(
                        "Corrupt bridge metadata was quarantined before repair: {error}"
                    ));
                    None
                }
                Err(quarantine_error) => {
                    self.model.apply_event(LauncherEvent::OfficialBridge(
                        OfficialBridgeEvent::RecoveryFailed(format!(
                            "Could not quarantine invalid bridge metadata: {quarantine_error}"
                        )),
                    ));
                    return;
                }
            },
        };

        self.battle_net_window_host.release();
        self.battle_net_window_host = BattleNetWindowHost::detect();
        self.session_supervisor = SessionSupervisor::default();
        self.last_session_observation = None;
        self.bridge_failures.record_success();

        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        let outcome = recover_bridge(&home, saved_hint.as_ref());
        self.official_discovery = outcome.discovery.clone();
        self.model
            .apply_event(LauncherEvent::BridgeHealth(outcome.report.clone()));

        if !outcome.recovered {
            self.model.apply_event(LauncherEvent::OfficialBridge(
                OfficialBridgeEvent::RecoveryFailed(outcome.detail),
            ));
            return;
        }

        if let Some(record) = outcome.record.as_ref()
            && let Err(error) = save_bridge_record(&self.bridge_record_path, record)
        {
            self.model.apply_event(LauncherEvent::OfficialBridge(
                OfficialBridgeEvent::RecoveryFailed(format!(
                    "Bridge rediscovered but could not be persisted: {error}"
                )),
            ));
            return;
        }

        self.model.apply_event(LauncherEvent::OfficialBridge(
            OfficialBridgeEvent::RecoveryCompleted {
                detail: outcome.detail.clone(),
            },
        ));

        if let Some(path) = outcome.discovery.diablo_install {
            self.config.install_path = Some(path.clone());
            let _ = self.config.save_to(&xdg_config_path());
            self.auto_index_after_probe = true;
            self.start_probe(path);
        } else {
            self.config.install_path = None;
            let _ = self.config.save_to(&xdg_config_path());
            self.model.install_path = None;
            self.model.install_state = InstallState::NotConfigured;
            self.model.inventory_state = InventoryState::NotIndexed;
            self.model.inventory = None;
            self.model.bridge_state = BridgeState::NeedsSetup;
            self.model.bridge_detail = outcome.detail.clone();
            self.model.message =
                "Battle.net software bridge repaired. Diablo III can now be installed from OpenSanctuary."
                    .into();
        }
    }

    fn preparation_context(&mut self) -> PreparationContext {
        let install_path = self
            .model
            .install_path
            .as_deref()
            .or(self.config.install_path.as_deref());
        let official_game_ready =
            install_path.is_some_and(|path| self.official_discovery.official_game_ready_for(path));
        let launch_cooldown_active = !self.bridge_failures.can_attempt(Instant::now());
        PreparationContext {
            network_state: self.model.network_bridge.state,
            software_bridge_ready: self.official_discovery.battlenet_ready(),
            battlenet_running: self.model.battlenet_running,
            diablo_running: self.model.diablo_running,
            install_state: self.model.install_state,
            inventory_state: self.model.inventory_state,
            bridge_state: self.model.bridge_state,
            official_game_ready,
            launch_cooldown_active,
        }
    }

    fn prepare_for(&mut self, intent: SessionIntent) -> PreparationDecision {
        if !matches!(
            self.model.bridge_state,
            BridgeState::Running | BridgeState::Updating
        ) {
            self.refresh_official_discovery();
        }
        let context = self.preparation_context();
        prepare_session(&context, intent)
    }

    fn execute_preparation(&mut self, decision: PreparationDecision, intent: SessionIntent) {
        if preparation_surfaces_battlenet(decision, intent) {
            self.page = Page::BattleNet;
        }
        match decision {
            PreparationDecision::RepairNetwork => self.repair_network_bridge(),
            PreparationDecision::RepairSoftwareBridge => {
                self.model.message =
                    "Repairing the Battle.net software bridge before continuing…".into();
                self.repair_official_bridge();
            }
            PreparationDecision::OpenDownloadPage => self.start_official_install(),
            PreparationDecision::ShowBattleNetInteraction => match intent {
                SessionIntent::Install | SessionIntent::Play => self.start_official_install(),
                SessionIntent::SignIn => self.show_battlenet_sign_in(),
                SessionIntent::OpenClient => self.open_battlenet_client(),
            },
            PreparationDecision::WatchInstall => {
                if !self.official_install_active {
                    self.start_official_install();
                } else {
                    self.model.message =
                        "Waiting for Battle.net to finish the Diablo III installation…".into();
                }
            }
            PreparationDecision::WaitForUpdate => {
                self.model.message =
                    "Battle.net is updating Diablo III. OpenSanctuary will continue automatically when the update settles."
                        .into();
            }
            PreparationDecision::VerifyGame => {
                if let Some(path) = self
                    .model
                    .install_path
                    .clone()
                    .or_else(|| self.config.install_path.clone())
                {
                    self.auto_index_after_probe = true;
                    self.start_probe(path);
                } else {
                    self.start_official_install();
                }
            }
            PreparationDecision::IndexContent => {
                if let Some(path) = self
                    .model
                    .install_path
                    .clone()
                    .or_else(|| self.config.install_path.clone())
                {
                    if self.model.inventory_state != InventoryState::Indexing {
                        self.start_index(path);
                    }
                } else {
                    self.start_official_install();
                }
            }
            PreparationDecision::LaunchGame => self.launch_ready_game(),
            PreparationDecision::AlreadyRunning => {
                self.official_game_active = true;
                self.model.bridge_state = BridgeState::Running;
                self.model.bridge_detail = "Diablo III is already running.".into();
                self.model.message =
                    "Diablo III is already running; duplicate launch prevented.".into();
            }
            PreparationDecision::LaunchBlocked => {
                let seconds = self
                    .bridge_failures
                    .cooldown_remaining(Instant::now())
                    .map_or(1, |remaining| remaining.as_secs().max(1));
                self.model.bridge_state = BridgeState::Error;
                self.model.bridge_detail = format!(
                    "Bridge launch paused after repeated failures; retry available in {seconds}s."
                );
                self.model.message = self.model.bridge_detail.clone();
            }
        }
    }

    fn launch_ready_game(&mut self) {
        let Some(install_path) = self.model.install_path.clone() else {
            self.start_official_install();
            return;
        };
        if self.model.play_ready() {
            let descriptor_dir = xdg_cache_dir().join("launch");
            match write_launch_descriptor(&descriptor_dir, &install_path).and_then(|descriptor| {
                let engine = Self::engine_binary_path();
                spawn_engine(&engine, &descriptor).map(|child| (child, descriptor))
            }) {
                Ok((child, descriptor)) => {
                    self.engine_child = Some(child);
                    self.engine_descriptor = Some(descriptor);
                    self.model.record_task(TaskEvent::Started {
                        label: "Native engine".into(),
                    });
                    self.model.message = "OpenSanctuary native engine launched.".into();
                }
                Err(error) => {
                    self.model.record_task(TaskEvent::Failed {
                        label: "Native engine".into(),
                        message: error.to_string(),
                    });
                    self.start_official_game_launch(install_path);
                }
            }
        } else {
            self.start_official_game_launch(install_path);
        }
    }

    fn start_official_install(&mut self) {
        self.page = Page::BattleNet;
        if self.official_install_active {
            self.model.message =
                "OpenSanctuary is already watching Battle.net for a Diablo III installation."
                    .into();
            return;
        }
        if !self.bridge_launch_allowed() {
            return;
        }
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        self.official_install_active = true;
        self.refresh_official_discovery();
        self.model.bridge_state = if self.official_discovery.battlenet_ready() {
            BridgeState::Installing
        } else {
            BridgeState::NeedsSetup
        };
        self.model.bridge_detail = self.model.bridge_state.label().into();
        self.model.message = if self.official_discovery.battlenet_ready() {
            "Launching Battle.net. Install Diablo III there; OpenSanctuary will detect it.".into()
        } else {
            "Battle.net is not installed yet. Opening Blizzard's official download page…".into()
        };
        spawn_official_install_bridge(home, self.model.battlenet_running, self.bus.sender());
    }

    fn refresh_official_discovery(&mut self) {
        let saved_record = load_bridge_record(&self.bridge_record_path).ok().flatten();
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        self.official_discovery = discover_for_home_with_record(&home, saved_record.as_ref());
        self.model
            .apply_event(LauncherEvent::BridgeHealth(evaluate_bridge_health(
                &self.official_discovery,
            )));
    }

    fn persist_official_bridge_for(&mut self, install_path: &Path) {
        self.refresh_official_discovery();
        let Some(record) = bridge_record_for_install(&self.official_discovery, install_path) else {
            return;
        };
        let validated = match record.validated() {
            Ok(validated) => validated,
            Err(error) => {
                self.model.bridge_detail = format!("Bridge validation failed: {error}");
                return;
            }
        };
        match save_bridge_record(&self.bridge_record_path, &record) {
            Ok(()) => {
                self.official_discovery = validated;
                self.model
                    .apply_event(LauncherEvent::BridgeHealth(evaluate_bridge_health(
                        &self.official_discovery,
                    )));
            }
            Err(error) => {
                self.model.bridge_detail =
                    format!("Bridge is usable for this session, but persistence failed: {error}");
            }
        }
    }

    fn revalidate_official_bridge(&mut self) {
        let previous_path = self
            .model
            .install_path
            .clone()
            .or_else(|| self.config.install_path.clone());
        self.refresh_official_discovery();
        let install_path = self
            .official_discovery
            .diablo_install
            .clone()
            .or(previous_path.clone());
        let Some(install_path) = install_path else {
            self.model.apply_event(LauncherEvent::OfficialBridge(
                OfficialBridgeEvent::BridgeInvalidated(
                    "No Diablo III installation is configured.".into(),
                ),
            ));
            return;
        };

        if let Some(record) = bridge_record_for_install(&self.official_discovery, &install_path) {
            match record.validated() {
                Ok(validated) => {
                    let moved = previous_path.as_deref() != Some(install_path.as_path());
                    self.official_discovery = validated;
                    let _ = save_bridge_record(&self.bridge_record_path, &record);
                    self.model
                        .apply_event(LauncherEvent::BridgeHealth(evaluate_bridge_health(
                            &self.official_discovery,
                        )));
                    if moved {
                        self.config.install_path = Some(install_path.clone());
                        let _ = self.config.save_to(&xdg_config_path());
                        self.model.apply_event(LauncherEvent::OfficialBridge(
                            OfficialBridgeEvent::RecoveryCompleted {
                                detail: format!(
                                    "Diablo III moved; bridge recovered at {}.",
                                    install_path.display()
                                ),
                            },
                        ));
                        self.auto_index_after_probe = true;
                        self.start_probe(install_path);
                    } else {
                        self.model.apply_event(LauncherEvent::OfficialBridge(
                            OfficialBridgeEvent::BridgeValidated,
                        ));
                    }
                }
                Err(error) => self.model.apply_event(LauncherEvent::OfficialBridge(
                    OfficialBridgeEvent::BridgeInvalidated(error.to_string()),
                )),
            }
        } else {
            self.model.apply_event(LauncherEvent::OfficialBridge(
                OfficialBridgeEvent::BridgeInvalidated(
                    "Battle.net or Diablo III could not be revalidated; rediscovery is required."
                        .into(),
                ),
            ));
        }
    }

    fn bridge_launch_allowed(&mut self) -> bool {
        if self.model.network_bridge.state == NetworkBridgeState::Offline {
            self.model.message =
                "Battle.net network bridge is offline. Repair the network bridge before launching."
                    .into();
            self.page = Page::BattleNet;
            return false;
        }
        let now = Instant::now();
        if self.bridge_failures.can_attempt(now) {
            return true;
        }
        let seconds = self
            .bridge_failures
            .cooldown_remaining(now)
            .map_or(1, |remaining| remaining.as_secs().max(1));
        self.model.bridge_state = BridgeState::Error;
        self.model.bridge_detail =
            format!("Bridge launch paused after repeated failures; retry available in {seconds}s.");
        self.model.message = self.model.bridge_detail.clone();
        false
    }

    fn open_battlenet(&mut self) {
        let decision = self.prepare_for(SessionIntent::OpenClient);
        self.execute_preparation(decision, SessionIntent::OpenClient);
    }

    fn open_battlenet_client(&mut self) {
        self.page = Page::BattleNet;
        if self.model.battlenet_running {
            self.model.message = "Battle.net is already running.".into();
            return;
        }
        if !self.bridge_launch_allowed() {
            return;
        }
        self.refresh_official_discovery();
        if self.official_discovery.battlenet_ready() {
            spawn_battlenet_client(self.official_discovery.clone(), self.bus.sender());
        } else {
            self.start_official_install();
        }
    }

    fn can_launch_game(&self) -> bool {
        if matches!(
            self.model.bridge_state,
            BridgeState::Updating | BridgeState::Starting | BridgeState::Running
        ) {
            return false;
        }
        self.model.play_ready()
            || (self.model.install_state == InstallState::Ready
                && self
                    .model
                    .install_path
                    .as_deref()
                    .is_some_and(|path| self.official_discovery.official_game_ready_for(path)))
    }

    fn start_official_game_launch(&mut self, install_path: PathBuf) {
        if self.model.diablo_running {
            self.official_game_active = true;
            self.model.bridge_state = BridgeState::Running;
            self.model.bridge_detail = "Diablo III is already running.".into();
            self.model.message =
                "Diablo III is already running; duplicate launch prevented.".into();
            return;
        }
        if !self.bridge_launch_allowed() {
            return;
        }
        self.refresh_official_discovery();
        if self
            .official_discovery
            .official_game_ready_for(&install_path)
        {
            self.official_game_active = true;
            self.model.bridge_state = BridgeState::Starting;
            self.model.bridge_detail = "Starting Diablo III…".into();
            self.model.message = if self.model.battlenet_running {
                "Launching the official Diablo III client…".into()
            } else {
                "Starting Battle.net in the background and launching Diablo III…".into()
            };
            spawn_official_game(
                self.official_discovery.clone(),
                install_path,
                self.model.battlenet_running,
                self.bus.sender(),
            );
        } else {
            self.model.message = "The native engine could not launch and the official Battle.net/Diablo III fallback was not detected. Use Open Battle.net or Locate Game.".into();
        }
    }

    fn primary_action_label(&self, action: PrimaryAction) -> &'static str {
        match (action, self.model.bridge_state) {
            (PrimaryAction::Install, BridgeState::Installing) => "INSTALLING / WAITING",
            (_, BridgeState::Verifying) if self.model.install_state == InstallState::Searching => {
                "VERIFYING"
            }
            (_, BridgeState::Updating) => "UPDATING",
            (_, BridgeState::Indexing) => "INDEXING",
            (PrimaryAction::Play, BridgeState::Starting) => "STARTING...",
            (PrimaryAction::Play, BridgeState::Running) => "RUNNING",
            _ => action.label(),
        }
    }

    fn poll_engine(&mut self) {
        let outcome = self.engine_child.as_mut().map(Child::try_wait);
        match outcome {
            Some(Ok(Some(status))) => {
                if status.success() {
                    self.model.record_task(TaskEvent::Finished {
                        label: "Native engine".into(),
                    });
                    self.model.message = "Native engine exited cleanly.".into();
                } else {
                    self.model.record_task(TaskEvent::Failed {
                        label: "Native engine".into(),
                        message: format!("Exited with {status}"),
                    });
                    self.model.message = format!("Native engine exited unexpectedly: {status}");
                }
                self.engine_child = None;
                if let Some(path) = self.engine_descriptor.take() {
                    let _ = std::fs::remove_file(path);
                }
            }
            Some(Err(error)) => {
                self.model.record_task(TaskEvent::Failed {
                    label: "Native engine".into(),
                    message: error.to_string(),
                });
                self.engine_child = None;
                if let Some(path) = self.engine_descriptor.take() {
                    let _ = std::fs::remove_file(path);
                }
            }
            _ => {}
        }
    }

    fn engine_binary_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.join("opensanctuary-engine")))
            .unwrap_or_else(|| PathBuf::from("opensanctuary-engine"))
    }

    fn diagnostics_export_path(&self, extension: &str) -> PathBuf {
        let root = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
            })
            .unwrap_or_else(xdg_cache_dir);
        root.join("opensanctuary/diagnostics")
            .join(format!("{}.{}", self.model.diagnostics.trace_id, extension))
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let active_downloads = self.model.activity.iter().filter(|item| !item.done).count();
        let view = TopBarView {
            home_active: self.page.home_nav_active(),
            games_active: self.page.games_nav_active(),
            battlenet_active: self.page.battlenet_nav_active(),
            shop_active: self.page.shop_nav_active(),
            notifications_active: self.utility_open == Some(UtilityDrawer::Notifications),
            downloads_active: self.activity_open,
            account_active: self.utility_open == Some(UtilityDrawer::Account),
            native_ready: self.model.diagnostics.vulkan_available,
            notification_count: activity_notification_count(&self.model),
            active_downloads,
        };
        match render_top_bar(ui, &mut self.launcher_search, view) {
            TopBarAction::None => {}
            TopBarAction::Home => self.page = Page::Home,
            TopBarAction::Games => self.page = Page::Games,
            TopBarAction::BattleNet => self.page = Page::BattleNet,
            TopBarAction::Shop => self.page = Page::Shop,
            TopBarAction::Notifications => {
                self.utility_open =
                    toggle_utility_drawer(self.utility_open, UtilityDrawer::Notifications);
            }
            TopBarAction::Downloads => self.activity_open = !self.activity_open,
            TopBarAction::Account => {
                self.utility_open =
                    toggle_utility_drawer(self.utility_open, UtilityDrawer::Account);
            }
            TopBarAction::Search(query) => {
                if let Some(page) = search_route(&query) {
                    self.page = page;
                    self.launcher_search.clear();
                }
            }
        }
    }

    fn game_strip(&mut self, ui: &mut egui::Ui) {
        let detail = state_caption(self.model.install_state);
        let view = GameStripView {
            diablo_selected: self.page.is_game_page(),
            last_played_detail: detail,
            launcher_scope: self.page.is_launcher_page(),
            reduced_motion: self.config.reduced_motion,
        };
        match render_game_strip(ui, view) {
            GameStripAction::None => {}
            GameStripAction::Diablo => self.page = Page::Overview,
            GameStripAction::Games => self.page = Page::Games,
        }
    }

    fn utility_drawer(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        match self.utility_open {
            Some(UtilityDrawer::Notifications) => {
                utility_drawer_header(ui, "Notifications", &mut self.utility_open);
                ui.label(
                    RichText::new("Game, Battle.net, and OpenSanctuary events")
                        .small()
                        .color(Color32::from_rgb(121, 143, 162)),
                );
                ui.separator();
                let entries = notification_entries(self.model.bridge_state, &self.model.activity);
                render_notifications(ui, &entries);
            }
            Some(UtilityDrawer::Account) => {
                utility_drawer_header(ui, "Account", &mut self.utility_open);
                let session = self.desktop_session_state();
                ui.label(
                    RichText::new(self.account_display_label())
                        .strong()
                        .color(Color32::from_rgb(213, 225, 234)),
                );
                ui.label(
                    RichText::new(session.label().to_ascii_uppercase())
                        .small()
                        .strong()
                        .color(if session.interactive() {
                            Color32::from_rgb(73, 194, 140)
                        } else {
                            Color32::from_rgb(143, 158, 173)
                        }),
                );
                ui.separator();
                if ui
                    .add_sized([260.0, 34.0], egui::Button::new("Battle.net Account"))
                    .clicked()
                {
                    self.page = Page::Account;
                    self.utility_open = None;
                }
                if ui
                    .add_sized([260.0, 34.0], egui::Button::new("Downloads"))
                    .clicked()
                {
                    self.activity_open = true;
                    self.utility_open = None;
                }
                if ui
                    .add_sized([260.0, 34.0], egui::Button::new("Game Settings"))
                    .clicked()
                {
                    self.page = Page::Settings;
                    self.utility_open = None;
                }
                if ui
                    .add_sized([260.0, 34.0], egui::Button::new("Open Battle.net"))
                    .clicked()
                {
                    self.page = Page::BattleNet;
                    self.utility_open = None;
                    self.open_battlenet();
                }
                ui.separator();
                if ui
                    .add_sized([260.0, 34.0], egui::Button::new("OpenSanctuary Settings"))
                    .clicked()
                {
                    self.page = Page::Settings;
                    self.utility_open = None;
                }
                if ui
                    .add_sized([260.0, 34.0], egui::Button::new("Diagnostics"))
                    .clicked()
                {
                    self.page = Page::Diagnostics;
                    self.utility_open = None;
                }
                ui.add_space(8.0);
                drawer_message(
                    ui,
                    "Credential boundary",
                    "OpenSanctuary stores no raw Blizzard password, authenticator code, recovery code, or captcha response.",
                );
            }
            None => {}
        }
    }

    fn desktop_session_state(&self) -> DesktopSessionState {
        self.last_session_observation
            .map_or(DesktopSessionState::Stopped, |observation| {
                account_state_from_processes(observation.snapshot.processes)
            })
    }

    fn account_display_label(&self) -> String {
        self.account_profile
            .battletag
            .clone()
            .or_else(|| self.account_profile.masked_email())
            .unwrap_or_else(|| "Battle.net account".into())
    }

    fn save_account_profile_state(&mut self) {
        match save_account_profile(&self.account_profile_path, &self.account_profile) {
            Ok(()) => {
                self.model.message =
                    "Battle.net account hint saved locally (no password stored).".into();
            }
            Err(error) => {
                self.model.message = format!("Could not save Battle.net account hint: {error}");
            }
        }
    }

    fn sign_in_battlenet(&mut self) {
        let decision = self.prepare_for(SessionIntent::SignIn);
        self.execute_preparation(decision, SessionIntent::SignIn);
    }

    fn show_battlenet_sign_in(&mut self) {
        self.page = Page::BattleNet;
        self.open_battlenet_client();
        self.model.message = "Use the Blizzard-controlled Battle.net surface to sign in; OpenSanctuary does not capture the password or 2FA response.".into();
    }

    fn manage_battlenet_account(&mut self) {
        match open_official_account() {
            Ok(_) => {
                self.model.message =
                    "Opened Blizzard account management in the default browser.".into();
            }
            Err(error) => {
                self.model.message = format!("Could not open Blizzard account management: {error}");
            }
        }
    }

    fn account_page(&mut self, ui: &mut egui::Ui) {
        let session = self.desktop_session_state();
        let health = self.model.bridge_health.state.label().to_string();
        let game_access = if self.model.install_path.is_some() {
            "Installed"
        } else {
            "Not installed"
        };
        let game_status = if self.can_launch_game() {
            "Ready"
        } else {
            state_caption(self.model.install_state)
        };
        let view = AccountPageView {
            session,
            game_access,
            game_status,
            bridge_health: &health,
            oauth_client_configured: oauth_client_id().is_some(),
        };
        match render_account_page(ui, &mut self.account_profile, view) {
            AccountPageAction::None => {}
            AccountPageAction::SaveProfile => self.save_account_profile_state(),
            AccountPageAction::SignIn => self.sign_in_battlenet(),
            AccountPageAction::OpenBattleNet => {
                self.page = Page::BattleNet;
                self.open_battlenet();
            }
            AccountPageAction::ManageAccount => self.manage_battlenet_account(),
            AccountPageAction::Diagnostics => self.page = Page::Diagnostics,
        }
    }

    fn integrated_battlenet_page(&mut self, ui: &mut egui::Ui) {
        let session = self.desktop_session_state();
        let account_label = self.account_display_label();
        let bridge_health = self.model.bridge_health.state.label().to_string();
        let bridge_state = self.model.bridge_state.label().to_string();
        let install_state = state_caption(self.model.install_state).to_string();
        let content_state = self.model.inventory_state.label().to_string();
        let network_state = self.model.network_bridge.state.label().to_string();
        let network_summary = self.model.network_bridge.summary.clone();
        let primary_action = self.model.primary_action();
        let primary_label = self.primary_action_label(primary_action).to_string();
        let primary_enabled = match primary_action {
            PrimaryAction::Install => !self.official_install_active,
            PrimaryAction::Play => self.can_launch_game() && !self.official_game_active,
            PrimaryAction::Index => self.model.inventory_state != InventoryState::Indexing,
            PrimaryAction::Locate => self.model.bridge_state != BridgeState::Verifying,
            PrimaryAction::Repair => true,
        };
        let host_status = self.battle_net_window_host.status().clone();
        let output = render_battlenet_page(
            ui,
            BattleNetPageView {
                account_label: &account_label,
                session,
                bridge_health: &bridge_health,
                bridge_state: &bridge_state,
                bridge_detail: &self.model.bridge_detail,
                battlenet_running: self.model.battlenet_running,
                agent_running: self.model.agent_running,
                diablo_running: self.model.diablo_running,
                install_state: &install_state,
                content_state: &content_state,
                primary_label: &primary_label,
                primary_enabled,
                repair_recommended: self.model.bridge_health.state != BridgeHealthState::Healthy,
                network_state: &network_state,
                network_summary: &network_summary,
                network_repair_recommended: self.model.network_bridge.repair_recommended(),
                network_probe_in_flight: self.network_probe_in_flight,
                host_status: &host_status,
            },
        );

        if let Some(target) = HostRect::new(
            output.host_rect.min.x.round() as i32,
            output.host_rect.min.y.round() as i32,
            output.host_rect.width().round().max(0.0) as u32,
            output.host_rect.height().round().max(0.0) as u32,
        ) {
            let _ = self.battle_net_window_host.sync(target);
        }

        match output.action {
            BattleNetPageAction::None => {}
            BattleNetPageAction::Primary => self.on_primary_action(primary_action),
            BattleNetPageAction::OpenClient => self.open_battlenet(),
            BattleNetPageAction::SignIn => self.sign_in_battlenet(),
            BattleNetPageAction::Account => self.page = Page::Account,
            BattleNetPageAction::GamePage => self.page = Page::Overview,
            BattleNetPageAction::RepairBridge => self.repair_official_bridge(),
            BattleNetPageAction::RepairNetworkBridge => self.repair_network_bridge(),
        }
    }

    fn game_subnav(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(10.0);
            let (icon, _) = ui.allocate_exact_size(Vec2::splat(30.0), egui::Sense::hover());
            paint_mini_diablo_icon(ui, icon, self.page.is_game_page());
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("DIABLO III")
                        .strong()
                        .size(17.0)
                        .color(Color32::from_rgb(226, 234, 241)),
                );
                ui.label(
                    RichText::new("OPEN SANCTUARY • NATIVE LINUX")
                        .small()
                        .color(Color32::from_rgb(98, 126, 149)),
                );
            });
            ui.add_space((ui.available_width() - 252.0).max(0.0));
            inventory_badge(ui, self.model.inventory_state);
        });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            subnav_button(ui, "OVERVIEW", self.page == Page::Overview, || {
                self.page = Page::Overview;
            });
            subnav_button(ui, "CONTENT", self.page == Page::Content, || {
                self.page = Page::Content;
            });
            subnav_button(ui, "ACTIVITY", self.page == Page::Activity, || {
                self.page = Page::Activity;
            });
            subnav_button(ui, "SETTINGS", self.page == Page::Settings, || {
                self.page = Page::Settings;
            });
        });
        ui.separator();
    }

    fn home_page(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.add_space(18.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("HOME")
                                .strong()
                                .size(27.0)
                                .color(Color32::from_rgb(231, 237, 243)),
                        );
                        ui.label(
                            RichText::new(
                                "Featured native runtime updates and your local game library",
                            )
                            .color(Color32::from_rgb(125, 146, 165)),
                        );
                    });
                });
                ui.add_space(14.0);

                let width = ui.available_width().min(1160.0);
                let (feature, _) =
                    ui.allocate_exact_size(Vec2::new(width, 330.0), egui::Sense::hover());
                paint_hero(ui, feature, !self.config.reduced_motion);
                ui.painter().rect_filled(
                    Rect::from_min_max(
                        feature.min,
                        Pos2::new(feature.left() + feature.width() * 0.56, feature.bottom()),
                    ),
                    CornerRadius::same(7),
                    Color32::from_rgba_unmultiplied(4, 8, 13, 205),
                );
                ui.painter().text(
                    Pos2::new(feature.left() + 34.0, feature.top() + 42.0),
                    Align2::LEFT_TOP,
                    "FEATURED GAME",
                    FontId::proportional(12.0),
                    Color32::from_rgb(76, 171, 231),
                );
                ui.painter().text(
                    Pos2::new(feature.left() + 34.0, feature.top() + 75.0),
                    Align2::LEFT_TOP,
                    "DIABLO III",
                    FontId::proportional(40.0),
                    Color32::from_rgb(238, 241, 244),
                );
                ui.painter().text(
                    Pos2::new(feature.left() + 36.0, feature.top() + 130.0),
                    Align2::LEFT_TOP,
                    "Clean-room native Linux runtime",
                    FontId::proportional(16.0),
                    Color32::from_rgb(165, 183, 199),
                );
                ui.painter().text(
                    Pos2::new(feature.left() + 36.0, feature.top() + 160.0),
                    Align2::LEFT_TOP,
                    state_caption(self.model.install_state),
                    FontId::proportional(13.0),
                    Color32::from_rgb(125, 147, 167),
                );
                let open_rect = Rect::from_min_size(
                    Pos2::new(feature.left() + 34.0, feature.bottom() - 76.0),
                    Vec2::new(210.0, 44.0),
                );
                if ui
                    .put(
                        open_rect,
                        egui::Button::new(RichText::new("OPEN GAME PAGE").strong())
                            .fill(Color32::from_rgb(24, 113, 179)),
                    )
                    .clicked()
                {
                    self.page = Page::Overview;
                }

                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("LAST PLAYED")
                            .strong()
                            .size(14.0)
                            .color(Color32::from_rgb(197, 211, 223)),
                    );
                    ui.label(
                        RichText::new("QUICK ACCESS")
                            .small()
                            .color(Color32::from_rgb(90, 113, 133)),
                    );
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if home_last_played_card(
                        ui,
                        "DIABLO III",
                        state_caption(self.model.install_state),
                        self.model.play_ready(),
                    ) {
                        self.page = Page::Overview;
                    }
                    home_last_played_placeholder(ui, "No recent game");
                    home_last_played_placeholder(ui, "No recent game");
                });

                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("LATEST")
                            .strong()
                            .size(16.0)
                            .color(Color32::from_rgb(205, 217, 228)),
                    );
                });
                ui.add_space(8.0);
                ui.columns(3, |columns| {
                    if news_card(
                        &mut columns[0],
                        "CONTENT",
                        "Local Diablo III inventory",
                        "Browse indexed archives, indexes, config files, and install metadata.",
                        true,
                    ) {
                        self.page = Page::Content;
                    }
                    if news_card(
                        &mut columns[1],
                        "RUNTIME",
                        "Native engine readiness",
                        if self.model.diagnostics.vulkan_available {
                            "Vulkan is available for the OpenSanctuary native renderer."
                        } else {
                            "Vulkan readiness needs attention before native launch."
                        },
                        true,
                    ) {
                        self.page = Page::Diagnostics;
                    }
                    if news_card(
                        &mut columns[2],
                        "ACTIVITY",
                        "Downloads & launcher events",
                        &activity_tray_summary(&self.model),
                        true,
                    ) {
                        self.page = Page::Activity;
                    }
                });
            });
    }

    fn shop_page(&mut self, ui: &mut egui::Ui) {
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            ui.add_space(18.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("SHOP")
                        .strong()
                        .size(27.0)
                        .color(Color32::from_rgb(231, 237, 243)),
                );
                ui.label(
                    RichText::new("OpenSanctuary does not connect to Blizzard commerce services")
                        .color(Color32::from_rgb(125, 146, 165)),
                );
            });
        });
        ui.add_space(18.0);
        let width = ui.available_width().min(1050.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 300.0), egui::Sense::hover());
        paint_hero(ui, rect, !self.config.reduced_motion);
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(7),
            Color32::from_rgba_unmultiplied(7, 11, 17, 186),
        );
        ui.painter().text(
            rect.center_top() + Vec2::new(0.0, 70.0),
            Align2::CENTER_TOP,
            "NO STOREFRONT CONNECTED",
            FontId::proportional(28.0),
            Color32::from_rgb(221, 229, 236),
        );
        ui.painter().text(
            rect.center_top() + Vec2::new(0.0, 120.0),
            Align2::CENTER_TOP,
            "This clean-room launcher manages your local installation only.",
            FontId::proportional(14.0),
            Color32::from_rgb(147, 166, 184),
        );
        ui.painter().text(
            rect.center_top() + Vec2::new(0.0, 150.0),
            Align2::CENTER_TOP,
            "Purchases, account billing, and Blizzard shop content remain outside OpenSanctuary.",
            FontId::proportional(12.0),
            Color32::from_rgb(113, 136, 156),
        );
    }

    fn games_page(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.add_space(18.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("GAMES")
                        .strong()
                        .size(28.0)
                        .color(Color32::from_rgb(231, 237, 243)),
                );
                ui.label(
                    RichText::new("Your native OpenSanctuary library")
                        .color(Color32::from_rgb(126, 148, 168)),
                );
            });
        });
        ui.add_space(16.0);

        let width = ui.available_width();
        let feature_width = width.min(1120.0);
        let (feature, _) =
            ui.allocate_exact_size(Vec2::new(feature_width, 350.0), egui::Sense::hover());
        paint_hero(ui, feature, !self.config.reduced_motion);
        ui.painter().rect_filled(
            Rect::from_min_max(
                Pos2::new(feature.left(), feature.top()),
                Pos2::new(feature.left() + feature.width() * 0.52, feature.bottom()),
            ),
            CornerRadius::same(7),
            Color32::from_rgba_unmultiplied(5, 9, 14, 188),
        );
        ui.painter().text(
            Pos2::new(feature.left() + 34.0, feature.top() + 48.0),
            Align2::LEFT_TOP,
            "FEATURED GAME",
            FontId::proportional(12.0),
            Color32::from_rgb(87, 172, 229),
        );
        ui.painter().text(
            Pos2::new(feature.left() + 34.0, feature.top() + 78.0),
            Align2::LEFT_TOP,
            "DIABLO III",
            FontId::proportional(38.0),
            Color32::from_rgb(238, 241, 244),
        );
        ui.painter().text(
            Pos2::new(feature.left() + 36.0, feature.top() + 130.0),
            Align2::LEFT_TOP,
            state_caption(self.model.install_state),
            FontId::proportional(15.0),
            Color32::from_rgb(164, 181, 197),
        );
        if let Some(inventory) = &self.model.inventory {
            let version = inventory
                .build
                .version
                .as_deref()
                .unwrap_or("Unknown version");
            ui.painter().text(
                Pos2::new(feature.left() + 36.0, feature.top() + 158.0),
                Align2::LEFT_TOP,
                version,
                FontId::proportional(13.0),
                Color32::from_rgb(120, 143, 163),
            );
        }

        let action = self.model.primary_action();
        let action_rect = Rect::from_min_size(
            Pos2::new(feature.left() + 34.0, feature.bottom() - 82.0),
            Vec2::new(230.0, 48.0),
        );
        let action_enabled = match action {
            PrimaryAction::Install => !self.official_install_active,
            PrimaryAction::Play => self.can_launch_game() && !self.official_game_active,
            PrimaryAction::Index => self.model.inventory_state != InventoryState::Indexing,
            PrimaryAction::Locate => self.model.bridge_state != BridgeState::Verifying,
            PrimaryAction::Repair => true,
        };
        let action_label = self.primary_action_label(action);
        let action_response = ui.put(
            action_rect,
            egui::Button::new(RichText::new(action_label).strong().size(16.0))
                .fill(if action_enabled {
                    Color32::from_rgb(23, 118, 184)
                } else {
                    Color32::from_rgb(39, 51, 62)
                })
                .sense(if action_enabled {
                    egui::Sense::click()
                } else {
                    egui::Sense::hover()
                }),
        );
        if action_enabled && action_response.clicked() {
            self.on_primary_action(action);
        }
        let view_rect = Rect::from_min_size(
            Pos2::new(feature.left() + 278.0, feature.bottom() - 82.0),
            Vec2::new(142.0, 48.0),
        );
        if ui.put(view_rect, egui::Button::new("GAME PAGE")).clicked() {
            self.page = Page::Overview;
        }
        ui.add_space(18.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(
                RichText::new("YOUR GAMES")
                    .strong()
                    .size(16.0)
                    .color(Color32::from_rgb(200, 214, 226)),
            );
        });
        ui.add_space(8.0);
        let mut open_diablo = false;
        ui.columns(3, |columns| {
            let diablo_library_status = if self.model.install_path.is_some() {
                "Installed • OpenSanctuary managed"
            } else {
                "Not installed • Battle.net setup available"
            };
            open_diablo = library_card(&mut columns[0], "DIABLO III", diablo_library_status, true);
            library_card(
                &mut columns[1],
                "AETHERFORGE",
                "Future OpenSanctuary target",
                false,
            );
            library_card(
                &mut columns[2],
                "ADD GAME",
                "More clean-room runtimes later",
                false,
            );
        });
        if open_diablo {
            self.page = Page::Overview;
        }
    }

    fn overview_page(&mut self, ui: &mut egui::Ui) {
        self.game_subnav(ui);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(8.0);
                let primary_action = self.model.primary_action();
                let primary_enabled = match primary_action {
                    PrimaryAction::Install => !self.official_install_active,
                    PrimaryAction::Play => self.can_launch_game() && !self.official_game_active,
                    PrimaryAction::Index => self.model.inventory_state != InventoryState::Indexing,
                    PrimaryAction::Locate => self.model.bridge_state != BridgeState::Verifying,
                    PrimaryAction::Repair => true,
                };
                let readiness = if self.can_launch_game() {
                    GameReadiness::Ready
                } else if matches!(
                    self.model.bridge_state,
                    BridgeState::Installing
                        | BridgeState::Updating
                        | BridgeState::Verifying
                        | BridgeState::Indexing
                        | BridgeState::Starting
                ) {
                    GameReadiness::Working
                } else {
                    GameReadiness::NeedsAttention
                };
                let build_version = self
                    .model
                    .inventory
                    .as_ref()
                    .and_then(|inventory| inventory.build.version.clone())
                    .unwrap_or_else(|| "Version unknown".into());
                let battle_net_caption = if self.model.battlenet_running {
                    "Signed in / running"
                } else if self.official_discovery.battlenet_ready() {
                    "Available"
                } else {
                    "Setup required"
                }
                .to_owned();
                let install_caption = state_caption(self.model.install_state).to_owned();
                let content_caption = self.model.inventory_state.label().to_owned();
                let status_intent = if primary_action == PrimaryAction::Install {
                    SessionIntent::Install
                } else {
                    SessionIntent::Play
                };
                let session_context = self.preparation_context();
                let session_status = prepare_session(&session_context, status_intent)
                    .label()
                    .to_owned();
                let view = GamePageView {
                    primary_label: self.primary_action_label(primary_action).to_owned(),
                    primary_enabled,
                    readiness,
                    build_version,
                    install_caption,
                    battle_net_caption,
                    content_caption,
                    message: self.model.message.clone(),
                    session_status,
                    reduced_motion: self.config.reduced_motion,
                };
                let hero_action = render_game_hero(ui, &view);
                self.handle_game_page_action(hero_action, primary_action);
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width((ui.available_width() - 294.0).max(460.0));
                        self.featured_update_panel(ui);
                    });
                    ui.add_space(12.0);
                    ui.vertical(|ui| {
                        ui.set_width(282.0);
                        let status_action = render_game_status(ui, &view);
                        self.handle_game_page_action(status_action, primary_action);
                    });
                });
                ui.add_space(12.0);
                self.latest_content_cards(ui);
            });
    }

    fn handle_game_page_action(&mut self, action: GamePageAction, primary_action: PrimaryAction) {
        match action {
            GamePageAction::None => {}
            GamePageAction::Primary => self.on_primary_action(primary_action),
            GamePageAction::Settings => self.page = Page::Settings,
            GamePageAction::Details => self.page = Page::Diagnostics,
            GamePageAction::OpenBattleNet => self.open_battlenet(),
            GamePageAction::Content => self.page = Page::Content,
        }
    }

    fn featured_update_panel(&mut self, ui: &mut egui::Ui) {
        let width = ui.available_width().max(420.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 355.0), egui::Sense::hover());
        let now = ui.input(|input| input.time);
        let selected = self.featured_carousel.current();
        let card = featured_card(selected);
        let progress = transition_progress(
            self.config.reduced_motion,
            now,
            self.featured_transition_started,
            0.28,
        );
        let slide = if self.config.reduced_motion {
            0.0
        } else {
            (1.0 - progress) * 16.0
        };

        paint_hero(ui, rect, !self.config.reduced_motion);
        paint_featured_accent(ui, rect, selected, self.config.reduced_motion, now);

        let overlay = Rect::from_min_max(
            Pos2::new(rect.left(), rect.top()),
            Pos2::new(rect.left() + rect.width() * 0.49, rect.bottom()),
        );
        ui.painter().rect_filled(
            overlay,
            CornerRadius::same(7),
            Color32::from_rgba_unmultiplied(4, 8, 13, 204),
        );
        let accent = Color32::from_rgb(card.accent.0, card.accent.1, card.accent.2);
        ui.painter().rect_filled(
            Rect::from_min_max(
                Pos2::new(overlay.left(), overlay.top()),
                Pos2::new(overlay.left() + 3.0, overlay.bottom()),
            ),
            CornerRadius::ZERO,
            accent,
        );
        ui.painter().text(
            Pos2::new(rect.left() + 28.0 + slide, rect.top() + 36.0),
            Align2::LEFT_TOP,
            card.eyebrow,
            FontId::proportional(11.0),
            accent,
        );
        ui.painter().text(
            Pos2::new(rect.left() + 28.0 + slide, rect.top() + 68.0),
            Align2::LEFT_TOP,
            card.title,
            FontId::proportional(29.0),
            Color32::from_rgb(238, 241, 244),
        );
        ui.painter().text(
            Pos2::new(rect.left() + 30.0 + slide, rect.top() + 150.0),
            Align2::LEFT_TOP,
            card.detail,
            FontId::proportional(13.0),
            Color32::from_rgb(160, 180, 197),
        );

        let action_rect = Rect::from_min_size(
            Pos2::new(rect.left() + 28.0, rect.bottom() - 68.0),
            Vec2::new(158.0, 38.0),
        );
        if ui
            .put(
                action_rect,
                egui::Button::new(RichText::new(card.action).strong())
                    .fill(Color32::from_rgb(24, 92, 151)),
            )
            .clicked()
        {
            self.page = card.target;
        }

        let previous_rect = Rect::from_min_size(
            Pos2::new(rect.right() - 88.0, rect.bottom() - 52.0),
            Vec2::new(32.0, 32.0),
        );
        if ui
            .put(
                previous_rect,
                egui::Button::new(RichText::new("‹").size(20.0)).frame(false),
            )
            .on_hover_text("Previous feature")
            .clicked()
        {
            self.featured_carousel.previous();
            self.featured_transition_started = now;
        }
        let next_rect = Rect::from_min_size(
            Pos2::new(rect.right() - 48.0, rect.bottom() - 52.0),
            Vec2::new(32.0, 32.0),
        );
        if ui
            .put(
                next_rect,
                egui::Button::new(RichText::new("›").size(20.0)).frame(false),
            )
            .on_hover_text("Next feature")
            .clicked()
        {
            self.featured_carousel.next();
            self.featured_transition_started = now;
        }

        for index in 0..3 {
            let dot_rect = Rect::from_min_size(
                Pos2::new(
                    rect.center().x - 28.0 + index as f32 * 24.0,
                    rect.bottom() - 40.0,
                ),
                Vec2::new(20.0, 20.0),
            );
            let selected_dot = self.featured_carousel.current() == index;
            if ui
                .put(
                    dot_rect,
                    egui::Button::new(
                        RichText::new(if selected_dot { "●" } else { "○" })
                            .small()
                            .color(if selected_dot {
                                Color32::from_rgb(78, 174, 232)
                            } else {
                                Color32::from_rgb(98, 118, 136)
                            }),
                    )
                    .frame(false),
                )
                .clicked()
            {
                self.featured_carousel.select(index);
                self.featured_transition_started = now;
            }
        }
    }

    fn latest_content_cards(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("LATEST STORIES")
                    .strong()
                    .size(15.0)
                    .color(Color32::from_rgb(199, 213, 225)),
            );
            ui.label(
                RichText::new("LOCAL GAME & RUNTIME")
                    .small()
                    .color(Color32::from_rgb(91, 115, 136)),
            );
            ui.add_space((ui.available_width() - 76.0).max(0.0));
            if ui
                .add(egui::Button::new(RichText::new("VIEW ALL").small()).frame(false))
                .clicked()
            {
                self.page = Page::Activity;
            }
        });
        ui.add_space(6.0);
        ui.columns(2, |columns| {
            if latest_feature_card(
                &mut columns[0],
                "CONTENT INDEX",
                "Your Diablo III installation is mapped locally",
                "Search archives, indexes, config files, and build metadata without modifying the installed game.",
                self.model.inventory_state == InventoryState::Indexed,
            ) {
                self.page = Page::Content;
            }
            columns[1].vertical(|ui| {
                if compact_news_card(
                    ui,
                    "SYSTEM",
                    "Native diagnostics",
                    if self.model.diagnostics.vulkan_available {
                        "Vulkan renderer ready"
                    } else {
                        "Review renderer readiness"
                    },
                    true,
                ) {
                    self.page = Page::Diagnostics;
                }
                ui.add_space(8.0);
                if compact_news_card(
                    ui,
                    "ACTIVITY",
                    "Downloads & launcher events",
                    &activity_tray_summary(&self.model),
                    true,
                ) {
                    self.page = Page::Activity;
                }
            });
        });
        ui.add_space(8.0);
        ui.columns(3, |columns| {
            if story_strip_card(
                &mut columns[0],
                "BUILD",
                "Installation metadata",
                self.model
                    .inventory
                    .as_ref()
                    .and_then(|inventory| inventory.build.version.as_deref())
                    .unwrap_or("Index to discover version"),
            ) {
                self.page = Page::Content;
            }
            if story_strip_card(
                &mut columns[1],
                "PREFERENCES",
                "Launcher settings",
                if self.config.reduced_motion {
                    "Reduced motion enabled"
                } else {
                    "Motion effects enabled"
                },
            ) {
                self.page = Page::Settings;
            }
            if story_strip_card(
                &mut columns[2],
                "NATIVE STACK",
                "Runtime readiness",
                if self.model.play_ready() {
                    "Ready for native launch"
                } else {
                    "Review readiness checks"
                },
            ) {
                self.page = Page::Diagnostics;
            }
        });
    }
    fn content_page(&mut self, ui: &mut egui::Ui) {
        self.game_subnav(ui);
        ui.horizontal(|ui| {
            ui.heading("Local Content Inventory");
            ui.add_space(12.0);
            inventory_badge(ui, self.model.inventory_state);
            if self.model.inventory_state != InventoryState::Indexing
                && ui.button("Re-index").clicked()
                && let Some(path) = self.model.install_path.clone()
            {
                self.start_index(path);
            }
        });
        ui.label(
            RichText::new(
                "Read-only view of files physically present under the Diablo III Data/ tree. No payloads are extracted in v0.2.",
            )
            .color(Color32::from_rgb(145, 162, 180)),
        );
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.content_search)
                    .desired_width(360.0)
                    .hint_text("Search relative paths…"),
            );
            for filter in [
                ContentFilter::All,
                ContentFilter::Archive,
                ContentFilter::Index,
                ContentFilter::Config,
                ContentFilter::Other,
            ] {
                if ui
                    .selectable_label(self.content_filter == filter, filter.label())
                    .clicked()
                {
                    self.content_filter = filter;
                }
            }
        });
        ui.separator();

        let Some(inventory) = self.model.inventory.as_ref() else {
            ui.add_space(24.0);
            ui.label("No content inventory is loaded yet. Use INDEX CONTENT on Overview.");
            return;
        };

        let search = self.content_search.trim().to_ascii_lowercase();
        let filter = self.content_filter;
        let matches = inventory
            .entries
            .iter()
            .filter(|entry| filter.matches(entry.kind))
            .filter(|entry| {
                search.is_empty() || entry.relative_path.to_ascii_lowercase().contains(&search)
            })
            .collect::<Vec<_>>();

        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("{} entries", matches.len()))
                    .strong()
                    .color(Color32::from_rgb(193, 207, 220)),
            );
            ui.label(format!(
                "{} total • fingerprint {}",
                format_bytes(inventory.total_bytes),
                compact_fingerprint(&inventory.fingerprint)
            ));
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_sized(
                [140.0, 20.0],
                egui::Label::new(RichText::new("KIND").strong()),
            );
            ui.add_sized(
                [120.0, 20.0],
                egui::Label::new(RichText::new("SIZE").strong()),
            );
            ui.label(RichText::new("RELATIVE PATH").strong());
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for entry in matches {
                    ui.horizontal(|ui| {
                        ui.add_sized([140.0, 20.0], egui::Label::new(kind_rich_text(entry.kind)));
                        ui.add_sized(
                            [120.0, 20.0],
                            egui::Label::new(
                                RichText::new(format_bytes(entry.byte_len)).monospace(),
                            ),
                        );
                        ui.monospace(&entry.relative_path);
                    });
                }
            });
    }

    fn activity_page(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.heading("Downloads & Activity");
            ui.add_space(8.0);
            let active = self.model.activity.iter().filter(|item| !item.done).count();
            if active > 0 {
                ui.label(
                    RichText::new(format!("{active} ACTIVE"))
                        .small()
                        .strong()
                        .color(Color32::from_rgb(68, 164, 226)),
                );
            }
        });
        ui.label(
            RichText::new("Native scans, indexes, cache operations, and engine lifecycle events")
                .color(Color32::from_rgb(123, 145, 164)),
        );
        ui.add_space(10.0);
        if self.model.activity.is_empty() {
            drawer_message(
                ui,
                "No activity yet",
                "Launcher jobs and native engine events will appear here.",
            );
            return;
        }
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for item in self.model.activity.iter().rev() {
                    activity_row(ui, item, self.config.reduced_motion, true);
                    ui.add_space(7.0);
                }
            });
    }

    fn settings_page(&mut self, ui: &mut egui::Ui) {
        self.game_subnav(ui);
        ui.heading("Settings");
        ui.add_space(8.0);
        ui.label("Diablo III installation");
        ui.horizontal(|ui| {
            ui.monospace(
                self.config
                    .install_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Not configured".into()),
            );
            if ui.button("Locate…").clicked() {
                self.open_locate();
            }
        });
        ui.label(format!(
            "Inventory cache: {}",
            default_inventory_cache_path().display()
        ));
        if ui
            .checkbox(&mut self.config.reduced_motion, "Reduce launcher animation")
            .changed()
        {
            let _ = self.config.save_to(&xdg_config_path());
        }
        if ui
            .checkbox(
                &mut self.config.show_desktop_notifications,
                "Desktop notifications",
            )
            .changed()
        {
            let _ = self.config.save_to(&xdg_config_path());
        }
        ui.separator();
        ui.label(RichText::new("Battle.net bridge").strong());
        ui.monospace(self.bridge_record_path.display().to_string());
        ui.horizontal(|ui| {
            if ui.button("REPAIR SOFTWARE BRIDGE").clicked() {
                self.repair_official_bridge();
            }
            ui.label(
                RichText::new("Resets OpenSanctuary bridge metadata and safely rediscovers Battle.net/Diablo III")
                    .small()
                    .color(Color32::from_rgb(112, 137, 158)),
            );
        });
        ui.separator();
        ui.label(format!(
            "Renderer: {}",
            if self.model.diagnostics.vulkan_available {
                "Vulkan available"
            } else {
                "Vulkan unavailable"
            }
        ));
        ui.label(format!(
            "Audio: {}",
            if self.model.diagnostics.pipewire_available {
                "PipeWire available"
            } else {
                "PipeWire unavailable"
            }
        ));
    }

    fn diagnostics_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Advanced Diagnostics");
        ui.label(
            RichText::new("Bridge, installation, content, renderer, and audio details")
                .color(Color32::from_rgb(116, 141, 161)),
        );
        ui.add_space(10.0);
        render_advanced_diagnostics(
            ui,
            AdvancedDiagnosticsView {
                bridge_health: self.model.bridge_health.state.label(),
                bridge_state: self.model.bridge_state.label(),
                runner: self.official_discovery.runner.as_deref(),
                prefix: self.official_discovery.runtime_prefix(),
                launcher: self.official_discovery.launcher_exe.as_deref(),
                install: self.model.install_path.as_deref(),
                battlenet_running: self.model.battlenet_running,
                agent_running: self.model.agent_running,
                content_state: self.model.inventory_state.label(),
                last_recovery: self.model.last_bridge_recovery.as_deref(),
                network_report: &self.model.network_bridge,
            },
        );
        ui.add_space(12.0);
        ui.collapsing("Native runtime report", |ui| {
            ui.monospace(self.model.diagnostics.to_text());
            if let Some(inventory) = &self.model.inventory {
                ui.label(format!(
                    "Inventory: {} entries • {} • {}",
                    inventory.entries.len(),
                    format_bytes(inventory.total_bytes),
                    inventory.fingerprint
                ));
            }
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button("Export JSON").clicked() {
                let path = self.diagnostics_export_path("json");
                if export_diagnostics(&self.model.diagnostics, &path, DiagnosticsFormat::Json)
                    .is_ok()
                {
                    self.last_export = Some(path);
                }
            }
            if ui.button("Export text").clicked() {
                let path = self.diagnostics_export_path("txt");
                if export_diagnostics(&self.model.diagnostics, &path, DiagnosticsFormat::Text)
                    .is_ok()
                {
                    self.last_export = Some(path);
                }
            }
            if ui.button("REPAIR SOFTWARE BRIDGE").clicked() {
                self.repair_official_bridge();
            }
        });
        if let Some(path) = &self.last_export {
            ui.label(format!("Exported: {}", path.display()));
        }
        ui.label(
            RichText::new("Diagnostics contain no account credentials.")
                .small()
                .color(Color32::from_rgb(105, 131, 151)),
        );
    }

    fn activity_tray(&mut self, ui: &mut egui::Ui) {
        let view = DownloadsView {
            bridge_state: self.model.bridge_state,
            inventory_state: self.model.inventory_state,
            activity: &self.model.activity,
            reduced_motion: self.config.reduced_motion,
            expanded: self.activity_open,
        };
        match render_downloads_tray(ui, view) {
            DownloadsAction::None => {}
            DownloadsAction::Toggle => self.activity_open = !self.activity_open,
            DownloadsAction::ViewAll => self.page = Page::Activity,
            DownloadsAction::Details => self.page = Page::Diagnostics,
        }
    }

    fn open_locate(&mut self) {
        if self.locate_text.is_empty() {
            self.locate_text = std::env::var_os("HOME")
                .map(PathBuf::from)
                .and_then(|home| discover_candidates(&home).into_iter().next())
                .map(|first| first.display().to_string())
                .unwrap_or_default();
        }
        self.locate_open = true;
    }

    fn locate_window(&mut self, ctx: &egui::Context) {
        if !self.locate_open {
            return;
        }
        let mut open = self.locate_open;
        let mut confirm = false;
        let mut cancel = false;
        egui::Window::new("Locate Diablo III")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Select the root directory containing .build.info and Data/.");
                ui.add(egui::TextEdit::singleline(&mut self.locate_text).desired_width(520.0));
                ui.horizontal(|ui| {
                    if ui.button("Use this folder").clicked() {
                        confirm = true;
                    }
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                });
            });
        if cancel {
            open = false;
        }
        if confirm {
            let path = PathBuf::from(self.locate_text.trim());
            self.start_probe(path);
            self.page = Page::Overview;
            open = false;
        }
        self.locate_open = open;
    }

    fn on_primary_action(&mut self, action: PrimaryAction) {
        match action {
            PrimaryAction::Install => {
                let decision = self.prepare_for(SessionIntent::Install);
                self.execute_preparation(decision, SessionIntent::Install);
            }
            PrimaryAction::Locate => self.open_locate(),
            PrimaryAction::Index => {
                let decision = self.prepare_for(SessionIntent::Play);
                self.execute_preparation(decision, SessionIntent::Play);
            }
            PrimaryAction::Repair => {
                if let Some(path) = self
                    .model
                    .install_path
                    .clone()
                    .or_else(|| self.config.install_path.clone())
                {
                    self.start_probe(path);
                } else {
                    self.repair_official_bridge();
                }
            }
            PrimaryAction::Play => {
                let decision = self.prepare_for(SessionIntent::Play);
                self.execute_preparation(decision, SessionIntent::Play);
            }
        }
    }
}

impl Drop for LauncherApp {
    fn drop(&mut self) {
        self.battle_net_window_host.release();
    }
}

impl eframe::App for LauncherApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_session_supervisor();
        self.poll_network_bridge();
        self.process_events();
        self.poll_engine();
        ui.ctx()
            .request_repaint_after(if self.config.reduced_motion {
                Duration::from_secs(1)
            } else {
                Duration::from_millis(33)
            });

        egui::Panel::top("top_bar")
            .exact_size(52.0)
            .show(ui, |ui| self.top_bar(ui));
        egui::Panel::top("game_strip")
            .exact_size(60.0)
            .show(ui, |ui| self.game_strip(ui));
        egui::Panel::bottom("activity_tray")
            .resizable(false)
            .show(ui, |ui| self.activity_tray(ui));
        if self.utility_open.is_some() {
            egui::Panel::right("utility_drawer")
                .exact_size(316.0)
                .resizable(false)
                .show(ui, |ui| self.utility_drawer(ui));
        }
        egui::CentralPanel::default().show(ui, |ui| match self.page {
            Page::Home => self.home_page(ui),
            Page::Games => self.games_page(ui),
            Page::BattleNet => self.integrated_battlenet_page(ui),
            Page::Account => self.account_page(ui),
            Page::Shop => self.shop_page(ui),
            Page::Overview => self.overview_page(ui),
            Page::Content => self.content_page(ui),
            Page::Activity => self.activity_page(ui),
            Page::Settings => self.settings_page(ui),
            Page::Diagnostics => self.diagnostics_page(ui),
        });
        if self.page != Page::BattleNet {
            self.battle_net_window_host.release();
        }
        self.locate_window(ui.ctx());
    }
}

fn search_route(query: &str) -> Option<Page> {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return None;
    }
    if ["content", "files", "archive", "casc"]
        .iter()
        .any(|term| query.contains(term))
    {
        return Some(Page::Content);
    }
    if ["battle.net", "battlenet", "blizzard", "login", "install"]
        .iter()
        .any(|term| query.contains(term))
    {
        return Some(Page::BattleNet);
    }
    if ["account", "battletag", "profile"]
        .iter()
        .any(|term| query.contains(term))
    {
        return Some(Page::Account);
    }
    if ["settings", "options", "preferences"]
        .iter()
        .any(|term| query.contains(term))
    {
        return Some(Page::Settings);
    }
    if ["diagnostics", "system", "vulkan", "pipewire"]
        .iter()
        .any(|term| query.contains(term))
    {
        return Some(Page::Diagnostics);
    }
    if ["downloads", "activity", "tasks"]
        .iter()
        .any(|term| query.contains(term))
    {
        return Some(Page::Activity);
    }
    if ["diablo", "overview", "play"]
        .iter()
        .any(|term| query.contains(term))
    {
        return Some(Page::Overview);
    }
    if ["games", "library", "favorites"]
        .iter()
        .any(|term| query.contains(term))
    {
        return Some(Page::Games);
    }
    if query.contains("shop") {
        return Some(Page::Shop);
    }
    if query.contains("home") {
        return Some(Page::Home);
    }
    None
}

fn utility_drawer_header(ui: &mut egui::Ui, title: &str, open: &mut Option<UtilityDrawer>) {
    ui.horizontal(|ui| {
        ui.heading(title);
        ui.add_space((ui.available_width() - 28.0).max(0.0));
        if ui.button("×").clicked() {
            *open = None;
        }
    });
}

#[derive(Debug, Clone, Copy)]
struct FeaturedCard {
    eyebrow: &'static str,
    title: &'static str,
    detail: &'static str,
    action: &'static str,
    target: Page,
    accent: (u8, u8, u8),
}

fn featured_card(index: usize) -> FeaturedCard {
    match index % 3 {
        0 => FeaturedCard {
            eyebrow: "NATIVE RUNTIME UPDATE",
            title: "SANCTUARY,\nREBUILT FOR LINUX",
            detail: "Index your local Diablo III installation and\nprepare it for the clean-room native engine.",
            action: "VIEW CONTENT",
            target: Page::Content,
            accent: (79, 173, 232),
        },
        1 => FeaturedCard {
            eyebrow: "LOCAL CONTENT",
            title: "YOUR SANCTUARY.\nYOUR INSTALL.",
            detail: "Browse the read-only local storage inventory,\nbuild metadata, archives, and indexes.",
            action: "OPEN INVENTORY",
            target: Page::Content,
            accent: (210, 125, 57),
        },
        _ => FeaturedCard {
            eyebrow: "NATIVE STACK",
            title: "VULKAN. PIPEWIRE.\nLINUX NATIVE.",
            detail: "Review renderer, audio, and runtime readiness\nbefore launching the clean-room engine.",
            action: "DIAGNOSTICS",
            target: Page::Diagnostics,
            accent: (87, 191, 152),
        },
    }
}

#[cfg(test)]
mod ui_state_tests {
    use super::*;
    use crate::widgets::{ActivityVisualState, activity_progress_fraction, activity_visual_state};
    use sanctuary_launcher::ActivityItem;

    #[test]
    fn selected_game_pages_are_classified_separately_from_launcher_pages() {
        assert!(Page::Overview.is_game_page());
        assert!(Page::Content.is_game_page());
        assert!(Page::Settings.is_game_page());
        assert!(Page::Diagnostics.is_game_page());
        assert!(!Page::Games.is_game_page());
        assert!(!Page::Activity.is_game_page());
    }

    #[test]
    fn launcher_navigation_classifies_home_games_and_shop() {
        assert!(Page::Home.is_launcher_page());
        assert!(Page::Games.is_launcher_page());
        assert!(Page::Shop.is_launcher_page());
        assert!(Page::BattleNet.is_launcher_page());
        assert!(Page::Account.is_launcher_page());
        assert!(!Page::Overview.is_launcher_page());
        assert!(!Page::Activity.is_launcher_page());

        assert!(Page::Home.home_nav_active());
        assert!(Page::Games.games_nav_active());
        assert!(Page::Overview.games_nav_active());
        assert!(Page::Content.games_nav_active());
        assert!(Page::Shop.shop_nav_active());
        assert!(Page::BattleNet.battlenet_nav_active());
        assert!(Page::Account.battlenet_nav_active());
        assert!(!Page::Overview.home_nav_active());
    }

    #[test]
    fn utility_drawer_toggle_is_mutually_exclusive() {
        let state = toggle_utility_drawer(None, UtilityDrawer::Notifications);
        assert_eq!(state, Some(UtilityDrawer::Notifications));

        let state = toggle_utility_drawer(state, UtilityDrawer::Account);
        assert_eq!(state, Some(UtilityDrawer::Account));

        let state = toggle_utility_drawer(state, UtilityDrawer::Account);
        assert_eq!(state, None);
    }

    #[test]
    fn activity_notification_count_tracks_unfinished_items() {
        let mut model = LauncherModel::default();
        assert_eq!(activity_notification_count(&model), 0);

        model.record_task(TaskEvent::Started {
            label: "Index Diablo III content".into(),
        });
        assert_eq!(activity_notification_count(&model), 1);

        model.record_task(TaskEvent::Finished {
            label: "Index Diablo III content".into(),
        });
        assert_eq!(activity_notification_count(&model), 0);
    }

    #[test]
    fn activity_summary_prefers_active_count_then_latest_completed_item() {
        let mut model = LauncherModel::default();
        assert_eq!(activity_tray_summary(&model), "Idle");

        model.record_task(TaskEvent::Started {
            label: "Index Diablo III content".into(),
        });
        assert_eq!(activity_tray_summary(&model), "1 active");

        model.record_task(TaskEvent::Finished {
            label: "Index Diablo III content".into(),
        });
        assert_eq!(
            activity_tray_summary(&model),
            "Index Diablo III content • Complete"
        );
    }

    #[test]
    fn launcher_search_routes_only_local_surfaces() {
        assert_eq!(search_route("casc archives"), Some(Page::Content));
        assert_eq!(search_route("game settings"), Some(Page::Settings));
        assert_eq!(search_route("vulkan diagnostics"), Some(Page::Diagnostics));
        assert_eq!(search_route("downloads"), Some(Page::Activity));
        assert_eq!(search_route("diablo iii"), Some(Page::Overview));
        assert_eq!(search_route("my games"), Some(Page::Games));
        assert_eq!(search_route("battle.net login"), Some(Page::BattleNet));
        assert_eq!(search_route("battle account"), Some(Page::Account));
        assert_eq!(search_route("unknown remote service"), None);
        assert_eq!(search_route("   "), None);
    }

    #[test]
    fn activity_visual_state_distinguishes_active_complete_and_failed() {
        let active = ActivityItem {
            label: "Index".into(),
            detail: "Running".into(),
            done: false,
            failed: false,
        };
        let complete = ActivityItem {
            done: true,
            ..active.clone()
        };
        let failed = ActivityItem {
            failed: true,
            ..active.clone()
        };

        assert_eq!(activity_visual_state(&active), ActivityVisualState::Active);
        assert_eq!(
            activity_visual_state(&complete),
            ActivityVisualState::Complete
        );
        assert_eq!(activity_visual_state(&failed), ActivityVisualState::Failed);
        assert_eq!(activity_progress_fraction(&active, 0.0), 0.18);
        assert_eq!(activity_progress_fraction(&active, 1.0), 0.8);
        assert_eq!(activity_progress_fraction(&complete, 0.0), 1.0);
        assert_eq!(activity_progress_fraction(&failed, 0.0), 1.0);
    }
}

#[cfg(test)]
mod seamless_flow_tests {
    use super::*;

    #[test]
    fn direct_play_does_not_require_battlenet_surface() {
        assert!(!preparation_surfaces_battlenet(
            PreparationDecision::LaunchGame,
            SessionIntent::Play,
        ));
    }

    #[test]
    fn background_update_does_not_require_battlenet_surface() {
        assert!(!preparation_surfaces_battlenet(
            PreparationDecision::WaitForUpdate,
            SessionIntent::Play,
        ));
    }

    #[test]
    fn install_and_sign_in_require_battlenet_surface() {
        assert!(preparation_surfaces_battlenet(
            PreparationDecision::ShowBattleNetInteraction,
            SessionIntent::Install,
        ));
        assert!(preparation_surfaces_battlenet(
            PreparationDecision::ShowBattleNetInteraction,
            SessionIntent::SignIn,
        ));
    }
}
