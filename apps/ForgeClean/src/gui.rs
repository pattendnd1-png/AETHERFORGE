use crate::gui_state::{
    ActionResult, CoreAction, DashboardSnapshot, GUI_VERSION, GuiSettings,
    collect_dashboard_snapshot, load_settings, run_core_action, save_settings,
};
use eframe::egui;
use std::collections::VecDeque;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

pub const DRAGONGLASS_WINDOW_ALPHA: u8 = 26;
const REFRESH_SECONDS: u64 = 5;
const NAV_WIDTH: f32 = 184.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Dashboard,
    Projects,
    Inbox,
    Cleanup,
    ColdPack,
    Storage,
    Activity,
    Settings,
    Diagnostics,
}

impl Page {
    const ALL: [Self; 9] = [
        Self::Dashboard,
        Self::Projects,
        Self::Inbox,
        Self::Cleanup,
        Self::ColdPack,
        Self::Storage,
        Self::Activity,
        Self::Settings,
        Self::Diagnostics,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Projects => "Projects",
            Self::Inbox => "Downloads / Inbox",
            Self::Cleanup => "Cleanup",
            Self::ColdPack => "ColdPack",
            Self::Storage => "Storage",
            Self::Activity => "Activity",
            Self::Settings => "Settings",
            Self::Diagnostics => "About / Diagnostics",
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            Self::Dashboard => "◇",
            Self::Projects => "▣",
            Self::Inbox => "⇣",
            Self::Cleanup => "✦",
            Self::ColdPack => "◈",
            Self::Storage => "▰",
            Self::Activity => "≋",
            Self::Settings => "⚙",
            Self::Diagnostics => "⌁",
        }
    }
}

#[derive(Debug, Clone)]
enum PendingConfirmation {
    Cleanup,
    ColdPackGc,
    RunArtifact(PathBuf),
}

#[derive(Debug)]
enum WorkerRequest {
    Refresh(PathBuf),
    Action(CoreAction, PathBuf),
}

#[derive(Debug)]
enum WorkerEvent {
    Snapshot(Box<Result<DashboardSnapshot, String>>),
    Action(ActionResult),
}

pub struct ForgeCleanGui {
    page: Page,
    downloads: PathBuf,
    snapshot: DashboardSnapshot,
    settings: GuiSettings,
    settings_dirty: bool,
    selected_project: Option<String>,
    selected_build: Option<String>,
    pending_confirmation: Option<PendingConfirmation>,
    tx: Sender<WorkerRequest>,
    rx: Receiver<WorkerEvent>,
    refresh_pending: bool,
    action_pending: bool,
    last_refresh: Instant,
    activity: VecDeque<String>,
    status_line: String,
}

