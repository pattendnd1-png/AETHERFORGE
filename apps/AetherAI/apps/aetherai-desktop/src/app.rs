use crate::terminal::TerminalWorkspace;
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::env;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::thread;

use crate::health::{
    HealthAction, HealthPanelState, HealthState, diagnostic_path_in, export_diagnostic,
    mark_pending_release_healthy, repair_safe_directories,
};
use crate::update::UpdatePanelState;
use aether_chat::{ChatEvent, ChatService};
use aether_core::{Conversation, NetworkPolicy, PermissionDecision, ProcessingPreset, Role};
use aether_import::{ChatGptImportSummary, import_chatgpt_export_all};
use aether_model_api::{ModelBackend, ModelDescriptor, ModelFormat, ModelLoadState, ModelProvider};
use aether_model_gguf::GgufProvider;
use aether_model_native::NativeProvider;
use aether_model_runtime::{
    catalog::{ModelCatalog, ModelCatalogEntry},
    download::download_model,
    runtime::{RuntimeManifest, install_gguf_runtime, stable_runtime_path},
};
use aether_storage::{
    SqliteStore,
    models::{ActivityRecord, ModelRecord, ProjectRecord, UiState},
};
use aether_system::{ResourceSnapshot, current_system_integration};
use aether_tools::{ActivityEvent, ActivitySink, ProcessRequest, ProcessRunner, ToolError};
use aether_ui::{DesktopPage, DesktopUiState, DragonGlassTheme, Rgba};
use aether_workspace::{DiscoveryConfig, ProjectCandidate, scan_root};
use chrono::Utc;
use std::io::Read;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum DrawCommand {
    Rect {
        rect: Rect,
        color: Rgba,
        radius: i32,
    },
    Text {
        x: i32,
        y: i32,
        text: String,
        color: Rgba,
    },
}
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}
impl Rect {
    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SettingsAction {
    ConnectChatGpt,
    OpenPrivacyPortal,
    ImportChatGptExport,
}

fn health_action_at(x: i32, y: i32, content_x: i32, content_width: i32) -> Option<HealthAction> {
    let available = (content_width - 48).max(300);
    let button_width = ((available - 32) / 5).max(56);
    for (index, action) in [
        HealthAction::Recheck,
        HealthAction::Repair,
        HealthAction::RollBack,
        HealthAction::OpenLogs,
        HealthAction::ExportDiagnostic,
    ]
    .into_iter()
    .enumerate()
    {
        let rect = Rect {
            x: content_x + 24 + index as i32 * (button_width + 8),
            y: 110,
            w: button_width,
            h: 34,
        };
        if rect.contains(x, y) {
            return Some(action);
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateSettingsAction {
    CheckUpdates,
    PrepareUpdate,
    RestartAndUpdate,
    RollBack,
}
fn update_settings_action_at(x: i32, y: i32, cx: i32, cw: i32) -> Option<UpdateSettingsAction> {
    let w = (cw - 48).clamp(220, 420);
    for (yy, a) in [
        (430, UpdateSettingsAction::CheckUpdates),
        (472, UpdateSettingsAction::PrepareUpdate),
        (514, UpdateSettingsAction::RestartAndUpdate),
        (556, UpdateSettingsAction::RollBack),
    ] {
        if (Rect {
            x: cx + 24,
            y: yy,
            w,
            h: 36,
        })
        .contains(x, y)
        {
            return Some(a);
        }
    }
    None
}

fn settings_action_at(
    x: i32,
    y: i32,
    content_x: i32,
    content_width: i32,
) -> Option<SettingsAction> {
    let width = (content_width - 48).clamp(220, 420);
    for (rect, action) in [
        (
            Rect {
                x: content_x + 24,
                y: 138,
                w: width,
                h: 38,
            },
            SettingsAction::ConnectChatGpt,
        ),
        (
            Rect {
                x: content_x + 24,
                y: 194,
                w: width,
                h: 38,
            },
            SettingsAction::OpenPrivacyPortal,
        ),
        (
            Rect {
                x: content_x + 24,
                y: 280,
                w: width,
                h: 42,
            },
            SettingsAction::ImportChatGptExport,
        ),
    ] {
        if rect.contains(x, y) {
            return Some(action);
        }
    }
    None
}

#[derive(Debug, Clone)]
enum BackgroundEvent {
    Delta(String),
    ChatFinished(Conversation, Option<String>),
    Activity(ActivityEvent),
    ToolFinished {
        command: String,
        stdout: String,
        stderr: String,
        exit: i32,
    },
    LiveAssetsFinished(ModelRecord),
    DownloadFailed(String),
    ChatGptImportFinished(Result<ChatGptImportSummary, String>),
}
struct ChannelSink {
    tx: mpsc::Sender<BackgroundEvent>,
}
impl ActivitySink for ChannelSink {
    fn record(&self, e: ActivityEvent) -> Result<(), ToolError> {
        self.tx
            .send(BackgroundEvent::Activity(e))
            .map_err(|_| ToolError::Activity("desktop closed".into()))
    }
}

const MAX_COMPOSER_ATTACHMENTS: usize = 12;
const MAX_ATTACHMENT_PREVIEW_BYTES: u64 = 64 * 1024;
const MAX_FOLDER_PREVIEW_FILES: usize = 32;
const MAX_FOLDER_PREVIEW_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentKind {
    File,
    Folder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposerAttachment {
    pub path: PathBuf,
    pub display_name: String,
    pub kind: AttachmentKind,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSurfaceEntry {
    pub path: PathBuf,
    pub display_name: String,
    pub is_dir: bool,
    pub size_bytes: u64,
}

pub struct AetherApp {
    pub store: SqliteStore,
    pub conversation: Conversation,
    pub ui: DesktopUiState,
    pub theme: DragonGlassTheme,
    pub composer: String,
    pub composer_cursor: usize,
    pub composer_attachments: Vec<ComposerAttachment>,
    pub files_root: PathBuf,
    pub files_entries: Vec<FileSurfaceEntry>,
    pub file_pick_mode: Option<AttachmentKind>,
    pub streaming: String,
    pub status: String,
    pub terminal: TerminalWorkspace,
    pub models: Vec<ModelRecord>,
    providers: HashMap<String, Arc<dyn ModelProvider>>,
    pub projects: Vec<ProjectCandidate>,
    pub activities: Vec<ActivityEvent>,
    pub resources: Option<ResourceSnapshot>,
    pub update: UpdatePanelState,
    pub health: HealthPanelState,
    tx: mpsc::Sender<BackgroundEvent>,
    rx: mpsc::Receiver<BackgroundEvent>,
    pub busy: bool,
    pub first_run_step: u8,
    pub should_quit: bool,
    pub startup_health_pending: bool,
    pub catalog: Vec<ModelCatalogEntry>,
    data_dir: PathBuf,
    model_dir: PathBuf,
    runtime_dir: PathBuf,
}
impl AetherApp {
    pub fn new(data_dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(data_dir)?;
        let store = SqliteStore::open(data_dir.join("aetherai.db"))?;
        let stored = store.load_ui_state()?;
        let ui = DesktopUiState {
            navigation: aether_ui::PanelState {
                open: stored.left_open,
                width: stored.left_width,
                min_width: 190.0,
                max_width: 440.0,
            },
            processing: aether_ui::PanelState {
                open: stored.processing_open,
                width: stored.processing_width,
                min_width: 260.0,
                max_width: 520.0,
            },
            page: DesktopPage::Chat,
            last_conversation: stored.last_conversation,
            ui_scale: stored.ui_scale,
            first_run_complete: stored.first_run_complete,
        };
        let conversations = store.list_conversations()?;
        let conversation = stored
            .last_conversation
            .and_then(|id| conversations.iter().find(|c| c.id == id).cloned())
            .or_else(|| conversations.first().cloned())
            .unwrap_or_else(|| Conversation::new("New Chat"));
        store.save_conversation(&conversation)?;
        let models = store.list_models()?;
        let runtime_dir = data_dir.join("runtime");
        let providers = build_providers(&models, &conversation, &runtime_dir);
        let projects = discover_common_projects();
        let resources = current_system_integration().resource_snapshot().ok();
        let update = UpdatePanelState::for_user()?;
        let catalog =
            ModelCatalog::parse(include_str!("../../../resources/model-catalog.json"))?.models;
        let model_dir = data_dir.join("models");
        let (tx, rx) = mpsc::channel();
        let files_root = default_files_root(data_dir);
        let files_entries = scan_files_surface(&files_root);
        let health = HealthPanelState::evaluate(
            data_dir,
            &store,
            models.len(),
            providers.len(),
            &update.summary(),
        );
        Ok(Self {
            store,
            conversation,
            ui,
            theme: DragonGlassTheme::default(),
            composer: String::new(),
            composer_cursor: 0,
            composer_attachments: Vec::new(),
            files_root,
            files_entries,
            file_pick_mode: None,
            streaming: String::new(),
            status: if models.is_empty() {
                "No live model configured — open Models to download or import one.".into()
            } else {
                "Ready".into()
            },
            terminal: TerminalWorkspace::new(),
            models,
            providers,
            projects,
            activities: vec![],
            resources,
            update,
            health,
            tx,
            rx,
            busy: false,
            first_run_step: 0,
            should_quit: false,
            startup_health_pending: false,
            catalog,
            data_dir: data_dir.to_path_buf(),
            model_dir,
            runtime_dir,
        })
    }
    pub fn diagnostic(&self) {
        println!("AETHERAI_DESKTOP=LIVE");
        println!(
            "AETHERAI_DESKTOP_PLATFORM={}",
            current_system_integration().platform_name()
        );
        println!(
            "AETHERAI_LEFT_PANEL={}",
            if self.ui.navigation.open {
                "OPEN"
            } else {
                "CLOSED"
            }
        );
        println!(
            "AETHERAI_PROCESSING_PANEL={}",
            if self.ui.processing.open {
                "OPEN"
            } else {
                "CLOSED"
            }
        );
        println!("AETHERAI_THEME=DRAGONGLASS");
        println!("AETHERAI_RUNTIME=LIVE");
        println!("AETHERAI_TERMINAL_AUTHORITY=AETHERFORGE_TERMINAL");
        println!("AETHERAI_TERMINAL_DONOR_VERSION=10.2.21");
        println!("AETHERAI_TERMINAL_PROTOCOL=unix-socket-bincode-v1");
        println!(
            "AETHERAI_TERMINAL_CAPABILITY_COUNT={}",
            TerminalWorkspace::capability_count()
        );
        println!("AETHERAI_LIVE_MODELS={}", self.models.len());
    }
    fn persist_ui(&self) {
        let _ = self.store.save_ui_state(&UiState {
            left_open: self.ui.navigation.open,
            left_width: self.ui.navigation.width,
            processing_open: self.ui.processing.open,
            processing_width: self.ui.processing.width,
            ui_scale: self.ui.ui_scale,
            first_run_complete: self.ui.first_run_complete,
            last_conversation: Some(self.conversation.id),
        });
    }
    pub fn tick(&mut self) -> bool {
        let mut changed = false;
        while let Ok(event) = self.rx.try_recv() {
            changed = true;
            match event {
                BackgroundEvent::Delta(delta) => self.streaming.push_str(&delta),
                BackgroundEvent::ChatFinished(conversation, error) => {
                    self.conversation = conversation;
                    self.busy = false;
                    self.streaming.clear();
                    self.status = error.unwrap_or_else(|| "Ready".into());
                    let _ = self.store.save_conversation(&self.conversation);
                }
                BackgroundEvent::Activity(activity) => {
                    if let Ok(payload) = serde_json::to_string(&activity) {
                        let _ = self.store.append_activity(&ActivityRecord {
                            id: activity.id,
                            conversation_id: Some(self.conversation.id),
                            project_id: self
                                .conversation
                                .workspace
                                .as_ref()
                                .and_then(|w| w.project_id),
                            started_at: activity.started_at,
                            payload_json: payload,
                        });
                    }
                    self.activities.push(activity);
                }
                BackgroundEvent::LiveAssetsFinished(record) => {
                    let id = record.descriptor.id.clone();
                    let _ = self.store.save_model(&record);
                    if !self.models.iter().any(|m| m.descriptor.id == id) {
                        self.models.push(record);
                    }
                    self.providers =
                        build_providers(&self.models, &self.conversation, &self.runtime_dir);
                    self.conversation.settings.model = Some(id.clone());
                    let _ = self.store.save_conversation(&self.conversation);
                    self.busy = false;
                    self.status = format!("Model installed and selected: {id}");
                }
                BackgroundEvent::DownloadFailed(error) => {
                    self.busy = false;
                    self.status = format!("Model download failed: {error}");
                }
                BackgroundEvent::ChatGptImportFinished(result) => {
                    self.busy = false;
                    match result {
                        Ok(summary) => {
                            self.status = format!(
                                "ChatGPT import complete: {} project(s), {} chat(s), {} message(s), {} attachment(s), {} checkpoint(s)",
                                summary.project_count,
                                summary.conversation_count,
                                summary.message_count,
                                summary.attachment_count,
                                summary.checkpoint_count,
                            );
                        }
                        Err(error) => {
                            self.status = format!("ChatGPT import failed: {error}");
                        }
                    }
                }
                BackgroundEvent::ToolFinished {
                    command,
                    stdout,
                    stderr,
                    exit,
                } => {
                    let mut body = format!("$ {command}\n");
                    body.push_str(&stdout);
                    if !stderr.is_empty() {
                        body.push_str("\n[stderr]\n");
                        body.push_str(&stderr);
                    }
                    body.push_str(&format!("\nexit={exit}"));
                    self.conversation.push_message(Role::Tool, body);
                    let _ = self.store.save_conversation(&self.conversation);
                    self.busy = false;
                    self.status = if exit == 0 {
                        "Work command passed".into()
                    } else {
                        format!("Work command failed: exit {exit}")
                    };
                }
            }
        }
        changed
    }
    pub fn toggle_navigation(&mut self) {
        self.ui.navigation.toggle();
        self.persist_ui();
    }
    pub fn toggle_processing(&mut self) {
        self.ui.processing.toggle();
        self.persist_ui();
    }
    pub fn attach_path(&mut self, path: PathBuf) {
        if self.composer_attachments.len() >= MAX_COMPOSER_ATTACHMENTS {
            self.status =
                format!("Attachment limit reached: {MAX_COMPOSER_ATTACHMENTS}. Remove one first.");
            return;
        }
        let canonical = path.canonicalize().unwrap_or(path);
        if self
            .composer_attachments
            .iter()
            .any(|item| item.path == canonical)
        {
            self.status = format!("Already attached: {}", canonical.display());
            return;
        }
        let metadata = match std::fs::metadata(&canonical) {
            Ok(value) => value,
            Err(error) => {
                self.status = format!("Cannot attach {}: {error}", canonical.display());
                return;
            }
        };
        let kind = if metadata.is_dir() {
            AttachmentKind::Folder
        } else if metadata.is_file() {
            AttachmentKind::File
        } else {
            self.status = format!("Unsupported attachment: {}", canonical.display());
            return;
        };
        let display_name = canonical
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| canonical.display().to_string());
        self.composer_attachments.push(ComposerAttachment {
            path: canonical,
            display_name,
            kind,
            size_bytes: if kind == AttachmentKind::File {
                metadata.len()
            } else {
                0
            },
        });
        self.status = format!(
            "{} attachment(s) ready — local context only.",
            self.composer_attachments.len()
        );
    }

    pub fn native_attach_dropped_paths(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            self.attach_path(path);
        }
        if !self.composer_attachments.is_empty() {
            self.ui.page = DesktopPage::Chat;
        }
    }

    fn remove_attachment(&mut self, index: usize) {
        if index < self.composer_attachments.len() {
            let removed = self.composer_attachments.remove(index);
            self.status = format!("Removed attachment: {}", removed.display_name);
        }
    }

    fn begin_file_picker(&mut self, kind: AttachmentKind) {
        self.file_pick_mode = Some(kind);
        self.ui.page = DesktopPage::Files;
        self.refresh_files_surface();
        self.status = match kind {
            AttachmentKind::File => "Choose a file — folders open for browsing.".into(),
            AttachmentKind::Folder => "Browse to a folder, then choose ATTACH THIS FOLDER.".into(),
        };
    }

    fn cancel_file_picker(&mut self) {
        self.file_pick_mode = None;
        self.ui.page = DesktopPage::Chat;
        self.status = "Attachment picker closed.".into();
    }

    fn refresh_files_surface(&mut self) {
        self.files_entries = scan_files_surface(&self.files_root);
    }

    fn navigate_files_to(&mut self, path: PathBuf) {
        if path.is_dir() {
            self.files_root = path.canonicalize().unwrap_or(path);
            self.refresh_files_surface();
        }
    }

    fn files_parent(&mut self) {
        if let Some(parent) = self.files_root.parent().map(Path::to_path_buf) {
            self.navigate_files_to(parent);
        }
    }

    fn attach_current_folder(&mut self) {
        self.attach_path(self.files_root.clone());
        self.file_pick_mode = None;
        self.ui.page = DesktopPage::Chat;
    }

    fn clamp_composer_cursor(&mut self) {
        self.composer_cursor = self.composer_cursor.min(self.composer.len());
        while self.composer_cursor > 0 && !self.composer.is_char_boundary(self.composer_cursor) {
            self.composer_cursor -= 1;
        }
    }

    fn previous_char_boundary(&self, from: usize) -> usize {
        self.composer[..from]
            .char_indices()
            .next_back()
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    fn next_char_boundary(&self, from: usize) -> usize {
        if from >= self.composer.len() {
            return self.composer.len();
        }
        from + self.composer[from..]
            .chars()
            .next()
            .map(char::len_utf8)
            .unwrap_or(0)
    }

    fn previous_word_boundary(&self, from: usize) -> usize {
        let mut cursor = from;
        while cursor > 0 {
            let previous = self.previous_char_boundary(cursor);
            let ch = self.composer[previous..cursor]
                .chars()
                .next()
                .unwrap_or(' ');
            if !ch.is_whitespace() {
                break;
            }
            cursor = previous;
        }
        while cursor > 0 {
            let previous = self.previous_char_boundary(cursor);
            let ch = self.composer[previous..cursor]
                .chars()
                .next()
                .unwrap_or(' ');
            if ch.is_whitespace() {
                break;
            }
            cursor = previous;
        }
        cursor
    }

    fn next_word_boundary(&self, from: usize) -> usize {
        let mut cursor = from;
        while cursor < self.composer.len() {
            let next = self.next_char_boundary(cursor);
            let ch = self.composer[cursor..next].chars().next().unwrap_or(' ');
            if !ch.is_whitespace() {
                break;
            }
            cursor = next;
        }
        while cursor < self.composer.len() {
            let next = self.next_char_boundary(cursor);
            let ch = self.composer[cursor..next].chars().next().unwrap_or(' ');
            if ch.is_whitespace() {
                break;
            }
            cursor = next;
        }
        cursor
    }

    pub fn handle_text(&mut self, text: &str) {
        if self.ui.page == DesktopPage::Terminal {
            if let Err(error) = self.terminal.write_text(text) {
                self.status = format!("Terminal input failed: {error}");
            }
            return;
        }

        if self.ui.page == DesktopPage::Chat && !self.busy {
            self.clamp_composer_cursor();
            self.composer.insert_str(self.composer_cursor, text);
            self.composer_cursor += text.len();
        }
    }

    pub fn backspace(&mut self) {
        if self.ui.page == DesktopPage::Terminal {
            if let Err(error) = self.terminal.write_bytes(&[0x7f]) {
                self.status = format!("Terminal backspace failed: {error}");
            }
            return;
        }

        if self.ui.page == DesktopPage::Chat && !self.busy {
            self.clamp_composer_cursor();
            if self.composer_cursor > 0 {
                let previous = self.previous_char_boundary(self.composer_cursor);
                self.composer.drain(previous..self.composer_cursor);
                self.composer_cursor = previous;
            }
        }
    }

    pub fn delete_forward(&mut self) {
        if self.ui.page == DesktopPage::Terminal {
            if let Err(error) = self.terminal.write_bytes(b"\x1b[3~") {
                self.status = format!("Terminal delete failed: {error}");
            }
            return;
        }

        if self.ui.page == DesktopPage::Chat && !self.busy {
            self.clamp_composer_cursor();
            if self.composer_cursor < self.composer.len() {
                let next = self.next_char_boundary(self.composer_cursor);
                self.composer.drain(self.composer_cursor..next);
            }
        }
    }

    pub fn cursor_left(&mut self, ctrl: bool) {
        if self.ui.page == DesktopPage::Terminal {
            let bytes: &[u8] = if ctrl { b"\x1b[1;5D" } else { b"\x1b[D" };
            if let Err(error) = self.terminal.write_bytes(bytes) {
                self.status = format!("Terminal cursor-left failed: {error}");
            }
            return;
        }

        if self.ui.page == DesktopPage::Chat && !self.busy {
            self.clamp_composer_cursor();
            self.composer_cursor = if ctrl {
                self.previous_word_boundary(self.composer_cursor)
            } else {
                self.previous_char_boundary(self.composer_cursor)
            };
        }
    }

    pub fn cursor_right(&mut self, ctrl: bool) {
        if self.ui.page == DesktopPage::Terminal {
            let bytes: &[u8] = if ctrl { b"\x1b[1;5C" } else { b"\x1b[C" };
            if let Err(error) = self.terminal.write_bytes(bytes) {
                self.status = format!("Terminal cursor-right failed: {error}");
            }
            return;
        }

        if self.ui.page == DesktopPage::Chat && !self.busy {
            self.clamp_composer_cursor();
            self.composer_cursor = if ctrl {
                self.next_word_boundary(self.composer_cursor)
            } else {
                self.next_char_boundary(self.composer_cursor)
            };
        }
    }

    pub fn cursor_home(&mut self) {
        if self.ui.page == DesktopPage::Terminal {
            if let Err(error) = self.terminal.write_bytes(b"\x1b[H") {
                self.status = format!("Terminal home failed: {error}");
            }
            return;
        }

        if self.ui.page == DesktopPage::Chat && !self.busy {
            self.composer_cursor = 0;
        }
    }

    pub fn cursor_end(&mut self) {
        if self.ui.page == DesktopPage::Terminal {
            if let Err(error) = self.terminal.write_bytes(b"\x1b[F") {
                self.status = format!("Terminal end failed: {error}");
            }
            return;
        }

        if self.ui.page == DesktopPage::Chat && !self.busy {
            self.composer_cursor = self.composer.len();
        }
    }

    pub fn enter(&mut self, shift: bool) {
        if self.ui.page == DesktopPage::Terminal {
            let bytes: &[u8] = if shift { b"\n" } else { b"\r" };
            if let Err(error) = self.terminal.write_bytes(bytes) {
                self.status = format!("Terminal enter failed: {error}");
            }
            return;
        }

        if self.ui.page != DesktopPage::Chat || self.busy {
            return;
        }

        if shift {
            self.clamp_composer_cursor();
            self.composer.insert(self.composer_cursor, '\n');
            self.composer_cursor += 1;
            return;
        }

        self.submit()
    }

    pub fn submit(&mut self) {
        let typed = self.composer.trim().to_string();
        if (typed.is_empty() && self.composer_attachments.is_empty()) || self.busy {
            return;
        }
        if let Some(command) = typed.strip_prefix("/run ") {
            self.composer.clear();
            self.composer_cursor = 0;
            self.run_work(command.trim().to_string());
            return;
        }
        let Some(model) = self
            .conversation
            .settings
            .model
            .clone()
            .or_else(|| self.models.first().map(|m| m.descriptor.id.clone()))
        else {
            self.status =
                "No live model configured. Open Models and import a GGUF/SafeTensors model.".into();
            return;
        };
        self.conversation.settings.model = Some(model.clone());
        let Some(provider) = self.providers.get(&model).cloned() else {
            self.status = format!("Model provider is not loaded: {model}");
            return;
        };

        let attachments = self.composer_attachments.clone();
        let attachment_context = build_attachment_context(&attachments);
        let mut input = if typed.is_empty() {
            "Please review the attached local files/folders.".to_string()
        } else {
            typed
        };
        if !attachment_context.is_empty() {
            input.push_str("\n\n");
            input.push_str(&attachment_context);
        }

        self.composer.clear();
        self.composer_attachments.clear();
        self.busy = true;
        self.streaming.clear();
        self.status = if attachments.is_empty() {
            "Generating…".into()
        } else {
            format!("Generating with {} local attachment(s)…", attachments.len())
        };

        let mut conversation = self.conversation.clone();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(value) => value,
                Err(error) => {
                    let _ = tx.send(BackgroundEvent::ChatFinished(
                        conversation,
                        Some(format!("runtime error: {error}")),
                    ));
                    return;
                }
            };
            let service = ChatService::new(provider);
            let result =
                rt.block_on(
                    service.send_user_message_with(&mut conversation, input, |event| {
                        if let ChatEvent::AssistantDelta(delta) = event {
                            let _ = tx.send(BackgroundEvent::Delta(delta.clone()));
                        }
                    }),
                );
            let error = result.err().map(|value| value.to_string());
            let _ = tx.send(BackgroundEvent::ChatFinished(conversation, error));
        });
    }

    fn run_work(&mut self, command: String) {
        if !self.conversation.settings.code_execution_enabled {
            self.status =
                "Work execution is Ask/Blocked. Enable Project + Build in Processing.".into();
            return;
        }
        let Some(root) = self
            .conversation
            .workspace
            .as_ref()
            .and_then(|w| w.roots.first())
            .map(|r| PathBuf::from(&r.path))
        else {
            self.status = "Attach/continue a project before running Work commands.".into();
            return;
        };
        self.busy = true;
        self.status = "Running Work command…".into();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let sink = ChannelSink { tx: tx.clone() };
            let runner = ProcessRunner::new(sink);
            #[cfg(target_os = "windows")]
            let req = ProcessRequest::new("cmd", ["/C", command.as_str()], &root);
            #[cfg(not(target_os = "windows"))]
            let req = ProcessRequest::new("sh", ["-lc", command.as_str()], &root);
            let result = runner.run(
                req.approved_roots(vec![root.clone()]),
                PermissionDecision::AllowThisProject,
            );
            match result {
                Ok(r) => {
                    let _ = tx.send(BackgroundEvent::ToolFinished {
                        command,
                        stdout: r.stdout,
                        stderr: r.stderr,
                        exit: r.exit_code,
                    });
                }
                Err(e) => {
                    let _ = tx.send(BackgroundEvent::ToolFinished {
                        command,
                        stdout: String::new(),
                        stderr: e.to_string(),
                        exit: 1,
                    });
                }
            }
        });
    }
    fn download_catalog_model(&mut self, index: usize) {
        if self.busy {
            return;
        }
        let Some(entry) = self.catalog.get(index).cloned() else {
            return;
        };
        match self.conversation.settings.network_policy {
            NetworkPolicy::Off => {
                self.status =
                    "Network is OFF. Enable Network in Processing to download models.".into();
                return;
            }
            NetworkPolicy::Ask => {
                self.status="Network approval required. Set Network to ON in Processing, then click Download again.".into();
                return;
            }
            NetworkPolicy::On => {}
        }
        self.busy = true;
        self.status = format!("Installing verified runtime + {}…", entry.display_name);
        let tx = self.tx.clone();
        let dir = self.model_dir.clone();
        let runtime_dir = self.runtime_dir.clone();
        thread::spawn(move || {
            let manifest = match RuntimeManifest::parse(include_str!(
                "../../../resources/inference/runtime-manifest.json"
            )) {
                Ok(value) => value,
                Err(error) => {
                    let _ = tx.send(BackgroundEvent::DownloadFailed(error.to_string()));
                    return;
                }
            };
            if let Err(error) = install_gguf_runtime(&manifest, &runtime_dir, NetworkPolicy::On) {
                let _ = tx.send(BackgroundEvent::DownloadFailed(format!(
                    "runtime install failed: {error}"
                )));
                return;
            }
            match download_model(&entry, &dir, NetworkPolicy::On) {
                Ok(path) => {
                    let record = ModelRecord {
                        descriptor: ModelDescriptor {
                            id: entry.id.clone(),
                            display_name: entry.display_name.clone(),
                            backend: ModelBackend::AetherGguf,
                            format: ModelFormat::Gguf,
                            path: path.to_string_lossy().into_owned(),
                            context_tokens: entry.context_tokens,
                            local: true,
                            load_state: ModelLoadState::Registered,
                        },
                        registered_at: Utc::now(),
                        last_used: None,
                    };
                    let _ = tx.send(BackgroundEvent::LiveAssetsFinished(record));
                }
                Err(e) => {
                    let _ = tx.send(BackgroundEvent::DownloadFailed(e.to_string()));
                }
            }
        });
    }
    pub fn click(&mut self, x: i32, y: i32, width: i32, height: i32) {
        if !self.ui.first_run_complete {
            self.click_first_run(x, y, width, height);
            return;
        }
        if y < 28 && x < 48 {
            self.toggle_navigation();
            return;
        }
        if y < 28 && x > width - 48 {
            self.toggle_processing();
            return;
        }
        let left = if self.ui.navigation.open {
            self.ui.navigation.width as i32
        } else {
            0
        };
        let right = if self.ui.processing.open {
            self.ui.processing.width as i32
        } else {
            0
        };
        if self.ui.navigation.open && x < left {
            let items = [
                DesktopPage::Chat,
                DesktopPage::Projects,
                DesktopPage::Files,
                DesktopPage::Terminal,
                DesktopPage::Models,
                DesktopPage::Artifacts,
                DesktopPage::Search,
                DesktopPage::Activity,
                DesktopPage::Settings,
            ];
            for (i, page) in items.into_iter().enumerate() {
                let hit = Rect {
                    x: 14,
                    y: 82 + i as i32 * 38,
                    w: left - 28,
                    h: 30,
                };
                if hit.contains(x, y) {
                    self.ui.page = page;
                    if page == DesktopPage::Terminal {
                        match self.terminal.ensure_session() {
                            Ok(()) => {
                                self.status = format!(
                                    "Aether Terminal connected • {} capabilities • {}",
                                    TerminalWorkspace::capability_count(),
                                    self.terminal
                                        .active_session_id()
                                        .unwrap_or("session unavailable")
                                );
                            }
                            Err(error) => {
                                self.status = format!("Aether Terminal connection failed: {error}");
                            }
                        }
                    }
                    return;
                }
            }
        }
        if self.ui.processing.open && x >= width - right {
            let px = width - right + 16;
            for (i, preset) in [
                ProcessingPreset::Fast,
                ProcessingPreset::Balanced,
                ProcessingPreset::Deep,
                ProcessingPreset::Maximum,
            ]
            .into_iter()
            .enumerate()
            {
                let hit = Rect {
                    x: px,
                    y: 104 + i as i32 * 34,
                    w: right - 32,
                    h: 28,
                };
                if hit.contains(x, y) {
                    self.conversation.settings.generation =
                        aether_core::GenerationSettings::from_preset(preset);
                    let _ = self.store.save_conversation(&self.conversation);
                    return;
                }
            }
            let work_hit = Rect {
                x: px,
                y: 270,
                w: right - 32,
                h: 32,
            };
            if work_hit.contains(x, y) {
                self.conversation.settings.code_execution_enabled =
                    !self.conversation.settings.code_execution_enabled;
                let _ = self.store.save_conversation(&self.conversation);
                return;
            }
            let network_hit = Rect {
                x: px,
                y: 308,
                w: right - 32,
                h: 32,
            };
            if network_hit.contains(x, y) {
                self.conversation.settings.network_policy =
                    match self.conversation.settings.network_policy {
                        NetworkPolicy::Ask => NetworkPolicy::On,
                        NetworkPolicy::On => NetworkPolicy::Off,
                        NetworkPolicy::Off => NetworkPolicy::Ask,
                    };
                let _ = self.store.save_conversation(&self.conversation);
                self.status = format!(
                    "Network policy: {:?}",
                    self.conversation.settings.network_policy
                );
                return;
            }
        }
        let cx = left;
        let cw = width - left - right;
        if self.ui.page == DesktopPage::Chat {
            let composer_y = height - 92;
            let file_hit = Rect {
                x: cx + 18,
                y: composer_y + 10,
                w: 54,
                h: 34,
            };
            let folder_hit = Rect {
                x: cx + 78,
                y: composer_y + 10,
                w: 68,
                h: 34,
            };
            let send_hit = Rect {
                x: cx + cw - 72,
                y: composer_y + 10,
                w: 54,
                h: 34,
            };

            if file_hit.contains(x, y) {
                self.begin_file_picker(AttachmentKind::File);
                return;
            }
            if folder_hit.contains(x, y) {
                self.begin_file_picker(AttachmentKind::Folder);
                return;
            }
            if send_hit.contains(x, y) {
                self.submit();
                return;
            }

            let chip_y = height - 122;
            for index in 0..self.composer_attachments.len().min(4) {
                let chip = Rect {
                    x: cx + 18 + index as i32 * 150,
                    y: chip_y,
                    w: 142,
                    h: 24,
                };
                if chip.contains(x, y) {
                    self.remove_attachment(index);
                    return;
                }
            }
        }

        if self.ui.page == DesktopPage::Files {
            let up_hit = Rect {
                x: cx + 24,
                y: 92,
                w: 58,
                h: 32,
            };
            let refresh_hit = Rect {
                x: cx + 88,
                y: 92,
                w: 82,
                h: 32,
            };
            let folder_hit = Rect {
                x: cx + 176,
                y: 92,
                w: 168,
                h: 32,
            };
            let cancel_hit = Rect {
                x: cx + 350,
                y: 92,
                w: 72,
                h: 32,
            };

            if up_hit.contains(x, y) {
                self.files_parent();
                return;
            }
            if refresh_hit.contains(x, y) {
                self.refresh_files_surface();
                return;
            }
            if folder_hit.contains(x, y) {
                self.attach_current_folder();
                return;
            }
            if self.file_pick_mode.is_some() && cancel_hit.contains(x, y) {
                self.cancel_file_picker();
                return;
            }

            for index in 0..self.files_entries.len().min(14) {
                let hit = Rect {
                    x: cx + 24,
                    y: 154 + index as i32 * 34,
                    w: cw - 48,
                    h: 30,
                };
                if hit.contains(x, y) {
                    let entry = self.files_entries[index].clone();
                    if entry.is_dir {
                        self.navigate_files_to(entry.path);
                    } else {
                        self.attach_path(entry.path);
                        if self.file_pick_mode == Some(AttachmentKind::File) {
                            self.file_pick_mode = None;
                            self.ui.page = DesktopPage::Chat;
                        }
                    }
                    return;
                }
            }
        }
        if self.ui.page == DesktopPage::Projects {
            for i in 0..self.projects.len().min(12) {
                let hit = Rect {
                    x: cx + 22,
                    y: 92 + i as i32 * 42,
                    w: cw - 44,
                    h: 34,
                };
                if hit.contains(x, y) {
                    self.continue_project(i);
                    return;
                }
            }
        }
        if self.ui.page == DesktopPage::Activity {
            if let Some(action) = health_action_at(x, y, cx, cw) {
                match action {
                    HealthAction::Recheck => self.recheck_health(),
                    HealthAction::Repair => self.repair_health(),
                    HealthAction::RollBack => self.rollback_update(),
                    HealthAction::OpenLogs => self.open_health_logs(),
                    HealthAction::ExportDiagnostic => self.export_health_diagnostic(),
                }
                return;
            }
        }

        if self.ui.page == DesktopPage::Settings {
            if let Some(action) = settings_action_at(x, y, cx, cw) {
                match action {
                    SettingsAction::ConnectChatGpt => {
                        self.pick_and_import_chatgpt_export();
                    }
                    SettingsAction::OpenPrivacyPortal => {
                        self.ui.page = DesktopPage::Projects;
                        self.status = "Imported ChatGPT projects are native AetherAI data. Search and local RAG can use imported conversations directly.".into();
                        self.persist_ui();
                    }
                    SettingsAction::ImportChatGptExport => {
                        self.pick_and_import_chatgpt_export();
                    }
                }
                return;
            }
        }
        if self.ui.page == DesktopPage::Settings {
            if let Some(a) = update_settings_action_at(x, y, cx, cw) {
                match a {
                    UpdateSettingsAction::CheckUpdates => self.check_for_updates(),
                    UpdateSettingsAction::PrepareUpdate => {
                        if self.update.can_prepare() {
                            self.prepare_update()
                        } else {
                            self.status =
                                "Prepare Update is disabled until an update is available.".into()
                        }
                    }
                    UpdateSettingsAction::RestartAndUpdate => {
                        if self.update.can_restart_and_update() {
                            self.restart_and_update()
                        } else {
                            self.status =
                                "Restart & Update is disabled until verification passes.".into()
                        }
                    }
                    UpdateSettingsAction::RollBack => self.rollback_update(),
                }
                return;
            }
        }
        if self.ui.page == DesktopPage::Models && !self.catalog.is_empty() {
            let hit = Rect {
                x: cx + 24,
                y: 132,
                w: 210,
                h: 36,
            };
            if hit.contains(x, y) {
                self.download_catalog_model(0);
            }
        }
    }
    fn recheck_health(&mut self) {
        self.health = HealthPanelState::evaluate(
            &self.data_dir,
            &self.store,
            self.models.len(),
            self.providers.len(),
            &self.update.summary(),
        );
        self.status = format!("System health rechecked — {}", self.health.summary());
    }