impl ForgeCleanGui {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_dragonglass_theme(&cc.egui_ctx);
        let downloads = user_downloads();
        let settings = load_settings().unwrap_or_default();
        let (tx, worker_rx) = mpsc::channel::<WorkerRequest>();
        let (worker_tx, rx) = mpsc::channel::<WorkerEvent>();
        thread::spawn(move || worker_loop(worker_rx, worker_tx));
        let mut app = Self {
            page: Page::Dashboard,
            downloads,
            snapshot: DashboardSnapshot::default(),
            settings,
            settings_dirty: false,
            selected_project: None,
            selected_build: None,
            pending_confirmation: None,
            tx,
            rx,
            refresh_pending: false,
            action_pending: false,
            last_refresh: Instant::now() - Duration::from_secs(REFRESH_SECONDS),
            activity: VecDeque::with_capacity(160),
            status_line: "Connecting to ForgeClean organizer…".to_owned(),
        };
        app.request_refresh();
        app
    }

    pub fn self_test() -> Result<(), String> {
        let settings = GuiSettings::default();
        settings.validate()?;
        if DRAGONGLASS_WINDOW_ALPHA != 26 {
            return Err("DragonGlass alpha contract mismatch".to_owned());
        }
        if GUI_VERSION != "1.0.1" {
            return Err("GUI version mismatch".to_owned());
        }
        Ok(())
    }

    fn request_refresh(&mut self) {
        if self.refresh_pending {
            return;
        }
        if self
            .tx
            .send(WorkerRequest::Refresh(self.downloads.clone()))
            .is_ok()
        {
            self.refresh_pending = true;
        }
    }

    fn send_action(&mut self, action: CoreAction, label: &str) {
        if self.action_pending {
            self.status_line = "Another ForgeClean action is still running.".to_owned();
            return;
        }
        match self
            .tx
            .send(WorkerRequest::Action(action, self.downloads.clone()))
        {
            Ok(()) => {
                self.action_pending = true;
                self.status_line = format!("{label}…");
                self.push_activity(format!("START  {label}"));
            }
            Err(error) => self.status_line = format!("Cannot start action: {error}"),
        }
    }

    fn drain_worker(&mut self) {
        let mut refresh_after_action = false;
        while let Ok(event) = self.rx.try_recv() {
            match event {
                WorkerEvent::Snapshot(result) => {
                    self.refresh_pending = false;
                    self.last_refresh = Instant::now();
                    match *result {
                        Ok(snapshot) => {
                            for line in snapshot.activity.iter().rev().take(8).rev() {
                                if !self.activity.iter().any(|existing| existing == line) {
                                    self.push_activity(line.clone());
                                }
                            }
                            self.snapshot = snapshot;
                            self.status_line = "ForgeClean state synchronized.".to_owned();
                        }
                        Err(error) => {
                            self.status_line = format!("State refresh blocked: {error}");
                            self.push_activity(format!("GUARD  {error}"));
                        }
                    }
                }
                WorkerEvent::Action(result) => {
                    self.action_pending = false;
                    let outcome = if result.success { "PASS" } else { "FAIL" };
                    self.status_line = format!("{}: {outcome}", result.label);
                    self.push_activity(format!("{outcome}  {}", result.label));
                    if self.settings.notifications_enabled {
                        notify_action_result(&result);
                    }
                    for line in result.output.lines().take(40) {
                        self.push_activity(format!("       {line}"));
                    }
                    refresh_after_action = true;
                }
            }
        }
        if refresh_after_action {
            self.request_refresh();
        }
    }

    fn push_activity(&mut self, line: String) {
        if self.activity.len() >= 160 {
            self.activity.pop_front();
        }
        self.activity.push_back(line);
    }

    fn title_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("forgeclean_title")
            .exact_size(42.0)
            .frame(
                glass_frame(DRAGONGLASS_WINDOW_ALPHA).inner_margin(egui::Margin::symmetric(12, 6)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let available = (ui.available_width() - 118.0).max(160.0);
                    let title = ui.add_sized(
                        [available, 28.0],
                        egui::Label::new(
                            egui::RichText::new("AETHERFORGE  /  FORGECLEAN")
                                .strong()
                                .color(text_primary()),
                        )
                        .sense(egui::Sense::click_and_drag()),
                    );
                    if title.drag_started_by(egui::PointerButton::Primary) {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                    if title.double_clicked() {
                        let maximized =
                            ui.input(|input| input.viewport().maximized.unwrap_or(false));
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                    }
                    if round_window_button(ui, "—", "Minimize") {
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                    if round_window_button(ui, "□", "Maximize") {
                        let maximized =
                            ui.input(|input| input.viewport().maximized.unwrap_or(false));
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                    }
                    if round_window_button(ui, "×", "Close") {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
    }

    fn nav(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("forgeclean_nav")
            .exact_size(NAV_WIDTH)
            .resizable(false)
            .frame(
                glass_frame(DRAGONGLASS_WINDOW_ALPHA).inner_margin(egui::Margin::symmetric(10, 12)),
            )
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new("ForgeClean")
                        .size(22.0)
                        .strong()
                        .color(accent()),
                );
                ui.label(
                    egui::RichText::new(format!("v{GUI_VERSION}"))
                        .small()
                        .color(text_muted()),
                );
                ui.add_space(10.0);
                for page in Page::ALL {
                    let selected = self.page == page;
                    let text = format!("{}  {}", page.glyph(), page.label());
                    if ui
                        .selectable_label(
                            selected,
                            egui::RichText::new(text).color(if selected {
                                text_primary()
                            } else {
                                text_muted()
                            }),
                        )
                        .clicked()
                    {
                        self.page = page;
                    }
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(if self.snapshot.service_active {
                            "● Organizer active"
                        } else {
                            "○ Organizer inactive"
                        })
                        .color(if self.snapshot.service_active {
                            ok_color()
                        } else {
                            warning_color()
                        }),
                    );
                    ui.label(
                        egui::RichText::new(&self.status_line)
                            .small()
                            .color(text_muted()),
                    );
                });
            });
    }

    fn dashboard(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "Dashboard",
            "Live ForgeClean organizer, build routing, cleanup and ColdPack state",
        );
        egui::Grid::new("dashboard_metrics")
            .num_columns(3)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                metric_card(
                    ui,
                    "Organizer",
                    if self.snapshot.service_active {
                        "ACTIVE"
                    } else {
                        "STOPPED"
                    },
                    "persistent systemd --user service",
                );
                metric_card(
                    ui,
                    "Projects",
                    &self.snapshot.project_count().to_string(),
                    "recognized canonical projects",
                );
                metric_card(
                    ui,
                    "Build bundles",
                    &self.snapshot.build_count().to_string(),
                    "project/version build folders",
                );
                ui.end_row();
                metric_card(
                    ui,
                    "Build files",
                    &self.snapshot.build_file_count().to_string(),
                    "source, scripts, verify, packages",
                );
                let reclaim = self
                    .snapshot
                    .coldpack
                    .as_ref()
                    .map_or(0, |r| r.orphan_bytes + r.expired_quarantine_bytes);
                metric_card(
                    ui,
                    "GC reclaimable",
                    &format_bytes(reclaim),
                    "orphan + expired quarantine",
                );
                let savings = self
                    .snapshot
                    .coldpack
                    .as_ref()
                    .map_or(0, |r| r.logical_bytes.saturating_sub(r.active_bytes));
                metric_card(
                    ui,
                    "ColdPack savings",
                    &format_bytes(savings),
                    "logical bytes minus stored objects",
                );
                ui.end_row();
            });
        ui.add_space(12.0);
        section(ui, "Quick actions", |ui| {
            ui.horizontal_wrapped(|ui| {
                if action_button(ui, "Organize now").clicked() {
                    self.send_action(CoreAction::OrganizeNow, "Organize now");
                }
                if action_button(ui, "Refresh").clicked() {
                    self.request_refresh();
                }
                if action_button(ui, "Cleanup preview").clicked() {
                    self.send_action(CoreAction::ScanSystem, "Cleanup preview");
                }
                if action_button(ui, "ColdPack audit").clicked() {
                    self.send_action(CoreAction::ColdpackAudit, "ColdPack audit");
                }
            });
        });
        ui.add_space(10.0);
        section(ui, "Latest activity", |ui| {
            for line in self.activity.iter().rev().take(10).rev() {
                ui.label(
                    egui::RichText::new(line)
                        .monospace()
                        .small()
                        .color(text_muted()),
                );
            }
            if self.activity.is_empty() {
                ui.label(egui::RichText::new("No activity captured yet.").color(text_muted()));
            }
        });
    }

    fn projects(&mut self, ui: &mut egui::Ui) {
        page_heading(ui, "Projects", "Project → version/build → related files");
        let projects = self.snapshot.projects.clone();
        ui.columns(2, |columns| {
            egui::ScrollArea::vertical()
                .id_salt("project_list")
                .show(&mut columns[0], |ui| {
                    for project in &projects {
                        let selected =
                            self.selected_project.as_deref() == Some(project.name.as_str());
                        let label = format!("{}  ·  {} builds", project.name, project.builds.len());
                        if ui.selectable_label(selected, label).clicked() {
                            self.selected_project = Some(project.name.clone());
                            self.selected_build =
                                project.builds.first().map(|build| build.version.clone());
                        }
                    }
                });
            egui::ScrollArea::vertical()
                .id_salt("build_detail")
                .show(&mut columns[1], |ui| {
                    let project = self
                        .selected_project
                        .as_ref()
                        .and_then(|name| projects.iter().find(|project| &project.name == name))
                        .cloned();
                    let Some(project) = project else {
                        ui.label(
                            egui::RichText::new(
                                "Select a project to inspect its Active tree and build bundles.",
                            )
                            .color(text_muted()),
                        );
                        return;
                    };
                    ui.heading(&project.name);
                    ui.horizontal_wrapped(|ui| {
                        if action_button(ui, "Open project folder").clicked() {
                            self.send_action(
                                CoreAction::OpenPath(project.path.clone()),
                                "Open project folder",
                            );
                        }
                        if action_button(ui, "Open Active").clicked() && project.active_exists {
                            self.send_action(
                                CoreAction::OpenPath(project.active_path.clone()),
                                "Open Active project",
                            );
                        }
                        if action_button(ui, "Copy Active path").clicked() {
                            ui.ctx()
                                .copy_text(project.active_path.to_string_lossy().into_owned());
                        }
                    });
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        for build in &project.builds {
                            let selected =
                                self.selected_build.as_deref() == Some(build.version.as_str());
                            if ui
                                .selectable_label(
                                    selected,
                                    format!(
                                        "{} · {}",
                                        build.version,
                                        format_bytes(build.total_bytes)
                                    ),
                                )
                                .clicked()
                            {
                                self.selected_build = Some(build.version.clone());
                            }
                        }
                    });
                    let build = self
                        .selected_build
                        .as_ref()
                        .and_then(|version| {
                            project
                                .builds
                                .iter()
                                .find(|build| &build.version == version)
                        })
                        .cloned();
                    if let Some(build) = build {
                        ui.add_space(8.0);
                        section(ui, &format!("Build {}", build.version), |ui| {
                            ui.horizontal_wrapped(|ui| {
                                if action_button(ui, "Open Folder").clicked() {
                                    self.send_action(
                                        CoreAction::OpenPath(build.path.clone()),
                                        "Open build folder",
                                    );
                                }
                                if action_button(ui, "Copy Path").clicked() {
                                    ui.ctx()
                                        .copy_text(build.path.to_string_lossy().into_owned());
                                }
                            });
                            ui.separator();
                            for file in &build.files {
                                ui.horizontal(|ui| {
                                    let name =
                                        file.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                                    ui.label(
                                        egui::RichText::new(name).monospace().color(text_primary()),
                                    );
                                    if looks_runnable(file) && ui.small_button("Run").clicked() {
                                        self.pending_confirmation =
                                            Some(PendingConfirmation::RunArtifact(file.clone()));
                                    }
                                    if ui.small_button("Copy path").clicked() {
                                        ui.ctx().copy_text(file.to_string_lossy().into_owned());
                                    }
                                });
                            }
                        });
                    }
                });
        });
    }

    fn inbox(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "Downloads / Inbox",
            "New, incomplete and unresolved files waiting for ForgeClean routing",
        );
        if self.snapshot.inbox.is_empty() {
            section(ui, "Inbox clear", |ui| {
                ui.label("No unresolved or incomplete live files are currently visible.");
            });
            return;
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for item in self.snapshot.inbox.clone() {
                section(
                    ui,
                    item.path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unidentified item"),
                    |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new(item.path.to_string_lossy())
                                    .monospace()
                                    .small()
                                    .color(text_muted()),
                            );
                            if item.incomplete {
                                ui.label(
                                    egui::RichText::new("INCOMPLETE — never promoted")
                                        .color(warning_color()),
                                );
                            }
                            if action_button(ui, "Open location").clicked()
                                && let Some(parent) = item.path.parent()
                            {
                                self.send_action(
                                    CoreAction::OpenPath(parent.to_path_buf()),
                                    "Open inbox location",
                                );
                            }
                        });
                    },
                );
                ui.add_space(6.0);
            }
        });
    }

    fn cleanup(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "Cleanup",
            "Preview first; permanent deletion remains explicit and uses the verified CLI safety gates",
        );
        section(ui, "Pacman cache cleanup", |ui| {
            ui.label("ForgeClean keeps the newest package versions, pins installed versions, and treats deletion as direct unlink with no desktop Trash.");
            ui.horizontal_wrapped(|ui| {
                if action_button(ui, "Preview candidates").clicked() {
                    self.send_action(CoreAction::ScanSystem, "Cleanup preview");
                }
                if danger_button(ui, "Clean / offload now").clicked() {
                    self.pending_confirmation = Some(PendingConfirmation::Cleanup);
                }
            });
        });
    }

    fn coldpack(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "ColdPack",
            "Shared CDC dedup store, exact restore and fail-closed 7-day quarantine GC",
        );
        if let Some(report) = &self.snapshot.coldpack {
            egui::Grid::new("coldpack_metrics")
                .num_columns(3)
                .spacing([10.0, 10.0])
                .show(ui, |ui| {
                    metric_card(
                        ui,
                        "Manifests",
                        &report.manifests.to_string(),
                        "active .fcoldpack manifests",
                    );
                    metric_card(
                        ui,
                        "Live objects",
                        &report.live_objects.to_string(),
                        &format_bytes(report.live_bytes),
                    );
                    metric_card(
                        ui,
                        "Orphan objects",
                        &report.orphan_objects.to_string(),
                        &format_bytes(report.orphan_bytes),
                    );
                    ui.end_row();
                    metric_card(
                        ui,
                        "Quarantine",
                        &report.quarantine_objects.to_string(),
                        &format_bytes(report.quarantine_bytes),
                    );
                    metric_card(
                        ui,
                        "Expired",
                        &report.expired_quarantine_objects.to_string(),
                        &format_bytes(report.expired_quarantine_bytes),
                    );
                    metric_card(
                        ui,
                        "Logical data",
                        &format_bytes(report.logical_bytes),
                        "before deduplication",
                    );
                    ui.end_row();
                });
        } else {
            ui.label(
                egui::RichText::new(
                    self.snapshot
                        .coldpack_error
                        .as_deref()
                        .unwrap_or("ColdPack state unavailable"),
                )
                .color(warning_color()),
            );
        }
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            if action_button(ui, "Audit").clicked() {
                self.send_action(CoreAction::ColdpackAudit, "ColdPack audit");
            }
            if action_button(ui, "GC preview").clicked() {
                self.send_action(CoreAction::ColdpackGcPreview, "ColdPack GC preview");
            }
            if danger_button(ui, "Apply GC").clicked() {
                self.pending_confirmation = Some(PendingConfirmation::ColdPackGc);
            }
        });
    }

    fn storage(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "Storage",
            "Internal cleanup mode and conditional verified external-drive offload",
        );
        egui::Grid::new("storage_metrics")
            .num_columns(2)
            .spacing([10.0, 10.0])
            .show(ui, |ui| {
                metric_card(
                    ui,
                    "Mode",
                    &self.snapshot.storage_mode,
                    "external detection is fail-closed",
                );
                metric_card(
                    ui,
                    "External drives",
                    &self.snapshot.external_drives.to_string(),
                    "eligible mounted devices",
                );
                ui.end_row();
            });
        ui.add_space(10.0);
        if action_button(ui, "Refresh storage status").clicked() {
            self.send_action(CoreAction::StorageStatus, "Storage status");
        }
    }

    fn activity(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "Activity",
            "Chronological organizer and GUI action feed",
        );
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in &self.activity {
                    ui.label(
                        egui::RichText::new(line)
                            .monospace()
                            .small()
                            .color(text_muted()),
                    );
                }
            });
    }

    fn settings(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "Settings",
            "Persistent organizer/runtime and GUI controls",
        );
        section(ui, "Organizer", |ui| {
            let mut enabled = self.settings.organizer_enabled;
            if ui
                .checkbox(&mut enabled, "Persistent organizer enabled")
                .changed()
            {
                self.settings.organizer_enabled = enabled;
                self.settings_dirty = true;
            }
            ui.horizontal(|ui| {
                ui.label("File stability delay (seconds)");
                if ui
                    .add(egui::DragValue::new(&mut self.settings.stable_seconds).range(0..=3600))
                    .changed()
                {
                    self.settings_dirty = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Polling interval (seconds)");
                if ui
                    .add(egui::DragValue::new(&mut self.settings.poll_seconds).range(1..=300))
                    .changed()
                {
                    self.settings_dirty = true;
                }
            });
            if ui
                .checkbox(
                    &mut self.settings.compatibility_symlinks,
                    "Compatibility symlinks / legacy path visibility",
                )
                .changed()
            {
                self.settings_dirty = true;
            }
        });
        ui.add_space(8.0);
        section(ui, "Routing policy", |ui| {
            diagnostic_line(ui, "Project identity", "AUTHORITATIVE / ALWAYS ON");
            diagnostic_line(ui, "Build-aware grouping", "AUTHORITATIVE / ALWAYS ON");
            diagnostic_line(ui, "Existing-file reconciliation", "CONTINUOUS");
            diagnostic_line(ui, "Generic categories", "FALLBACK ONLY");
            diagnostic_line(ui, "Incomplete downloads", "NEVER PROMOTED");
        });
        ui.add_space(8.0);
        section(ui, "External storage policy", |ui| {
            diagnostic_line(ui, "No external drive", "AUTO_CLEAN");
            diagnostic_line(ui, "Eligible external drive", "AUTO_OFFLOAD");
            diagnostic_line(ui, "Detection failure", "FAIL CLOSED");
        });
        ui.add_space(8.0);
        section(ui, "ColdPack maintenance", |ui| {
            if ui
                .checkbox(
                    &mut self.settings.coldpack_maintenance_enabled,
                    "Automatic ColdPack maintenance",
                )
                .changed()
            {
                self.settings_dirty = true;
            }
            ui.horizontal(|ui| {
                ui.label("Maintenance interval (hours)");
                if ui
                    .add(
                        egui::DragValue::new(&mut self.settings.coldpack_maintenance_hours)
                            .range(1..=720),
                    )
                    .changed()
                {
                    self.settings_dirty = true;
                }
            });
        });
        ui.add_space(8.0);
        section(ui, "Interface", |ui| {
            if ui
                .checkbox(&mut self.settings.notifications_enabled, "Notifications")
                .changed()
            {
                self.settings_dirty = true;
            }
            if ui
                .checkbox(
                    &mut self.settings.diagnostics_visible,
                    "Show diagnostics details",
                )
                .changed()
            {
                self.settings_dirty = true;
            }
            egui::ComboBox::from_label("Logging level")
                .selected_text(&self.settings.logging_level)
                .show_ui(ui, |ui| {
                    for level in ["quiet", "normal", "verbose"] {
                        if ui
                            .selectable_value(
                                &mut self.settings.logging_level,
                                level.to_owned(),
                                level,
                            )
                            .changed()
                        {
                            self.settings_dirty = true;
                        }
                    }
                });
        });
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            let save = ui.add_enabled(self.settings_dirty, egui::Button::new("Save settings"));
            if save.clicked() {
                match save_settings(&self.settings) {
                    Ok(path) => {
                        self.settings_dirty = false;
                        self.status_line = format!("Settings saved: {}", path.display());
                        self.push_activity("PASS  Settings saved".to_owned());
                        if self.settings.organizer_enabled != self.snapshot.service_enabled {
                            self.send_action(
                                CoreAction::SetServiceEnabled(self.settings.organizer_enabled),
                                "Apply organizer startup/service setting",
                            );
                        } else if self.settings.organizer_enabled {
                            self.send_action(
                                CoreAction::RestartService,
                                "Restart organizer with new settings",
                            );
                        }
                    }
                    Err(error) => self.status_line = format!("Settings blocked: {error}"),
                }
            }
            if self.snapshot.service_active {
                if danger_button(ui, "Stop organizer").clicked() {
                    self.send_action(
                        CoreAction::SetServiceEnabled(false),
                        "Stop organizer service",
                    );
                }
            } else if action_button(ui, "Start organizer").clicked() {
                self.send_action(
                    CoreAction::SetServiceEnabled(true),
                    "Start organizer service",
                );
            }
        });
    }

    fn diagnostics(&mut self, ui: &mut egui::Ui) {
        page_heading(
            ui,
            "About / Diagnostics",
            "ForgeClean GUI and authoritative engine state",
        );
        section(ui, "Identity", |ui| {
            diagnostic_line(ui, "ForgeClean GUI", GUI_VERSION);
            diagnostic_line(
                ui,
                "Organizer service",
                if self.snapshot.service_active {
                    "active"
                } else {
                    "inactive"
                },
            );
            diagnostic_line(
                ui,
                "Organizer startup",
                if self.snapshot.service_enabled {
                    "enabled"
                } else {
                    "disabled"
                },
            );
        });
        ui.add_space(8.0);
        section(ui, "Paths", |ui| {
            diagnostic_line(ui, "Downloads", &self.downloads.to_string_lossy());
            diagnostic_line(
                ui,
                "Organizer root",
                &self.snapshot.organizer_root.to_string_lossy(),
            );
            diagnostic_line(
                ui,
                "Registry",
                &self.snapshot.registry_path.to_string_lossy(),
            );
            diagnostic_line(
                ui,
                "ColdPack store",
                &self.snapshot.coldpack_store.to_string_lossy(),
            );
        });
        if self.settings.diagnostics_visible {
            ui.add_space(8.0);
            section(ui, "Current status", |ui| {
                ui.label(
                    egui::RichText::new(&self.status_line)
                        .monospace()
                        .color(text_muted()),
                );
            });
        }
    }

    fn confirmation_window(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.pending_confirmation.clone() else {
            return;
        };
        let (title, body, confirm_label) = match &pending {
            PendingConfirmation::Cleanup => (
                "Confirm permanent cleanup / offload",
                "ForgeClean will invoke its existing `auto --yes` safety path through system authorization. With no external drive, approved files are permanently unlinked with no Trash. With an external drive, eligible files are verified then offloaded.",
                "Authorize cleanup",
            ),
            PendingConfirmation::ColdPackGc => (
                "Confirm ColdPack GC apply",
                "ForgeClean will audit the authoritative manifests, quarantine orphan objects, and only purge objects that have remained unreferenced beyond the 7-day quarantine. Any metadata corruption blocks the operation.",
                "Apply GC",
            ),
            PendingConfirmation::RunArtifact(path) => (
                "Run build / install artifact",
                path.to_str().unwrap_or(
                    "Selected artifact will be executed from its canonical build folder.",
                ),
                "Run artifact",
            ),
        };
        let mut keep_open = true;
        egui::Window::new(title)
            .open(&mut keep_open)
            .collapsible(false)
            .resizable(false)
            .frame(glass_frame(DRAGONGLASS_WINDOW_ALPHA))
            .show(ctx, |ui| {
                ui.set_max_width(520.0);
                ui.label(body);
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if danger_button(ui, confirm_label).clicked() {
                        match pending.clone() {
                            PendingConfirmation::Cleanup => self.send_action(
                                CoreAction::AutoCleanupWithPrivilege,
                                "Automatic cleanup/offload",
                            ),
                            PendingConfirmation::ColdPackGc => {
                                self.send_action(CoreAction::ColdpackGcApply, "ColdPack GC apply")
                            }
                            PendingConfirmation::RunArtifact(path) => self.send_action(
                                CoreAction::RunArtifact(path),
                                "Run build/install artifact",
                            ),
                        }
                        self.pending_confirmation = None;
                    }
                    if action_button(ui, "Cancel").clicked() {
                        self.pending_confirmation = None;
                    }
                });
            });
        if !keep_open {
            self.pending_confirmation = None;
        }
    }
}