    fn repair_health(&mut self) {
        match repair_safe_directories(&self.data_dir) {
            Ok(()) => {
                self.recheck_health();
                self.status = format!(
                    "Safe repair completed — directories verified; {}",
                    self.health.summary()
                );
            }
            Err(error) => {
                self.status = format!("Safe repair failed: {error}");
            }
        }
    }

    fn open_health_logs(&mut self) {
        let logs = self.data_dir.join("logs");
        match std::fs::create_dir_all(&logs) {
            Ok(()) => {
                self.files_root = logs;
                self.refresh_files_surface();
                self.ui.page = DesktopPage::Files;
                self.status = "Opened AetherAI logs folder in Files.".into();
            }
            Err(error) => {
                self.status = format!("Could not open logs folder: {error}");
            }
        }
    }

    fn export_health_diagnostic(&mut self) {
        let downloads = downloads_dir().unwrap_or_else(|| self.data_dir.clone());
        let target = diagnostic_path_in(&downloads);
        match export_diagnostic(&target, &self.health) {
            Ok(()) => {
                self.health.last_export = Some(target.clone());
                self.status = format!("Diagnostic exported: {}", target.display());
            }
            Err(error) => {
                self.status = format!("Diagnostic export failed: {error}");
            }
        }
    }

    fn check_for_updates(&mut self) {
        match self.update.check_for_updates() {
            Ok(()) => self.status = self.update.summary(),
            Err(e) => {
                self.update.mark_failed(e.to_string());
                self.status = self.update.summary();
            }
        }
    }
    fn prepare_update(&mut self) {
        match self.update.prepare_update() {
            Ok(()) => self.status = self.update.summary(),
            Err(e) => {
                self.update.mark_failed(e.to_string());
                self.status = self.update.summary();
            }
        }
    }
    fn restart_and_update(&mut self) {
        match self.update.restart_and_update() {
            Ok(binary) => match std::process::Command::new(&binary).spawn() {
                Ok(_) => {
                    self.status = format!(
                        "Verified update activated from {} — restarting AetherAI…",
                        binary.display()
                    );
                    self.should_quit = true;
                }
                Err(e) => {
                    self.update
                        .mark_failed(format!("activated update could not restart: {e}"));
                    self.status = self.update.summary();
                }
            },
            Err(e) => {
                self.update.mark_failed(e.to_string());
                self.status = self.update.summary();
            }
        }
    }
    fn rollback_update(&mut self) {
        match self.update.rollback() {
            Ok(binary) => match std::process::Command::new(&binary).spawn() {
                Ok(_) => {
                    self.status =
                        "Update recovery completed — restarting previous verified release…".into();
                    self.should_quit = true;
                }
                Err(e) => {
                    self.update
                        .mark_failed(format!("rollback activated but restart failed: {e}"));
                    self.status = self.update.summary();
                }
            },
            Err(e) => {
                self.update.mark_failed(e.to_string());
                self.status = self.update.summary();
            }
        }
    }