impl eframe::App for ForgeCleanGui {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_worker();
        if self.last_refresh.elapsed() >= Duration::from_secs(REFRESH_SECONDS) {
            self.request_refresh();
        }
        ctx.request_repaint_after(Duration::from_millis(250));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.title_bar(ui);
        self.nav(ui);
        egui::CentralPanel::default()
            .frame(
                glass_frame(DRAGONGLASS_WINDOW_ALPHA).inner_margin(egui::Margin::symmetric(18, 16)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.page {
                        Page::Dashboard => self.dashboard(ui),
                        Page::Projects => self.projects(ui),
                        Page::Inbox => self.inbox(ui),
                        Page::Cleanup => self.cleanup(ui),
                        Page::ColdPack => self.coldpack(ui),
                        Page::Storage => self.storage(ui),
                        Page::Activity => self.activity(ui),
                        Page::Settings => self.settings(ui),
                        Page::Diagnostics => self.diagnostics(ui),
                    });
            });
        self.confirmation_window(ui.ctx());
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}

fn notify_action_result(result: &ActionResult) {
    let state = if result.success { "PASS" } else { "FAIL" };
    let _ = Command::new("notify-send")
        .args([
            "--app-name=ForgeClean",
            &format!("ForgeClean · {state}"),
            &result.label,
        ])
        .spawn();
}