    fn pick_and_import_chatgpt_export(&mut self) {
        if self.busy {
            return;
        }

        let downloads = downloads_dir().unwrap_or_else(|| self.data_dir.clone());
        let selected = match current_system_integration().pick_zip_file(&downloads) {
            Ok(Some(path)) => Some(path),
            Ok(None) => latest_chatgpt_export(&downloads),
            Err(error) => {
                self.status = format!("ChatGPT export picker failed: {error}");
                return;
            }
        };

        let Some(archive) = selected else {
            self.status = "No ChatGPT export ZIP selected or found in Downloads.".into();
            return;
        };

        self.start_chatgpt_import(archive);
    }

    fn start_chatgpt_import(&mut self, archive: PathBuf) {
        self.busy = true;
        self.status = format!(
            "Importing all recoverable ChatGPT data from {}…",
            archive.display()
        );

        let tx = self.tx.clone();
        let data_dir = self.data_dir.clone();
        thread::spawn(move || {
            let result = (|| {
                let store = SqliteStore::open(data_dir.join("aetherai.db"))
                    .map_err(|error| error.to_string())?;
                import_chatgpt_export_all(&store, &archive, &data_dir.join("imports"), Utc::now())
                    .map_err(|error| error.to_string())
            })();
            let _ = tx.send(BackgroundEvent::ChatGptImportFinished(result));
        });
    }