fn worker_loop(rx: Receiver<WorkerRequest>, tx: Sender<WorkerEvent>) {
    while let Ok(request) = rx.recv() {
        match request {
            WorkerRequest::Refresh(downloads) => {
                let _ = tx.send(WorkerEvent::Snapshot(Box::new(collect_dashboard_snapshot(
                    &downloads,
                ))));
            }
            WorkerRequest::Action(action, downloads) => {
                let result = run_core_action(action, &downloads);
                let _ = tx.send(WorkerEvent::Action(result));
            }
        }
    }
}

fn apply_dragonglass_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(text_primary());
    visuals.panel_fill = glass_color(8, 8, 18, DRAGONGLASS_WINDOW_ALPHA);
    visuals.window_fill = glass_color(11, 9, 24, DRAGONGLASS_WINDOW_ALPHA);
    visuals.faint_bg_color = glass_color(31, 20, 62, DRAGONGLASS_WINDOW_ALPHA);
    visuals.extreme_bg_color = glass_color(5, 5, 12, DRAGONGLASS_WINDOW_ALPHA);
    visuals.selection.bg_fill = glass_color(86, 56, 210, DRAGONGLASS_WINDOW_ALPHA);
    visuals.selection.stroke = egui::Stroke::new(1.0, accent());
    visuals.widgets.noninteractive.bg_fill = glass_color(15, 12, 30, DRAGONGLASS_WINDOW_ALPHA);
    visuals.widgets.inactive.bg_fill = glass_color(38, 25, 76, DRAGONGLASS_WINDOW_ALPHA);
    visuals.widgets.hovered.bg_fill = glass_color(65, 43, 130, DRAGONGLASS_WINDOW_ALPHA);
    visuals.widgets.active.bg_fill = glass_color(82, 55, 165, DRAGONGLASS_WINDOW_ALPHA);
    visuals.widgets.open.bg_fill = glass_color(65, 43, 130, DRAGONGLASS_WINDOW_ALPHA);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, text_muted());
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text_primary());
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
    visuals.window_stroke = egui::Stroke::new(1.0, glass_color(122, 103, 255, 70));
    ctx.set_visuals(visuals);
    ctx.global_style_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
    });
}