    pub fn import_chatgpt_archive_now(
        &self,
        archive: &Path,
    ) -> Result<ChatGptImportSummary, Box<dyn std::error::Error>> {
        Ok(import_chatgpt_export_all(
            &self.store,
            archive,
            &self.data_dir.join("imports"),
            Utc::now(),
        )?)
    }

    fn continue_project(&mut self, index: usize) {
        let Some(p) = self.projects.get(index).cloned() else {
            return;
        };
        let pid = Uuid::new_v4();
        self.conversation.ensure_workspace(&p.name);
        if let Some(w) = self.conversation.workspace.as_mut() {
            w.project_id = Some(pid)
        }
        let _ = self
            .conversation
            .attach_workspace_root(p.root.to_string_lossy(), true);
        let _ = self.store.save_project(&ProjectRecord {
            id: pid,
            name: p.name.clone(),
            root: p.root.to_string_lossy().into_owned(),
            provenance: format!("{:?}", p.provenance),
        });
        let _ = self.store.save_conversation(&self.conversation);
        self.ui.page = DesktopPage::Chat;
        self.status = format!(
            "Project attached: {} — Work is integrated into this chat.",
            p.name
        );
    }
    fn click_first_run(&mut self, x: i32, y: i32, width: i32, height: i32) {
        let next = Rect {
            x: width / 2 + 40,
            y: height - 120,
            w: 130,
            h: 38,
        };
        let skip = Rect {
            x: width / 2 - 170,
            y: height - 120,
            w: 130,
            h: 38,
        };
        if next.contains(x, y) {
            if self.first_run_step >= 6 {
                self.ui.first_run_complete = true;
                self.persist_ui();
                self.status = "AetherAI ready".into()
            } else {
                self.first_run_step += 1
            }
        } else if skip.contains(x, y) {
            self.ui.first_run_complete = true;
            self.persist_ui();
        }
    }
    pub fn render(&self, width: i32, height: i32) -> Vec<DrawCommand> {
        let t = &self.theme;
        let mut d = vec![
            rect(0, 0, width, height, t.app_fill, 0),
            rect(0, 0, width, 28, t.terminal_fill, 0),
            rect(0, 27, width, 1, t.accent_secondary, 0),
            rect(0, 0, 4, height, t.accent, 0),
            rect(92, 5, 34, 18, t.accent_secondary, 9),
            text(99, 19, "AF", t.text_primary),
            text(140, 19, "AETHERAI // LOCAL INTELLIGENCE", t.text_primary),
            text(
                width - 280,
                19,
                &format!(
                    "{}  •  {:?}",
                    self.conversation
                        .settings
                        .model
                        .as_deref()
                        .unwrap_or("No model"),
                    self.conversation.settings.generation.preset
                ),
                t.text_muted,
            ),
            text(
                16,
                19,
                if self.ui.navigation.open {
                    "◀"
                } else {
                    "▶"
                },
                t.accent,
            ),
            text(
                width - 34,
                19,
                if self.ui.processing.open {
                    "▶"
                } else {
                    "◀"
                },
                t.accent,
            ),
        ];
        if !self.ui.first_run_complete {
            self.render_first_run(width, height, &mut d);
            return d;
        }
        let left = if self.ui.navigation.open {
            self.ui.navigation.width as i32
        } else {
            0
        };
        let right = if self.ui.processing.open {
            self.ui.processing.width as i32
        } else {
            0
        };
        if self.ui.navigation.open {
            d.push(rect(
                8,
                58,
                left - 16,
                height - 68,
                t.panel_fallback,
                t.corner_radius as i32,
            ));
            d.push(text(22, 80, "NEW CHAT", t.accent));
            let labels = [
                ("Chat", DesktopPage::Chat),
                ("Projects", DesktopPage::Projects),
                ("Files", DesktopPage::Files),
                ("Terminal", DesktopPage::Terminal),
                ("Models", DesktopPage::Models),
                ("Artifacts", DesktopPage::Artifacts),
                ("Search", DesktopPage::Search),
                ("Activity", DesktopPage::Activity),
                ("Settings", DesktopPage::Settings),
            ];
            for (i, (label, page)) in labels.into_iter().enumerate() {
                let y = 103 + i as i32 * 38;
                if self.ui.page == page {
                    d.push(rect(
                        14,
                        y - 21,
                        left - 28,
                        30,
                        t.panel_fill,
                        t.corner_radius as i32,
                    ));
                }
                d.push(text(
                    24,
                    y,
                    label,
                    if self.ui.page == page {
                        t.text_primary
                    } else {
                        t.text_muted
                    },
                ));
            }
        }
        if self.ui.processing.open {
            let x = width - right;
            d.push(rect(
                x + 8,
                58,
                right - 16,
                height - 68,
                t.panel_fallback,
                t.corner_radius as i32,
            ));
            d.push(text(x + 20, 84, "PROCESSING", t.text_primary));
            for (i, p) in [
                ProcessingPreset::Fast,
                ProcessingPreset::Balanced,
                ProcessingPreset::Deep,
                ProcessingPreset::Maximum,
            ]
            .into_iter()
            .enumerate()
            {
                let y = 124 + i as i32 * 34;
                if self.conversation.settings.generation.preset == p {
                    d.push(rect(
                        x + 16,
                        y - 21,
                        right - 32,
                        28,
                        t.panel_fill,
                        t.corner_radius as i32,
                    ));
                }
                d.push(text(x + 24, y, &format!("{:?}", p), t.text_muted));
            }
            d.push(text(
                x + 20,
                252,
                &format!(
                    "Context  {}",
                    self.conversation.settings.generation.context_tokens
                ),
                t.text_muted,
            ));
            d.push(rect(
                x + 16,
                270,
                right - 32,
                32,
                if self.conversation.settings.code_execution_enabled {
                    t.accent_secondary
                } else {
                    t.terminal_fill
                },
                t.corner_radius as i32,
            ));
            d.push(text(
                x + 24,
                292,
                if self.conversation.settings.code_execution_enabled {
                    "Project + Build: ALLOW"
                } else {
                    "Project + Build: ASK"
                },
                t.text_primary,
            ));
            d.push(rect(
                x + 16,
                308,
                right - 32,
                32,
                t.terminal_fill,
                t.corner_radius as i32,
            ));
            d.push(text(
                x + 24,
                330,
                &format!("Network: {:?}", self.conversation.settings.network_policy),
                t.text_primary,
            ));
            if let Some(r) = &self.resources {
                d.push(text(
                    x + 20,
                    376,
                    &format!(
                        "RAM  {:.1}/{:.1} GiB",
                        (r.total_memory_bytes - r.available_memory_bytes) as f64 / 1073741824.0,
                        r.total_memory_bytes as f64 / 1073741824.0
                    ),
                    t.text_muted,
                ));
                d.push(text(
                    x + 20,
                    398,
                    &format!("CPU  {}%", r.cpu_usage_percent),
                    t.text_muted,
                ));
            }
        }
        let cx = left;
        let cw = width - left - right;
        match self.ui.page {
            DesktopPage::Chat => self.render_chat(cx, cw, height, &mut d),
            DesktopPage::Projects => self.render_projects(cx, cw, &mut d),
            DesktopPage::Files => self.render_files(cx, cw, height, &mut d),
            DesktopPage::Terminal => self.render_terminal(cx, cw, height, &mut d),
            DesktopPage::Models => self.render_models(cx, cw, &mut d),
            DesktopPage::Activity => self.render_activity(cx, cw, &mut d),
            DesktopPage::Settings => self.render_settings(cx, cw, &mut d),
            page => {
                d.push(text(cx + 26, 86, &format!("{:?}", page), t.text_primary));
                d.push(text(cx+26,116,"This capability lives in the same AetherAI application and conversation system.",t.text_muted));
            }
        }
        d.push(text(cx + 18, height - 8, &self.status, t.text_muted));
        d
    }
    fn composer_text_with_caret(&self, max_chars: usize) -> String {
        let mut cursor = self.composer_cursor.min(self.composer.len());
        while cursor > 0 && !self.composer.is_char_boundary(cursor) {
            cursor -= 1;
        }

        let caret_char = self.composer[..cursor].chars().count();
        let chars: Vec<char> = self.composer.chars().collect();
        let max_chars = max_chars.max(8);
        let start = caret_char.saturating_sub(max_chars / 2);
        let end = (start + max_chars.saturating_sub(1)).min(chars.len());

        let mut visible: String = chars[start..end]
            .iter()
            .map(|ch| if *ch == '\n' { '↵' } else { *ch })
            .collect();

        let caret_in_visible = caret_char
            .saturating_sub(start)
            .min(visible.chars().count());
        let caret_byte = visible
            .char_indices()
            .nth(caret_in_visible)
            .map(|(index, _)| index)
            .unwrap_or(visible.len());

        visible.insert_str(caret_byte, "│");

        if start > 0 {
            visible.insert(0, '…');
        }
        if end < chars.len() {
            visible.push('…');
        }

        visible
    }