fn glass_frame(alpha: u8) -> egui::Frame {
    egui::Frame::new()
        .fill(glass_color(10, 8, 22, alpha))
        .stroke(egui::Stroke::new(1.0, glass_color(123, 105, 255, 55)))
        .corner_radius(14)
}

fn section<R>(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui) -> R) -> R {
    glass_frame(DRAGONGLASS_WINDOW_ALPHA)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(title).strong().color(accent()));
            ui.add_space(4.0);
            body(ui)
        })
        .inner
}

fn page_heading(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(
        egui::RichText::new(title)
            .size(26.0)
            .strong()
            .color(text_primary()),
    );
    ui.label(egui::RichText::new(subtitle).color(text_muted()));
    ui.add_space(12.0);
}

fn metric_card(ui: &mut egui::Ui, label: &str, value: &str, detail: &str) {
    glass_frame(DRAGONGLASS_WINDOW_ALPHA)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.set_min_width(170.0);
            ui.label(egui::RichText::new(label).small().color(text_muted()));
            ui.label(
                egui::RichText::new(value)
                    .size(20.0)
                    .strong()
                    .color(accent()),
            );
            ui.label(egui::RichText::new(detail).small().color(text_muted()));
        });
}

fn diagnostic_line(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new(format!("{label}:"))
                .strong()
                .color(text_muted()),
        );
        ui.label(egui::RichText::new(value).monospace().color(text_primary()));
    });
}