    fn render_chat(&self, cx: i32, cw: i32, height: i32, d: &mut Vec<DrawCommand>) {
        let t = &self.theme;
        d.push(text(cx + 24, 82, &self.conversation.title, t.text_primary));
        if let Some(workspace) = &self.conversation.workspace {
            d.push(text(
                cx + 24,
                104,
                &format!(
                    "Workspace: {}  •  {} root(s)",
                    workspace.name,
                    workspace.roots.len()
                ),
                t.accent_secondary,
            ));
        }

        let mut y = 132;
        let available = ((height - 292) / 44).max(3) as usize;
        for message in self
            .conversation
            .messages
            .iter()
            .rev()
            .take(available)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            let label = match message.role {
                Role::User => "YOU",
                Role::Assistant => "AETHERAI",
                Role::Tool => "WORK",
                Role::System => "SYSTEM",
                Role::Developer => "DEV",
            };
            let color = if message.role == Role::User {
                t.accent
            } else if message.role == Role::Tool {
                t.accent_secondary
            } else {
                t.text_primary
            };
            d.push(text(cx + 24, y, label, color));
            d.push(text(
                cx + 24,
                y + 20,
                &compact(&message.content, ((cw - 70) / 8).max(20) as usize),
                t.text_muted,
            ));
            y += 48;
        }

        if self.busy {
            d.push(text(cx + 24, y, "AETHERAI", t.accent_secondary));
            d.push(text(
                cx + 24,
                y + 20,
                &compact(&self.streaming, ((cw - 70) / 8).max(20) as usize),
                t.text_primary,
            ));
        }

        let chip_y = height - 110;
        if !self.composer_attachments.is_empty() {
            d.push(text(cx + 18, chip_y - 8, "ATTACHED", t.accent_secondary));
            for (index, attachment) in self.composer_attachments.iter().take(4).enumerate() {
                let x = cx + 18 + index as i32 * 150;
                d.push(rect(
                    x,
                    chip_y,
                    142,
                    24,
                    t.panel_fill,
                    t.corner_radius as i32,
                ));
                let prefix = match attachment.kind {
                    AttachmentKind::File => "F",
                    AttachmentKind::Folder => "D",
                };
                d.push(text(
                    x + 8,
                    chip_y + 17,
                    &format!("× {prefix} {}", compact(&attachment.display_name, 12)),
                    t.text_primary,
                ));
            }
        }

        let composer_y = height - 92;
        d.push(rect(
            cx + 16,
            composer_y,
            cw - 32,
            64,
            t.terminal_fill,
            t.corner_radius as i32,
        ));
        d.push(rect(
            cx + 18,
            composer_y + 10,
            54,
            34,
            t.panel_fill,
            t.corner_radius as i32,
        ));
        d.push(text(cx + 28, composer_y + 33, "+ FILE", t.accent));
        d.push(rect(
            cx + 78,
            composer_y + 10,
            68,
            34,
            t.panel_fill,
            t.corner_radius as i32,
        ));
        d.push(text(cx + 87, composer_y + 33, "+ FOLDER", t.accent));

        let prompt = if self.composer.is_empty() {
            "│  Message AetherAI…  Shift+Enter for newline".into()
        } else {
            self.composer_text_with_caret(((cw - 270) / 8).max(20) as usize)
        };
        d.push(text(
            cx + 160,
            composer_y + 34,
            &prompt,
            if self.composer.is_empty() {
                t.text_muted
            } else {
                t.text_primary
            },
        ));

        d.push(rect(
            cx + cw - 72,
            composer_y + 10,
            54,
            34,
            if self.busy { t.panel_fill } else { t.accent },
            t.corner_radius as i32,
        ));
        d.push(text(
            cx + cw - 58,
            composer_y + 33,
            if self.busy { "…" } else { "SEND" },
            t.text_primary,
        ));
    }

    fn render_files(&self, cx: i32, cw: i32, _height: i32, d: &mut Vec<DrawCommand>) {
        let t = &self.theme;
        d.push(text(cx + 24, 82, "FILES", t.text_primary));
        d.push(text(
            cx + 24,
            106,
            &compact(
                &self.files_root.display().to_string(),
                ((cw - 48) / 8).max(24) as usize,
            ),
            t.accent_secondary,
        ));

        for (x, width, label) in [
            (cx + 24, 58, "UP"),
            (cx + 88, 82, "REFRESH"),
            (cx + 176, 168, "ATTACH THIS FOLDER"),
        ] {
            d.push(rect(x, 92, width, 32, t.panel_fill, t.corner_radius as i32));
            d.push(text(x + 10, 114, label, t.text_primary));
        }

        if self.file_pick_mode.is_some() {
            d.push(rect(
                cx + 350,
                92,
                72,
                32,
                t.panel_fill,
                t.corner_radius as i32,
            ));
            d.push(text(cx + 362, 114, "CANCEL", t.text_muted));
        }

        let instruction = match self.file_pick_mode {
            Some(AttachmentKind::File) => "Choose a file. Click a folder to browse into it.",
            Some(AttachmentKind::Folder) => {
                "Browse to the target directory, then ATTACH THIS FOLDER."
            }
            None => "Browse local files. Click a file to attach it to the current chat.",
        };
        d.push(text(cx + 24, 144, instruction, t.text_muted));

        if self.files_entries.is_empty() {
            d.push(text(
                cx + 24,
                184,
                "No readable entries in this folder.",
                t.text_muted,
            ));
        }

        for (index, entry) in self.files_entries.iter().take(14).enumerate() {
            let y = 176 + index as i32 * 34;
            d.push(rect(
                cx + 24,
                y - 22,
                cw - 48,
                30,
                t.panel_fallback,
                t.corner_radius as i32,
            ));
            let marker = if entry.is_dir { "DIR " } else { "FILE" };
            let detail = if entry.is_dir {
                String::new()
            } else {
                format!("  {:.1} KiB", entry.size_bytes as f64 / 1024.0)
            };
            d.push(text(
                cx + 34,
                y,
                &format!(
                    "{marker}  {}{}",
                    compact(&entry.display_name, ((cw - 180) / 8).max(16) as usize),
                    detail
                ),
                if entry.is_dir {
                    t.accent_secondary
                } else {
                    t.text_primary
                },
            ));
        }
    }

    fn render_terminal(&self, cx: i32, cw: i32, height: i32, d: &mut Vec<DrawCommand>) {
        let t = &self.theme;
        d.push(text(cx + 24, 82, "AETHERFORGE TERMINAL", t.text_primary));
        d.push(text(
            cx + 24,
            104,
            &format!(
                "{} • {} capabilities • {} pane(s)",
                self.terminal.current_title(),
                TerminalWorkspace::capability_count(),
                self.terminal.pane_count(),
            ),
            t.accent_secondary,
        ));
        if let Some(cwd) = self.terminal.current_cwd() {
            d.push(text(cx + 24, 124, &format!("CWD  {cwd}"), t.text_muted));
        }
        d.push(text(
            cx + 24,
            146,
            "NEW TAB  •  SPLIT ↔  •  SPLIT ↕  •  FIND  •  CLEAR",
            t.accent,
        ));

        let snapshot = self.terminal.snapshot_text();
        let max_lines = ((height - 210) / 18).max(4) as usize;
        let width_chars = ((cw - 54) / 8).max(20) as usize;
        let rows: Vec<_> = snapshot.lines().collect();
        let start = rows.len().saturating_sub(max_lines);
        let mut y = 176;
        for line in rows.into_iter().skip(start) {
            d.push(text(
                cx + 24,
                y,
                &compact(line, width_chars),
                t.text_primary,
            ));
            y += 18;
        }
        if self.terminal.tab_count() == 0 {
            d.push(text(
                cx + 24,
                198,
                "Select Terminal to start or reattach an Aether Terminal session.",
                t.text_muted,
            ));
        }
    }

    fn render_projects(&self, cx: i32, cw: i32, d: &mut Vec<DrawCommand>) {
        let t = &self.theme;
        d.push(text(cx + 24, 82, "Recovered Projects", t.text_primary));
        for (i, p) in self.projects.iter().take(12).enumerate() {
            let y = 110 + i as i32 * 42;
            d.push(rect(
                cx + 18,
                y - 22,
                cw - 36,
                34,
                t.panel_fallback,
                t.corner_radius as i32,
            ));
            d.push(text(
                cx + 28,
                y,
                &format!("{}  [{:?}]  {:?}", p.name, p.provenance, p.kind),
                t.text_primary,
            ));
        }
    }
    fn render_models(&self, cx: i32, _cw: i32, d: &mut Vec<DrawCommand>) {
        let t = &self.theme;
        d.push(text(
            cx + 24,
            82,
            "Models — real local inference",
            t.text_primary,
        ));
        if let Some(m) = self.catalog.first() {
            d.push(text(
                cx + 24,
                112,
                &format!(
                    "Starter: {} • {} • {:.0} MB",
                    m.display_name,
                    m.license,
                    m.size_bytes as f64 / 1_000_000.0
                ),
                t.text_muted,
            ));
            d.push(rect(
                cx + 24,
                132,
                210,
                36,
                t.accent_secondary,
                t.corner_radius as i32,
            ));
            d.push(text(
                cx + 42,
                156,
                "Install Runtime + Model",
                t.text_primary,
            ));
        }
        if self.models.is_empty() {
            d.push(text(cx+24,194,"No model installed yet. Network defaults to ASK; approve it in Processing before download.",t.text_muted));
        }
        for (i, m) in self.models.iter().enumerate() {
            d.push(text(
                cx + 24,
                226 + i as i32 * 34,
                &format!(
                    "{}  {:?}  {:?}",
                    m.descriptor.display_name, m.descriptor.backend, m.descriptor.load_state
                ),
                t.text_muted,
            ));
        }
    }
    fn render_settings(&self, cx: i32, cw: i32, d: &mut Vec<DrawCommand>) {
        let t = &self.theme;
        let button_width = (cw - 48).clamp(220, 420);

        d.push(text(
            cx + 24,
            82,
            "Settings · Data & Integrations",
            t.text_primary,
        ));
        d.push(text(cx + 24, 108, "ChatGPT", t.accent));
        d.push(text(
            cx + 24,
            128,
            "CHATGPT LIBRARY • LOCAL • FREE • No API key • No browser.",
            t.text_muted,
        ));

        d.push(rect(
            cx + 24,
            138,
            button_width,
            38,
            t.accent_secondary,
            t.corner_radius as i32,
        ));
        d.push(text(cx + 42, 163, "Import ChatGPT Library", t.text_primary));

        d.push(rect(
            cx + 24,
            194,
            button_width,
            38,
            t.terminal_fill,
            t.corner_radius as i32,
        ));
        d.push(text(cx + 42, 219, "Open Imported Projects", t.text_primary));

        d.push(text(
            cx + 24,
            256,
            "Choose an official ChatGPT export ZIP. AetherAI reconstructs it locally into native projects, search, RAG, and conversation history.",
            t.text_muted,
        ));

        d.push(rect(
            cx + 24,
            280,
            button_width,
            42,
            t.accent,
            t.corner_radius as i32,
        ));
        d.push(text(
            cx + 42,
            307,
            "Re-import ChatGPT Export",
            t.text_primary,
        ));

        d.push(text(
            cx + 24,
            350,
            "Import All reconstructs every recoverable exported project, chat, message and attachment reference.",
            t.text_muted,
        ));
        d.push(text(
            cx + 24,
            374,
            "Imported ChatGPT content stays local with typed provenance and can be continued with AetherAI local models at $0.",
            t.text_muted,
        ));
        d.push(text(
            cx + 24,
            382,
            "APPLICATION LIFECYCLE",
            t.accent_secondary,
        ));
        d.push(text(
            cx + 24,
            406,
            &compact(&self.update.summary(), 82),
            t.text_muted,
        ));
        for (y, label, active) in [
            (430, "Check Updates", true),
            (472, "Prepare Update", self.update.can_prepare()),
            (
                514,
                "Restart & Update",
                self.update.can_restart_and_update(),
            ),
            (556, "Roll Back", true),
        ] {
            d.push(rect(
                cx + 24,
                y,
                (cw - 48).clamp(220, 420),
                36,
                if active {
                    t.panel_fill
                } else {
                    t.panel_fallback
                },
                t.corner_radius as i32,
            ));
            d.push(text(
                cx + 38,
                y + 24,
                label,
                if active { t.text_primary } else { t.text_muted },
            ));
        }
        if let Some(n) = &self.update.recovery_notice {
            d.push(text(cx + 24, 612, &compact(n, 82), t.text_muted));
        }
    }

    fn render_activity(&self, cx: i32, cw: i32, d: &mut Vec<DrawCommand>) {
        let t = &self.theme;
        d.push(text(cx + 24, 78, "SYSTEM HEALTH", t.text_primary));
        d.push(text(
            cx + 24,
            98,
            &self.health.summary(),
            if self.health.failed_count() == 0 {
                t.accent_secondary
            } else {
                t.danger
            },
        ));

        let available = (cw - 48).max(300);
        let button_width = ((available - 32) / 5).max(56);
        for (index, (label, action)) in [
            ("Recheck", HealthAction::Recheck),
            ("Repair", HealthAction::Repair),
            ("Roll Back", HealthAction::RollBack),
            ("Open Logs", HealthAction::OpenLogs),
            ("Export Diagnostic", HealthAction::ExportDiagnostic),
        ]
        .into_iter()
        .enumerate()
        {
            let enabled = self.health.actions.contains(&action);
            let x = cx + 24 + index as i32 * (button_width + 8);
            d.push(rect(
                x,
                110,
                button_width,
                34,
                if enabled {
                    t.panel_fill
                } else {
                    t.panel_fallback
                },
                t.corner_radius as i32,
            ));
            d.push(text(
                x + 8,
                133,
                label,
                if enabled {
                    t.text_primary
                } else {
                    t.text_muted
                },
            ));
        }

        for (index, check) in self.health.checks.iter().take(11).enumerate() {
            let y = 174 + index as i32 * 32;
            let state = match check.state {
                HealthState::Pass => "PASS",
                HealthState::Warn => "WARN",
                HealthState::Fail => "FAIL",
            };
            let state_color = match check.state {
                HealthState::Pass => t.accent_secondary,
                HealthState::Warn => t.accent,
                HealthState::Fail => t.danger,
            };

            d.push(text(cx + 24, y, state, state_color));
            d.push(text(cx + 82, y, &check.name, t.text_primary));
            d.push(text(
                cx + 260,
                y,
                &compact(&check.detail, ((cw - 300) / 8).max(22) as usize),
                t.text_muted,
            ));
        }

        let activity_y = 542;
        d.push(text(
            cx + 24,
            activity_y,
            "RECENT BACKGROUND ACTIVITY",
            t.accent_secondary,
        ));

        if self.activities.is_empty() {
            d.push(text(
                cx + 24,
                activity_y + 24,
                "No background work activity recorded yet.",
                t.text_muted,
            ));
        }

        for (index, activity) in self.activities.iter().rev().take(4).enumerate() {
            d.push(text(
                cx + 24,
                activity_y + 26 + index as i32 * 24,
                &compact(&format!("{activity:?}"), ((cw - 48) / 8).max(24) as usize),
                t.text_muted,
            ));
        }

        if let Some(path) = &self.health.last_export {
            d.push(text(
                cx + 24,
                activity_y + 132,
                &format!("Last diagnostic: {}", path.display()),
                t.text_muted,
            ));
        }
    }

    fn render_first_run(&self, width: i32, height: i32, d: &mut Vec<DrawCommand>) {
        let t = &self.theme;
        let x = width / 2 - 360;
        let y = 100;
        d.push(rect(
            x,
            y,
            720,
            height - 190,
            t.panel_fallback,
            t.corner_radius as i32,
        ));
        d.push(text(
            x + 34,
            y + 48,
            "Welcome to AetherAI v0.3.7",
            t.text_primary,
        ));
        let(title,body)=match self.first_run_step{0=>("System Check",format!("Platform: {}    RAM: {:.1} GiB",current_system_integration().platform_name(),self.resources.as_ref().map(|r|r.total_memory_bytes as f64/1073741824.0).unwrap_or(0.0))),1=>("Find Existing Projects",format!("{} candidate projects found in approved common folders. Nothing is imported automatically.",self.projects.len())),2=>("Find Existing Models",format!("{} local models are already registered. Model files stay where they are.",self.models.len())),3=>("Model Storage","Use your existing model folders or the AetherAI data directory. GGUF and SafeTensors-class assets are registered in place.".into()),4=>("Hardware Optimization","Processing presets expose context, CPU threads and GPU-layer controls; recommendations remain user-overridable.".into()),5=>("Permissions","Files, terminal, network, sensors and system integration are capability-gated. Project + Build defaults to ASK.".into()),_=>("Ready","Normal chat and heavy Work use one conversation identity. Local model inference is live; no mock provider is used.".into())};
        d.push(text(x + 34, y + 104, title, t.accent));
        d.push(text(x + 34, y + 144, &compact(&body, 85), t.text_muted));
        d.push(rect(
            width / 2 - 170,
            height - 120,
            130,
            38,
            t.terminal_fill,
            t.corner_radius as i32,
        ));
        d.push(text(width / 2 - 140, height - 94, "Skip", t.text_muted));
        d.push(rect(
            width / 2 + 40,
            height - 120,
            130,
            38,
            t.accent_secondary,
            t.corner_radius as i32,
        ));
        d.push(text(
            width / 2 + 70,
            height - 94,
            if self.first_run_step >= 6 {
                "Start"
            } else {
                "Next"
            },
            t.text_primary,
        ));
    }
}
fn default_files_root(data_dir: &Path) -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .unwrap_or_else(|| data_dir.to_path_buf())
}