fn action_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(egui::Button::new(egui::RichText::new(label).color(text_primary())).corner_radius(18))
}

fn danger_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(label).color(egui::Color32::WHITE))
            .fill(glass_color(128, 35, 88, DRAGONGLASS_WINDOW_ALPHA))
            .corner_radius(18),
    )
}

fn round_window_button(ui: &mut egui::Ui, glyph: &str, tooltip: &str) -> bool {
    ui.add_sized([28.0, 28.0], egui::Button::new(glyph).corner_radius(14))
        .on_hover_text(tooltip)
        .clicked()
}

fn looks_runnable(path: &Path) -> bool {
    let lower = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    lower.ends_with(".sh")
        || lower.ends_with(".bash")
        || lower.ends_with(".fish")
        || lower.ends_with(".run")
        || lower.ends_with(".appimage")
        || lower.contains("hit-it")
        || lower.contains("install")
        || lower.contains("build")
}

fn user_downloads() -> PathBuf {
    env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("Downloads")
}

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let value = bytes as f64;
    if value >= GIB {
        format!("{:.2} GiB", value / GIB)
    } else if value >= MIB {
        format!("{:.1} MiB", value / MIB)
    } else if value >= KIB {
        format!("{:.1} KiB", value / KIB)
    } else {
        format!("{bytes} B")
    }
}

fn glass_color(r: u8, g: u8, b: u8, a: u8) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
}

fn accent() -> egui::Color32 {
    egui::Color32::from_rgb(151, 126, 255)
}

fn text_primary() -> egui::Color32 {
    egui::Color32::from_rgb(238, 234, 255)
}

fn text_muted() -> egui::Color32 {
    egui::Color32::from_rgb(180, 173, 205)
}

fn ok_color() -> egui::Color32 {
    egui::Color32::from_rgb(145, 234, 198)
}

fn warning_color() -> egui::Color32 {
    egui::Color32::from_rgb(255, 188, 126)
}