fn scan_files_surface(root: &Path) -> Vec<FileSurfaceEntry> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut output = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let metadata = entry.metadata().ok()?;
            if !metadata.is_dir() && !metadata.is_file() {
                return None;
            }
            Some(FileSurfaceEntry {
                display_name: entry.file_name().to_string_lossy().into_owned(),
                path,
                is_dir: metadata.is_dir(),
                size_bytes: if metadata.is_file() {
                    metadata.len()
                } else {
                    0
                },
            })
        })
        .collect::<Vec<_>>();
    output.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then_with(|| {
            a.display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase())
        })
    });
    output.truncate(200);
    output
}

fn read_text_preview(path: &Path, byte_limit: u64) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(byte_limit).read_to_end(&mut bytes).ok()?;
    String::from_utf8(bytes).ok()
}

fn collect_folder_preview(root: &Path, files: &mut Vec<PathBuf>, depth: usize) {
    if depth > 3 || files.len() >= MAX_FOLDER_PREVIEW_FILES {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        if files.len() >= MAX_FOLDER_PREVIEW_FILES {
            break;
        }
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            collect_folder_preview(&path, files, depth + 1);
        } else if metadata.is_file() {
            files.push(path);
        }
    }
}

fn build_attachment_context(attachments: &[ComposerAttachment]) -> String {
    if attachments.is_empty() {
        return String::new();
    }

    let mut output = String::from("AetherAI attachments (local-only context):\n");
    for attachment in attachments {
        match attachment.kind {
            AttachmentKind::File => {
                output.push_str(&format!(
                    "\n[Attached file: {} | {} bytes]\n",
                    attachment.display_name, attachment.size_bytes
                ));
                match read_text_preview(&attachment.path, MAX_ATTACHMENT_PREVIEW_BYTES) {
                    Some(preview) => output.push_str(&preview),
                    None => output.push_str(
                        "(Binary, non-UTF-8, unreadable, or unsupported for text preview.)",
                    ),
                }
            }
            AttachmentKind::Folder => {
                output.push_str(&format!(
                    "\n[Attached folder: {}]\n",
                    attachment.display_name
                ));
                let mut files = Vec::new();
                collect_folder_preview(&attachment.path, &mut files, 0);
                let mut remaining = MAX_FOLDER_PREVIEW_BYTES;
                for file in files {
                    if remaining == 0 {
                        break;
                    }
                    let relative = file
                        .strip_prefix(&attachment.path)
                        .unwrap_or(&file)
                        .display()
                        .to_string();
                    output.push_str(&format!("\n--- {relative} ---\n"));
                    let limit = remaining.min(MAX_ATTACHMENT_PREVIEW_BYTES);
                    match read_text_preview(&file, limit) {
                        Some(preview) => {
                            remaining = remaining.saturating_sub(preview.len() as u64);
                            output.push_str(&preview);
                        }
                        None => output.push_str("(Binary/non-UTF-8 file omitted.)"),
                    }
                }
                if remaining == 0 {
                    output.push_str("\n(Folder preview byte limit reached.)");
                }
            }
        }
    }
    output
}

fn build_providers(
    models: &[ModelRecord],
    c: &Conversation,
    runtime_dir: &Path,
) -> HashMap<String, Arc<dyn ModelProvider>> {
    let mut map = HashMap::new();
    for m in models {
        let d = m.descriptor.clone();
        let mut h = DefaultHasher::new();
        d.id.hash(&mut h);
        let offset = (h.finish() % 800) as u16;
        let p: Arc<dyn ModelProvider> = match d.backend {
            ModelBackend::AetherGguf => {
                if let Some(port) = env::var("AETHERAI_GGUF_EXTERNAL_PORT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                {
                    Arc::new(GgufProvider::external(d.clone(), port))
                } else {
                    let exe = env::var_os("AETHERAI_GGUF_RUNTIME")
                        .map(PathBuf::from)
                        .unwrap_or_else(|| {
                            let installed = stable_runtime_path(runtime_dir);
                            if installed.is_file() {
                                installed
                            } else {
                                default_packaged_gguf_runtime()
                            }
                        });
                    Arc::new(GgufProvider::managed(
                        d.clone(),
                        exe,
                        18080 + offset,
                        c.settings.generation.cpu_threads,
                        c.settings.generation.gpu_layers,
                    ))
                }
            }
            ModelBackend::AetherNative => {
                if let Some(port) = env::var("AETHERAI_NATIVE_EXTERNAL_PORT")
                    .ok()
                    .and_then(|v| v.parse().ok())
                {
                    Arc::new(NativeProvider::external(d.clone(), port))
                } else {
                    Arc::new(NativeProvider::managed_mistralrs(
                        d.clone(),
                        env::var("AETHERAI_NATIVE_RUNTIME").unwrap_or_else(|_| "mistralrs".into()),
                        19080 + offset,
                    ))
                }
            }
        };
        map.insert(d.id.clone(), p);
    }
    map
}
fn default_packaged_gguf_runtime() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from("tools/inference/aetherai-gguf-runtime.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("tools/inference/aetherai-gguf-runtime")
    }
}
fn discover_common_projects() -> Vec<ProjectCandidate> {
    let mut out = Vec::new();
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return out;
    };
    let cfg = DiscoveryConfig {
        max_depth: 5,
        ..DiscoveryConfig::default()
    };
    for p in [
        home.join("Downloads"),
        home.join("Documents"),
        home.join("Projects"),
        home.join("Source"),
        home.join("repos"),
    ] {
        if p.is_dir() {
            if let Ok(mut v) = scan_root(&p, &cfg) {
                out.append(&mut v)
            }
        }
    }
    out.sort_by(|a, b| a.root.cmp(&b.root));
    out.dedup_by(|a, b| a.root == b.root);
    out
}
fn downloads_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .map(|home| home.join("Downloads"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        env::var_os("HOME")
            .map(PathBuf::from)
            .map(|home| home.join("Downloads"))
    }
}

fn latest_chatgpt_export(downloads: &Path) -> Option<PathBuf> {
    let mut candidates = std::fs::read_dir(downloads)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file()
                || !path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
            {
                return None;
            }
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !(name.contains("chatgpt") || name.contains("openai") || name.contains("export")) {
                return None;
            }
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, path))
        })
        .collect::<Vec<_>>();

    candidates.sort_by_key(|left| std::cmp::Reverse(left.0));
    candidates.into_iter().next().map(|(_, path)| path)
}

pub fn import_model_record(path: &Path) -> Result<ModelRecord, Box<dyn std::error::Error>> {
    let canonical = std::fs::canonicalize(path)?;
    let name = canonical
        .file_stem()
        .or_else(|| canonical.file_name())
        .and_then(|v| v.to_str())
        .unwrap_or("model")
        .replace(
            |c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_',
            "-",
        );
    let gguf = canonical
        .extension()
        .and_then(|v| v.to_str())
        .is_some_and(|x| x.eq_ignore_ascii_case("gguf"));
    Ok(ModelRecord {
        descriptor: ModelDescriptor {
            id: format!("local/{name}"),
            display_name: name,
            backend: if gguf {
                ModelBackend::AetherGguf
            } else {
                ModelBackend::AetherNative
            },
            format: if gguf {
                ModelFormat::Gguf
            } else {
                ModelFormat::SafeTensors
            },
            path: canonical.to_string_lossy().into_owned(),
            context_tokens: 32768,
            local: true,
            load_state: aether_model_api::ModelLoadState::Registered,
        },
        registered_at: Utc::now(),
        last_used: None,
    })
}
fn rect(x: i32, y: i32, w: i32, h: i32, color: Rgba, radius: i32) -> DrawCommand {
    DrawCommand::Rect {
        rect: Rect { x, y, w, h },
        color,
        radius,
    }
}
fn text(x: i32, y: i32, s: &str, color: Rgba) -> DrawCommand {
    DrawCommand::Text {
        x,
        y,
        text: s.to_string(),
        color,
    }
}
fn compact(s: &str, max: usize) -> String {
    let one = s.replace('\n', "  ");
    if one.chars().count() <= max {
        return one;
    }
    let mut out = one.chars().take(max.saturating_sub(1)).collect::<String>();
    out.push('…');
    out
}

// AetherAI v0.3.7 native eframe host bridge.
impl AetherApp {
    #[must_use]
    pub fn native_active_page(&self) -> DesktopPage {
        self.ui.page
    }
    pub fn native_toggle_navigation(&mut self) {
        self.ui.navigation.toggle();
    }

    pub fn native_first_frame_startup_health(&mut self) {
        if !self.startup_health_pending {
            return;
        }

        if !self.health.startup_minimum_healthy() {
            self.status =
                "Pending release startup health is not green; rollback remains armed.".into();
            return;
        }

        match mark_pending_release_healthy() {
            Ok(Some(version)) => {
                self.startup_health_pending = false;
                self.status =
                    format!("AetherAI {version} startup health accepted on first rendered frame.");
            }
            Ok(None) => {
                self.startup_health_pending = false;
            }
            Err(error) => {
                self.status = format!("Startup health acknowledgement failed: {error}");
            }
        }
    }

    pub fn native_should_quit(&self) -> bool {
        self.should_quit
    }
    pub fn native_ctrl_c(&mut self) {
        if self.ui.page == DesktopPage::Terminal
            && let Err(error) = self.terminal.write_bytes(&[0x03])
        {
            self.status = format!("Terminal Ctrl+C failed: {error}");
        }
    }
    pub fn native_terminal_scroll(&mut self, rows: i32) {
        if self.ui.page == DesktopPage::Terminal {
            if rows != 0 {
                self.terminal.scroll(rows);
            }
            let _ = self.terminal.observe_monitoring();
        }
    }
    pub fn native_pointer_click(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.click(x, y, width, height);
    }
}

#[cfg(test)]
mod task5_composer_files_tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("aetherai-task5-{name}-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create task5 temp root");
        root
    }

    #[test]
    fn task5_files_surface_lists_directories_before_files() {
        let root = temp_root("surface");
        std::fs::create_dir(root.join("folder")).expect("create folder");
        std::fs::write(root.join("file.txt"), b"hello").expect("write file");
        let entries = scan_files_surface(&root);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].is_dir);
        assert!(!entries[1].is_dir);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn task5_attachment_context_reads_local_utf8_and_is_bounded() {
        let root = temp_root("context");
        let file = root.join("notes.txt");
        std::fs::write(&file, "hello from local attachment").expect("write text");
        let attachment = ComposerAttachment {
            path: file,
            display_name: "notes.txt".into(),
            kind: AttachmentKind::File,
            size_bytes: 27,
        };
        let context = build_attachment_context(&[attachment]);
        assert!(context.contains("AetherAI attachments"));
        assert!(context.contains("hello from local attachment"));
        assert!(context.len() < MAX_ATTACHMENT_PREVIEW_BYTES as usize + 1024);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn task5_folder_preview_caps_file_count() {
        let root = temp_root("folder");
        for index in 0..(MAX_FOLDER_PREVIEW_FILES + 8) {
            std::fs::write(root.join(format!("{index:03}.txt")), "x").expect("write file");
        }
        let mut files = Vec::new();
        collect_folder_preview(&root, &mut files, 0);
        assert_eq!(files.len(), MAX_FOLDER_PREVIEW_FILES);
        let _ = std::fs::remove_dir_all(root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn td() -> PathBuf {
        let p = env::temp_dir().join(format!("aether-desktop-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn unified_project_keeps_chat_identity() {
        let d = td();
        let mut a = AetherApp::new(&d).unwrap();
        let id = a.conversation.id;
        let p = d.join("project");
        std::fs::create_dir_all(&p).unwrap();
        std::fs::write(p.join("Cargo.toml"), "[workspace]").unwrap();
        a.projects = scan_root(&p, &DiscoveryConfig::default()).unwrap();
        a.continue_project(0);
        assert_eq!(a.conversation.id, id);
        assert!(a.conversation.workspace.is_some());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn settings_exposes_native_chatgpt_library_actions() {
        let root =
            std::env::temp_dir().join(format!("aetherai-stage6-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let mut app = AetherApp::new(&root).unwrap();
        app.ui.first_run_complete = true;
        app.ui.page = DesktopPage::Settings;

        let rendered = app.render(1440, 900);
        let labels = rendered
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(labels.contains(&"Import ChatGPT Library"));
        assert!(labels.contains(&"Open Imported Projects"));
        assert!(labels.contains(&"Re-import ChatGPT Export"));
        assert!(labels.iter().any(|line| line.contains("CHATGPT LIBRARY")));
        assert!(labels.iter().any(|line| line.contains("No API key")));

        assert_eq!(
            settings_action_at(340, 160, 272, 836),
            Some(SettingsAction::ConnectChatGpt)
        );
        assert_eq!(
            settings_action_at(340, 216, 272, 836),
            Some(SettingsAction::OpenPrivacyPortal)
        );
        assert_eq!(
            settings_action_at(340, 302, 272, 836),
            Some(SettingsAction::ImportChatGptExport)
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn panel_persistence_round_trip() {
        let d = td();
        {
            let mut a = AetherApp::new(&d).unwrap();
            a.ui.navigation.open = false;
            a.ui.processing.width = 401.0;
            a.persist_ui();
        }
        {
            let a = AetherApp::new(&d).unwrap();
            assert!(!a.ui.navigation.open);
            assert_eq!(a.ui.processing.width, 401.0);
        }
        let _ = std::fs::remove_dir_all(d);
    }
    #[test]
    fn no_model_status_is_real() {
        let d = td();
        let a = AetherApp::new(&d).unwrap();
        assert!(a.models.is_empty());
        assert!(a.status.contains("No live model"));
        let _ = std::fs::remove_dir_all(d);
    }
    #[test]
    fn starter_catalog_is_licensed_and_pinned() {
        let d = td();
        let a = AetherApp::new(&d).unwrap();
        let m = a.catalog.first().unwrap();
        assert_eq!(m.license, "Apache-2.0");
        assert_eq!(m.revision, "1208e45d782fe18602c5eaf10e5758d5b0f24c03");
        assert_eq!(
            m.sha256,
            "b0638f08417a2d3c8652760462eb5407c6e30173cf9608ad0820757a281eea0e"
        );
        let _ = std::fs::remove_dir_all(d);
    }
    #[test]
    fn default_network_policy_requires_approval_for_live_assets() {
        let d = td();
        let mut a = AetherApp::new(&d).unwrap();
        assert_eq!(a.conversation.settings.network_policy, NetworkPolicy::Ask);
        a.download_catalog_model(0);
        assert!(!a.busy);
        assert!(a.status.contains("approval required"));
        let _ = std::fs::remove_dir_all(d);
    }
}
