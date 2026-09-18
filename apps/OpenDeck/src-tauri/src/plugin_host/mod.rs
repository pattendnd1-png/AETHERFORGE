use crate::editor;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Emitter, State};
use tokio::{
    net::TcpListener,
    process::Command,
    sync::broadcast,
};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use uuid::Uuid;
use walkdir::WalkDir;
use zip::ZipArchive;

const HOST_PROTOCOL_VERSION: &str = "2.0.47";
const STREAM_DECK_COMPATIBILITY_TARGET: &str = "7.6";
const DEVICE_ID: &str = "opendeck-stream-deck-plus";
const DEVICE_TYPE_STREAM_DECK_PLUS: u8 = 7;
const PLUGIN_PROCESS_EVENT: &str = "opendeck://plugin-process";
const PLUGIN_FEEDBACK_EVENT: &str = "opendeck://plugin-feedback";
const PLUGIN_CATALOG_EVENT: &str = "opendeck://plugin-catalog";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginActionDescriptor {
    id: String,
    plugin_uuid: String,
    uuid: String,
    name: String,
    category: String,
    controllers: Vec<String>,
    supported_kinds: Vec<String>,
    default_interaction: String,
    property_inspector_path: Option<String>,
    states: Vec<PluginStateDescriptor>,
    supported_in_multi_actions: bool,
    supported_in_key_logic_actions: bool,
    #[serde(default)]
    disable_automatic_states: bool,
    #[serde(default = "default_true")]
    visible_in_actions_list: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginStateDescriptor {
    name: Option<String>,
    image: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginProfileDescriptor {
    name: String,
    device_type: u8,
    readonly: bool,
    auto_install: bool,
    dont_auto_switch_when_installed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginDescriptor {
    uuid: String,
    name: String,
    version: String,
    author: String,
    description: String,
    source_kind: String,
    root: String,
    enabled: bool,
    active: bool,
    process_state: String,
    compatibility: String,
    runtime_kind: Option<String>,
    last_error: Option<String>,
    sdk_version: u8,
    minimum_software_version: Option<String>,
    property_inspector_path: Option<String>,
    actions: Vec<PluginActionDescriptor>,
    profiles: Vec<PluginProfileDescriptor>,
    entry_path: String,
    node_version: Option<String>,
    secrets_path: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginHostStatus {
    protocol_version: String,
    stream_deck_compatibility_target: String,
    websocket_host: String,
    installed: usize,
    active: usize,
    enabled: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PluginProcessEvent {
    plugin_uuid: String,
    state: String,
    message: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ContextStore {
    settings: HashMap<String, Value>,
    resources: HashMap<String, HashMap<String, String>>,
    global_settings: HashMap<String, Value>,
    states: HashMap<String, u8>,
    metadata: HashMap<String, ContextMetadata>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContextMetadata {
    action: String,
    device: String,
    controller: String,
    position: usize,
    is_in_multi_action: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Registry {
    plugins: Vec<PluginDescriptor>,
}

struct RuntimeHandle {
    port: u16,
    stop: broadcast::Sender<()>,
    outbound: broadcast::Sender<String>,
}

struct HostState {
    registry: Registry,
    contexts: ContextStore,
    runtimes: HashMap<String, RuntimeHandle>,
    app: Option<AppHandle>,
}

#[derive(Clone)]
pub(crate) struct Service {
    state: Arc<Mutex<HostState>>,
}

impl Service {
    pub(crate) fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(HostState {
                registry: read_registry().unwrap_or_default(),
                contexts: read_context_store().unwrap_or_default(),
                runtimes: HashMap::new(),
                app: None,
            })),
        }
    }

    pub(crate) fn initialize(&self, app: AppHandle) -> Result<(), String> {
        let desired = {
            let mut state = self.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
            state.app = Some(app);
            state.registry.plugins.iter().filter(|plugin| plugin.enabled && plugin.active).map(|plugin| plugin.uuid.clone()).collect::<Vec<_>>()
        };
        for uuid in desired {
            let service = self.clone();
            tauri::async_runtime::spawn(async move {
                let _ = service.start_plugin(&uuid).await;
            });
        }
        Ok(())
    }

    fn app(&self) -> Result<AppHandle, String> {
        self.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?.app.clone().ok_or_else(|| "plugin host is not initialized".to_string())
    }

    fn plugin(&self, uuid: &str) -> Result<PluginDescriptor, String> {
        self.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?.registry.plugins.iter().find(|plugin| plugin.uuid == uuid).cloned().ok_or_else(|| format!("plugin {uuid} is not installed"))
    }

    fn save_locked(state: &HostState) -> Result<(), String> {
        write_registry(&state.registry)?;
        write_context_store(&state.contexts)
    }

    fn emit_catalog(&self) {
        if let Ok(state) = self.state.lock()
            && let Some(app) = state.app.as_ref()
        {
            let _ = app.emit(PLUGIN_CATALOG_EVENT, state.registry.plugins.clone());
        }
    }

    fn set_process_state(&self, uuid: &str, process_state: &str, error: Option<String>) {
        if let Ok(mut state) = self.state.lock() {
            if let Some(plugin) = state.registry.plugins.iter_mut().find(|plugin| plugin.uuid == uuid) {
                plugin.process_state = process_state.to_string();
                plugin.last_error = error.clone();
            }
            let _ = Self::save_locked(&state);
            if let Some(app) = state.app.as_ref() {
                let _ = app.emit(PLUGIN_PROCESS_EVENT, PluginProcessEvent {
                    plugin_uuid: uuid.to_string(),
                    state: process_state.to_string(),
                    message: error.unwrap_or_else(|| process_state.to_string()),
                });
            }
        }
    }

    async fn start_plugin(&self, uuid: &str) -> Result<(), String> {
        {
            let state = self.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
            if state.runtimes.contains_key(uuid) {
                return Ok(());
            }
        }
        let plugin = self.plugin(uuid)?;
        if !plugin.enabled {
            return Err(format!("plugin {} is disabled", plugin.name));
        }
        if plugin.compatibility == "drm-protected" {
            return Err("DRM-protected Marketplace plugin requires authorization from Elgato; OpenDeck will not bypass DRM".into());
        }
        if plugin.compatibility == "unsupported" {
            return Err(plugin.last_error.unwrap_or_else(|| "plugin runtime is unsupported".into()));
        }

        self.set_process_state(uuid, "starting", None);
        let listener = TcpListener::bind("127.0.0.1:0").await.map_err(|error| format!("plugin websocket bind: {error}"))?;
        let port = listener.local_addr().map_err(|error| error.to_string())?.port();
        let (outbound, _) = broadcast::channel::<String>(256);
        let (stop, _) = broadcast::channel::<()>(4);
        let app = self.app()?;
        let service = self.clone();
        let plugin_for_server = plugin.clone();
        let outbound_for_server = outbound.clone();
        let mut stop_server = stop.subscribe();
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop_server.recv() => break,
                    accepted = listener.accept() => {
                        let Ok((stream, _)) = accepted else { break; };
                        let app = app.clone();
                        let service = service.clone();
                        let plugin = plugin_for_server.clone();
                        let client_plugin_uuid = plugin.uuid.clone();
                        let tx = outbound_for_server.clone();
                        let rx = tx.subscribe();
                        tauri::async_runtime::spawn(async move {
                            if let Err(error) = client_loop(stream, plugin, service, app.clone(), tx, rx).await {
                                let _ = app.emit(PLUGIN_PROCESS_EVENT, PluginProcessEvent { plugin_uuid: client_plugin_uuid, state: "client-error".into(), message: error });
                            }
                        });
                    }
                }
            }
        });

        let info = registration_info(&plugin);
        let entry = PathBuf::from(&plugin.entry_path);
        let mut command = runtime_command(&plugin, &entry)?;
        command
            .current_dir(PathBuf::from(&plugin.root))
            .arg("-port").arg(port.to_string())
            .arg("-pluginUUID").arg(&plugin.uuid)
            .arg("-registerEvent").arg("registerPlugin")
            .arg("-info").arg(info.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut child = command.spawn().map_err(|error| format!("start {}: {error}", plugin.name))?;
        let plugin_uuid = plugin.uuid.clone();
        let plugin_name = plugin.name.clone();
        let service_for_wait = self.clone();
        let mut stop_process = stop.subscribe();
        tauri::async_runtime::spawn(async move {
            tokio::select! {
                _ = stop_process.recv() => {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }
                status = child.wait() => {
                    let message = match status {
                        Ok(status) if status.success() => format!("{plugin_name} stopped"),
                        Ok(status) => format!("{plugin_name} exited with {status}"),
                        Err(error) => format!("{plugin_name} wait failed: {error}"),
                    };
                    service_for_wait.runtime_exited(&plugin_uuid, message);
                }
            }
        });

        {
            let mut state = self.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
            state.runtimes.insert(plugin.uuid.clone(), RuntimeHandle { port, stop, outbound });
            if let Some(stored) = state.registry.plugins.iter_mut().find(|stored| stored.uuid == plugin.uuid) {
                stored.active = true;
                stored.process_state = "active".into();
                stored.last_error = None;
            }
            Self::save_locked(&state)?;
        }
        self.set_process_state(&plugin.uuid, "active", None);
        self.emit_catalog();
        Ok(())
    }

    fn runtime_exited(&self, uuid: &str, message: String) {
        if let Ok(mut state) = self.state.lock() {
            state.runtimes.remove(uuid);
            if let Some(plugin) = state.registry.plugins.iter_mut().find(|plugin| plugin.uuid == uuid) {
                if plugin.active {
                    plugin.process_state = "crashed".into();
                    plugin.last_error = Some(message.clone());
                } else {
                    plugin.process_state = "inactive".into();
                }
            }
            let _ = Self::save_locked(&state);
            if let Some(app) = state.app.as_ref() {
                let _ = app.emit(PLUGIN_PROCESS_EVENT, PluginProcessEvent { plugin_uuid: uuid.into(), state: "crashed".into(), message });
            }
        }
        self.emit_catalog();
    }

    async fn stop_plugin(&self, uuid: &str) -> Result<(), String> {
        let handle = {
            let mut state = self.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
            state.runtimes.remove(uuid)
        };
        if let Some(handle) = handle {
            let _ = handle.stop.send(());
        }
        {
            let mut state = self.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
            if let Some(plugin) = state.registry.plugins.iter_mut().find(|plugin| plugin.uuid == uuid) {
                plugin.active = false;
                plugin.process_state = "inactive".into();
                plugin.last_error = None;
            }
            Self::save_locked(&state)?;
        }
        self.emit_catalog();
        Ok(())
    }

    pub(crate) fn stop_all(&self) {
        let handles = if let Ok(mut state) = self.state.lock() {
            state.runtimes.drain().map(|(_, handle)| handle).collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        for handle in handles {
            let _ = handle.stop.send(());
        }
    }

    fn broadcast_to_active(&self, payload: &Value) -> Result<(), String> {
        let serialized = payload.to_string();
        let state = self.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
        for runtime in state.runtimes.values() {
            let _ = runtime.outbound.send(serialized.clone());
        }
        Ok(())
    }

    fn outbound(&self, uuid: &str) -> Result<broadcast::Sender<String>, String> {
        let state = self.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
        state.runtimes.get(uuid).map(|runtime| runtime.outbound.clone()).ok_or_else(|| format!("plugin {uuid} is not active"))
    }

    fn runtime_port(&self, uuid: &str) -> Result<u16, String> {
        let state = self.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
        state.runtimes.get(uuid).map(|runtime| runtime.port).ok_or_else(|| format!("plugin {uuid} is not active"))
    }
}

impl Default for Service {
    fn default() -> Self { Self::new() }
}


#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginDispatchRequest {
    plugin_uuid: String,
    action_uuid: String,
    context: String,
    device: String,
    control_kind: String,
    position: usize,
    interaction: Option<String>,
    hardware_event: Option<Value>,
    #[serde(default)]
    is_in_multi_action: bool,
    user_desired_state: Option<u8>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PluginFeedbackEvent {
    context: String,
    plugin_uuid: String,
    action_uuid: Option<String>,
    title: Option<String>,
    image_asset_id: Option<String>,
    image_path: Option<String>,
    image_data_url: Option<String>,
    state: Option<u8>,
    feedback: Option<Value>,
    feedback_layout: Option<String>,
    trigger_description: Option<Value>,
    signal: Option<String>,
}

fn config_root() -> Result<PathBuf, String> {
    if let Some(root) = std::env::var_os("XDG_CONFIG_HOME") { return Ok(PathBuf::from(root).join("opendeck-v2/plugins")); }
    Ok(home_dir()?.join(".config/opendeck-v2/plugins"))
}

fn data_root() -> Result<PathBuf, String> {
    if let Some(root) = std::env::var_os("XDG_DATA_HOME") { return Ok(PathBuf::from(root).join("opendeck-v2/plugins")); }
    Ok(home_dir()?.join(".local/share/opendeck-v2/plugins"))
}

fn cache_root() -> Result<PathBuf, String> {
    if let Some(root) = std::env::var_os("XDG_CACHE_HOME") { return Ok(PathBuf::from(root).join("opendeck-v2/plugin-feedback")); }
    Ok(home_dir()?.join(".cache/opendeck-v2/plugin-feedback"))
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| "HOME is not set".to_string())
}

fn ensure_private_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|error| format!("create {}: {error}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|error| format!("permissions {}: {error}", path.display()))?;
    Ok(())
}

fn registry_path() -> Result<PathBuf, String> { Ok(config_root()?.join("registry.json")) }
fn contexts_path() -> Result<PathBuf, String> { Ok(config_root()?.join("contexts.json")) }

fn read_json<T: for<'de> Deserialize<'de> + Default>(path: &Path) -> Result<T, String> {
    if !path.exists() { return Ok(T::default()); }
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {}: {error}", path.display()))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| "invalid plugin persistence path".to_string())?;
    ensure_private_dir(parent)?;
    let temp = parent.join(format!(".{}.{}.tmp", path.file_name().unwrap_or_default().to_string_lossy(), Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    fs::write(&temp, bytes).map_err(|error| format!("write {}: {error}", temp.display()))?;
    fs::set_permissions(&temp, fs::Permissions::from_mode(0o600)).map_err(|error| error.to_string())?;
    fs::rename(&temp, path).map_err(|error| format!("activate {}: {error}", path.display()))
}

fn read_registry() -> Result<Registry, String> { read_json(&registry_path()?) }
fn write_registry(value: &Registry) -> Result<(), String> { write_json(&registry_path()?, value) }
fn read_context_store() -> Result<ContextStore, String> { read_json(&contexts_path()?) }
fn write_context_store(value: &ContextStore) -> Result<(), String> { write_json(&contexts_path()?, value) }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ElgatoManifest {
    actions: Vec<ElgatoAction>,
    author: String,
    category: Option<String>,
    code_path: String,
    code_path_win: Option<String>,
    description: String,
    name: String,
    nodejs: Option<ElgatoNode>,
    profiles: Option<Vec<ElgatoProfile>>,
    property_inspector_path: Option<String>,
    #[serde(rename = "SDKVersion")]
    sdk_version: u8,
    software: ElgatoSoftware,
    #[serde(rename = "UUID")]
    uuid: String,
    version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ElgatoAction {
    controllers: Option<Vec<String>>,
    name: String,
    property_inspector_path: Option<String>,
    #[serde(default)]
    states: Vec<ElgatoState>,
    #[serde(rename = "UUID")]
    uuid: String,
    supported_in_multi_actions: Option<bool>,
    supported_in_key_logic_actions: Option<bool>,
    disable_automatic_states: Option<bool>,
    visible_in_actions_list: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ElgatoState {
    image: String,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ElgatoNode { version: String }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ElgatoSoftware { minimum_version: String }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ElgatoProfile {
    name: String,
    device_type: u8,
    readonly: Option<bool>,
    auto_install: Option<bool>,
    dont_auto_switch_when_installed: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenDeckManifest {
    uuid: String,
    name: String,
    version: String,
    author: String,
    #[serde(default)]
    description: String,
    entry: String,
    #[serde(default = "default_native_runtime")]
    runtime: String,
    #[serde(default)]
    property_inspector_path: Option<String>,
    actions: Vec<OpenDeckAction>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenDeckAction {
    uuid: String,
    name: String,
    #[serde(default = "default_controllers")]
    controllers: Vec<String>,
    property_inspector_path: Option<String>,
}

fn default_true() -> bool { true }
fn default_native_runtime() -> String { "native".into() }
fn default_controllers() -> Vec<String> { vec!["Keypad".into()] }

fn supported_kinds(controllers: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    if controllers.iter().any(|controller| controller == "Keypad" || controller == "Neo") { result.push("key".into()); }
    if controllers.iter().any(|controller| controller == "Encoder") { result.push("dial".into()); result.push("touch".into()); }
    result
}

fn descriptor_from_elgato(root: &Path, manifest: ElgatoManifest) -> PluginDescriptor {
    let generic = root.join(&manifest.code_path);
    let windows_relative = manifest.code_path_win.clone();
    let windows_entry = windows_relative.as_ref().map(|value| root.join(value));
    let use_windows_compat = !generic.exists() && windows_entry.as_ref().is_some_and(|path| path.exists());
    let entry_relative = if use_windows_compat {
        windows_relative.clone().unwrap_or_else(|| manifest.code_path.clone())
    } else {
        manifest.code_path.clone()
    };
    let entry = root.join(&entry_relative);
    let lower = entry_relative.to_ascii_lowercase();
    let runtime_kind = if manifest.nodejs.is_some() || lower.ends_with(".js") || lower.ends_with(".mjs") || lower.ends_with(".cjs") {
        Some("node".into())
    } else if lower.ends_with(".exe") {
        Some("wine".into())
    } else {
        Some("native".into())
    };
    let compatibility = match runtime_kind.as_deref() {
        Some("wine") if command_exists("wine") || command_exists("wine64") => "wine",
        Some("wine") => "unsupported",
        Some("node") if command_exists("node") => "node",
        Some("node") => "unsupported",
        Some("native") if entry.exists() => "native",
        _ => "unsupported",
    }.to_string();
    let runtime_error = match runtime_kind.as_deref() {
        Some("wine") if compatibility == "unsupported" => Some("Windows plugin requires Wine or Wine64".into()),
        Some("node") if compatibility == "unsupported" => Some("Plugin requires a local Node.js runtime".into()),
        Some("native") if !entry.exists() => Some(format!("Plugin entry point is missing: {}", entry.display())),
        _ => None,
    };
    let category = manifest.category.clone().unwrap_or_else(|| manifest.name.clone());
    let actions = manifest.actions.into_iter().map(|action| {
        let controllers = action.controllers.unwrap_or_else(|| vec!["Keypad".into()]);
        let kinds = supported_kinds(&controllers);
        PluginActionDescriptor {
            id: format!("plugin:{}:{}", manifest.uuid, action.uuid),
            plugin_uuid: manifest.uuid.clone(),
            uuid: action.uuid,
            name: action.name,
            category: category.clone(),
            controllers,
            supported_kinds: kinds.clone(),
            default_interaction: if kinds.iter().any(|kind| kind == "key" || kind == "dial") {
                "press".into()
            } else {
                "touch".into()
            },
            property_inspector_path: action.property_inspector_path.or_else(|| manifest.property_inspector_path.clone()),
            states: action.states.into_iter().map(|state| PluginStateDescriptor { name: state.name, image: Some(state.image) }).collect(),
            supported_in_multi_actions: action.supported_in_multi_actions.unwrap_or(true),
            supported_in_key_logic_actions: action.supported_in_key_logic_actions.unwrap_or(true),
            disable_automatic_states: action.disable_automatic_states.unwrap_or(false),
            visible_in_actions_list: action.visible_in_actions_list.unwrap_or(true),
        }
    }).collect();
    PluginDescriptor {
        uuid: manifest.uuid,
        name: manifest.name,
        version: manifest.version,
        author: manifest.author,
        description: manifest.description,
        source_kind: "elgato".into(),
        root: root.display().to_string(),
        enabled: true,
        active: false,
        process_state: "inactive".into(),
        compatibility,
        runtime_kind,
        last_error: runtime_error,
        sdk_version: manifest.sdk_version,
        minimum_software_version: Some(manifest.software.minimum_version),
        property_inspector_path: manifest.property_inspector_path,
        actions,
        profiles: manifest.profiles.unwrap_or_default().into_iter().map(|profile| PluginProfileDescriptor {
            name: profile.name,
            device_type: profile.device_type,
            readonly: profile.readonly.unwrap_or(false),
            auto_install: profile.auto_install.unwrap_or(true),
            dont_auto_switch_when_installed: profile.dont_auto_switch_when_installed.unwrap_or(false),
        }).collect(),
        entry_path: entry.display().to_string(),
        node_version: manifest.nodejs.map(|node| node.version),
        secrets_path: root.join("secrets.json").is_file().then(|| root.join("secrets.json").display().to_string()),
    }
}

fn descriptor_from_opendeck(root: &Path, manifest: OpenDeckManifest) -> PluginDescriptor {
    let entry = root.join(&manifest.entry);
    let runtime_kind = Some(manifest.runtime.clone());
    let compatibility = if manifest.runtime == "node" && command_exists("node") { "node" } else if manifest.runtime == "native" && entry.exists() { "native" } else { "unsupported" }.to_string();
    let actions = manifest.actions.into_iter().map(|action| {
        let kinds = supported_kinds(&action.controllers);
        PluginActionDescriptor {
            id: format!("plugin:{}:{}", manifest.uuid, action.uuid),
            plugin_uuid: manifest.uuid.clone(),
            uuid: action.uuid,
            name: action.name,
            category: manifest.name.clone(),
            controllers: action.controllers,
            supported_kinds: kinds.clone(),
            default_interaction: if kinds.iter().any(|kind| kind == "key") { "press".into() } else { "touch".into() },
            property_inspector_path: action.property_inspector_path.or_else(|| manifest.property_inspector_path.clone()),
            states: Vec::new(),
            supported_in_multi_actions: true,
            supported_in_key_logic_actions: true,
            disable_automatic_states: false,
            visible_in_actions_list: true,
        }
    }).collect();
    PluginDescriptor {
        uuid: manifest.uuid,
        name: manifest.name,
        version: manifest.version,
        author: manifest.author,
        description: manifest.description,
        source_kind: "opendeck".into(),
        root: root.display().to_string(),
        enabled: true,
        active: false,
        process_state: "inactive".into(),
        compatibility,
        runtime_kind,
        last_error: None,
        sdk_version: 3,
        minimum_software_version: Some(STREAM_DECK_COMPATIBILITY_TARGET.into()),
        property_inspector_path: manifest.property_inspector_path,
        actions,
        profiles: Vec::new(),
        entry_path: entry.display().to_string(),
        node_version: None,
        secrets_path: root.join("secrets.json").is_file().then(|| root.join("secrets.json").display().to_string()),
    }
}

fn command_exists(command: &str) -> bool {
    std::process::Command::new("sh").args(["-c", &format!("command -v {command} >/dev/null 2>&1")]).status().is_ok_and(|status| status.success())
}

fn runtime_command(plugin: &PluginDescriptor, entry: &Path) -> Result<Command, String> {
    match plugin.runtime_kind.as_deref() {
        Some("node") | Some("opendeck") if entry.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("js")) => {
            let mut command = Command::new("node");
            command.arg("--enable-source-maps").arg("--no-global-search-paths").arg(entry);
            Ok(command)
        }
        Some("wine") => {
            let wine = if command_exists("wine64") { "wine64" } else { "wine" };
            let mut command = Command::new(wine);
            command.arg(entry);
            Ok(command)
        }
        Some("native") | Some("opendeck") => Ok(Command::new(entry)),
        _ => Err(format!("no runnable runtime for {}", plugin.name)),
    }
}

fn registration_info(plugin: &PluginDescriptor) -> Value {
    json!({
        "application": {
            "font": "Inter",
            "language": "en",
            "platform": "windows",
            "platformVersion": "10.0",
            "version": STREAM_DECK_COMPATIBILITY_TARGET
        },
        "colors": {
            "buttonPressedBackgroundColor": "#30323a",
            "buttonPressedBorderColor": "#7c68ff",
            "buttonPressedTextColor": "#ffffff",
            "highlightColor": "#7c68ff"
        },
        "devicePixelRatio": 1,
        "devices": [{
            "id": DEVICE_ID,
            "name": "Stream Deck +",
            "size": {"columns": 4, "rows": 2},
            "type": DEVICE_TYPE_STREAM_DECK_PLUS
        }],
        "plugin": {"uuid": plugin.uuid, "version": plugin.version}
    })
}

fn device_info() -> Value {
    json!({"name":"Stream Deck +","size":{"columns":4,"rows":2},"type":DEVICE_TYPE_STREAM_DECK_PLUS})
}

async fn client_loop(
    stream: tokio::net::TcpStream,
    plugin: PluginDescriptor,
    service: Service,
    app: AppHandle,
    outbound: broadcast::Sender<String>,
    mut outbound_rx: broadcast::Receiver<String>,
) -> Result<(), String> {
    let websocket = accept_async(stream).await.map_err(|error| format!("plugin websocket accept: {error}"))?;
    let (mut sink, mut source) = websocket.split();
    loop {
        tokio::select! {
            incoming = source.next() => {
                let Some(incoming) = incoming else { break; };
                let incoming = incoming.map_err(|error| error.to_string())?;
                if let Message::Text(text) = incoming {
                    handle_client_message(&plugin, &service, &app, &outbound, text.as_str()).await?;
                }
            }
            outgoing = outbound_rx.recv() => {
                match outgoing {
                    Ok(text) => sink.send(Message::Text(text.into())).await.map_err(|error| error.to_string())?,
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
    Ok(())
}

async fn handle_client_message(plugin: &PluginDescriptor, service: &Service, app: &AppHandle, outbound: &broadcast::Sender<String>, text: &str) -> Result<(), String> {
    let message: Value = serde_json::from_str(text).map_err(|error| format!("invalid plugin JSON: {error}"))?;
    let event = message.get("event").and_then(Value::as_str).unwrap_or_default();
    if event == "registerPlugin" || event == "registerPropertyInspector" {
        let _ = outbound.send(json!({"event":"deviceDidConnect","device":DEVICE_ID,"deviceInfo":device_info()}).to_string());
        return Ok(());
    }
    let context = message.get("context").and_then(Value::as_str).unwrap_or_default().to_string();
    let action = message.get("action").and_then(Value::as_str).map(str::to_owned);
    let id = message.get("id").cloned();
    match event {
        "getSettings" => {
            let settings = {
                let state = service.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
                state.contexts.settings.get(&context).cloned().unwrap_or_else(|| json!({}))
            };
            let response_payload = context_response_payload(service, &context, settings, json!({}))?;
            let mut response = json!({"event":"didReceiveSettings","action":action,"context":context,"device":DEVICE_ID,"payload":response_payload});
            if let Some(id) = id { response["id"] = id; }
            let _ = outbound.send(response.to_string());
        }
        "setSettings" => {
            let payload = message.get("payload").cloned().unwrap_or_else(|| json!({}));
            {
                let mut state = service.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
                state.contexts.settings.insert(context.clone(), payload.clone());
                Service::save_locked(&state)?;
            }
            let response_payload = context_response_payload(service, &context, payload, json!({}))?;
            let _ = outbound.send(json!({"event":"didReceiveSettings","action":action,"context":context,"device":DEVICE_ID,"payload":response_payload}).to_string());
        }
        "getGlobalSettings" => {
            let settings = {
                let state = service.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
                state.contexts.global_settings.get(&plugin.uuid).cloned().unwrap_or_else(|| json!({}))
            };
            let mut response = json!({"event":"didReceiveGlobalSettings","payload":{"settings":settings}});
            if let Some(id) = id { response["id"] = id; }
            let _ = outbound.send(response.to_string());
        }
        "setGlobalSettings" => {
            let payload = message.get("payload").cloned().unwrap_or_else(|| json!({}));
            {
                let mut state = service.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
                state.contexts.global_settings.insert(plugin.uuid.clone(), payload.clone());
                Service::save_locked(&state)?;
            }
            let _ = outbound.send(json!({"event":"didReceiveGlobalSettings","payload":{"settings":payload}}).to_string());
        }
        "getResources" => {
            let resources = {
                let state = service.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
                state.contexts.resources.get(&context).cloned().unwrap_or_default()
            };
            let response_payload = context_response_payload(service, &context, json!({}), serde_json::to_value(resources).map_err(|error| error.to_string())?)?;
            let mut response = json!({"event":"didReceiveResources","action":action,"context":context,"device":DEVICE_ID,"payload":response_payload});
            if let Some(id) = id { response["id"] = id; }
            let _ = outbound.send(response.to_string());
        }
        "setResources" => {
            let resources: HashMap<String, String> = serde_json::from_value(message.get("payload").cloned().unwrap_or_else(|| json!({}))).unwrap_or_default();
            {
                let mut state = service.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
                state.contexts.resources.insert(context.clone(), resources.clone());
                Service::save_locked(&state)?;
            }
            let response_payload = context_response_payload(service, &context, json!({}), serde_json::to_value(resources).map_err(|error| error.to_string())?)?;
            let _ = outbound.send(json!({"event":"didReceiveResources","action":action,"context":context,"device":DEVICE_ID,"payload":response_payload}).to_string());
        }
        "getSecrets" => {
            let secrets = plugin.secrets_path.as_deref().and_then(|path| fs::read(path).ok()).and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok()).unwrap_or_else(|| json!({}));
            let _ = outbound.send(json!({"event":"didReceiveSecrets","payload":{"secrets":secrets}}).to_string());
        }
        "sendToPlugin" | "sendToPropertyInspector" => {
            let _ = outbound.send(message.to_string());
        }
        "setTitle" | "setImage" | "setState" | "setFeedback" | "setFeedbackLayout" | "setTriggerDescription" | "showOk" | "showAlert" => {
            if event == "setState"
                && let Some(value) = message.pointer("/payload/state").and_then(Value::as_u64)
            {
                set_context_state(service, &context, value as u8)?;
            }
            emit_feedback(plugin, app, &message)?;
        }
        "switchToProfile" => {
            let mut event = message.clone();
            event["pluginUuid"] = Value::String(plugin.uuid.clone());
            let _ = app.emit("opendeck://plugin-switch-profile", event);
        }
        "openUrl" => {
            if let Some(url) = message.pointer("/payload/url").and_then(Value::as_str) {
                let _ = std::process::Command::new("xdg-open").arg(url).spawn();
            }
        }
        "logMessage" => {
            if let Some(line) = message.pointer("/payload/message").and_then(Value::as_str) { write_plugin_log(plugin, line); }
        }
        _ => {
            let _ = app.emit("opendeck://plugin-unhandled-command", message.clone());
        }
    }
    Ok(())
}

fn write_plugin_log(plugin: &PluginDescriptor, line: &str) {
    let Ok(root) = config_root() else { return; };
    let path = root.join("logs").join(format!("{}.log", plugin.uuid));
    if let Some(parent) = path.parent() { let _ = ensure_private_dir(parent); }
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) { let _ = writeln!(file, "{line}"); }
}

fn emit_feedback(plugin: &PluginDescriptor, app: &AppHandle, message: &Value) -> Result<(), String> {
    let context = message.get("context").and_then(Value::as_str).unwrap_or_default().to_string();
    let event = message.get("event").and_then(Value::as_str).unwrap_or_default();
    let payload = message.get("payload").cloned().unwrap_or_else(|| json!({}));
    let mut feedback = PluginFeedbackEvent {
        context,
        plugin_uuid: plugin.uuid.clone(),
        action_uuid: message.get("action").and_then(Value::as_str).map(str::to_owned),
        title: None,
        image_asset_id: None,
        image_path: None,
        image_data_url: None,
        state: None,
        feedback: None,
        feedback_layout: None,
        trigger_description: None,
        signal: None,
    };
    match event {
        "setTitle" => feedback.title = payload.get("title").and_then(Value::as_str).map(str::to_owned),
        "setState" => feedback.state = payload.get("state").and_then(Value::as_u64).map(|state| state as u8),
        "setFeedback" => feedback.feedback = Some(payload),
        "setFeedbackLayout" => feedback.feedback_layout = payload.get("layout").and_then(Value::as_str).map(str::to_owned),
        "setTriggerDescription" => feedback.trigger_description = Some(payload),
        "showOk" => feedback.signal = Some("ok".into()),
        "showAlert" => feedback.signal = Some("alert".into()),
        "setImage" => {
            if let Some(image) = payload.get("image").and_then(Value::as_str)
                && let Some((asset_id, path, data_url)) = materialize_feedback_image(plugin, image)?
            {
                feedback.image_asset_id = Some(asset_id);
                feedback.image_path = Some(path);
                feedback.image_data_url = data_url;
            }
        }
        _ => {}
    }
    app.emit(PLUGIN_FEEDBACK_EVENT, feedback).map_err(|error| error.to_string())
}

fn materialize_feedback_image(plugin: &PluginDescriptor, image: &str) -> Result<Option<(String, String, Option<String>)>, String> {
    if image.is_empty() { return Ok(None); }
    let (bytes, extension, mime, data_url) = if let Some((header, encoded)) = image.split_once(',') {
        if header.starts_with("data:image/") && header.contains(";base64") {
            let bytes = STANDARD.decode(encoded).map_err(|error| format!("plugin image base64: {error}"))?;
            let (ext, mime) = if header.contains("svg+xml") { ("svg", "image/svg+xml") } else if header.contains("jpeg") || header.contains("jpg") { ("jpg", "image/jpeg") } else { ("png", "image/png") };
            (bytes, ext, mime, Some(image.to_string()))
        } else { return Ok(None); }
    } else if image.trim_start().starts_with("<svg") {
        let bytes = image.as_bytes().to_vec();
        let encoded = STANDARD.encode(&bytes);
        (bytes, "svg", "image/svg+xml", Some(format!("data:image/svg+xml;base64,{encoded}")))
    } else {
        let relative = Path::new(image);
        if relative.is_absolute() || relative.components().any(|component| matches!(component, std::path::Component::ParentDir)) {
            return Err("plugin image path must stay inside the plugin directory".into());
        }
        let root = PathBuf::from(&plugin.root).canonicalize().map_err(|error| format!("plugin root: {error}"))?;
        let mut candidate = root.join(relative);
        if !candidate.is_file() {
            for suffix in ["png", "jpg", "jpeg", "svg"] {
                let with_extension = root.join(format!("{image}.{suffix}"));
                if with_extension.is_file() { candidate = with_extension; break; }
            }
        }
        let candidate = candidate.canonicalize().map_err(|error| format!("plugin image {}: {error}", image))?;
        if !candidate.starts_with(&root) { return Err("plugin image path escaped the plugin directory".into()); }
        let bytes = fs::read(&candidate).map_err(|error| format!("read plugin image {}: {error}", candidate.display()))?;
        let extension = candidate.extension().and_then(|value| value.to_str()).unwrap_or("png").to_ascii_lowercase();
        let (extension, mime) = match extension.as_str() { "svg" => ("svg", "image/svg+xml"), "jpg" | "jpeg" => ("jpg", "image/jpeg"), _ => ("png", "image/png") };
        let encoded = STANDARD.encode(&bytes);
        (bytes, extension, mime, Some(format!("data:{mime};base64,{encoded}")))
    };
    let sha = format!("{:x}", Sha256::digest(&bytes));
    let root = cache_root()?.join(&plugin.uuid);
    ensure_private_dir(&root)?;
    let path = root.join(format!("{sha}.{extension}"));
    if !path.exists() { fs::write(&path, &bytes).map_err(|error| error.to_string())?; }
    let _ = mime;
    Ok(Some((format!("plugin-feedback-{sha}"), path.display().to_string(), data_url)))
}

fn expand_user_path(value: &str) -> Result<PathBuf, String> {
    if value == "~" { return home_dir(); }
    if let Some(rest) = value.strip_prefix("~/") { return Ok(home_dir()?.join(rest)); }
    Ok(PathBuf::from(value))
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    for entry in WalkDir::new(source).follow_links(false).into_iter().filter_map(Result::ok) {
        let relative = entry.path().strip_prefix(source).map_err(|error| error.to_string())?;
        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|error| error.to_string())?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() { fs::create_dir_all(parent).map_err(|error| error.to_string())?; }
            fs::copy(entry.path(), &target).map_err(|error| format!("copy {}: {error}", entry.path().display()))?;
        }
    }
    Ok(())
}

fn extract_package(source: &Path, destination: &Path) -> Result<(), String> {
    let file = fs::File::open(source).map_err(|error| format!("open {}: {error}", source.display()))?;
    let mut archive = ZipArchive::new(file).map_err(|error| format!("{} is not a readable Stream Deck package: {error}", source.display()))?;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|error| error.to_string())?;
        let Some(name) = file.enclosed_name() else { continue; };
        let target = destination.join(name);
        if file.is_dir() { fs::create_dir_all(&target).map_err(|error| error.to_string())?; continue; }
        if let Some(parent) = target.parent() { fs::create_dir_all(parent).map_err(|error| error.to_string())?; }
        let mut output = fs::File::create(&target).map_err(|error| error.to_string())?;
        std::io::copy(&mut file, &mut output).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn find_manifest_root(root: &Path) -> Result<(PathBuf, String), String> {
    for entry in WalkDir::new(root).max_depth(4).follow_links(false).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() { continue; }
        let name = entry.file_name().to_string_lossy();
        if name == "manifest.json" || name == "opendeck-plugin.json" {
            return Ok((entry.path().parent().unwrap_or(root).to_path_buf(), name.into_owned()));
        }
    }
    Err("plugin manifest was not found; Marketplace DRM-protected packages cannot be decrypted or bypassed by OpenDeck".into())
}

fn parse_descriptor(root: &Path, manifest_name: &str) -> Result<PluginDescriptor, String> {
    let bytes = fs::read(root.join(manifest_name)).map_err(|error| error.to_string())?;
    if manifest_name == "manifest.json" {
        let manifest: ElgatoManifest = serde_json::from_slice(&bytes).map_err(|error| format!("Stream Deck manifest: {error}"))?;
        Ok(descriptor_from_elgato(root, manifest))
    } else {
        let manifest: OpenDeckManifest = serde_json::from_slice(&bytes).map_err(|error| format!("OpenDeck plugin manifest: {error}"))?;
        Ok(descriptor_from_opendeck(root, manifest))
    }
}

fn install_plugin_from_path(path: &str) -> Result<PluginDescriptor, String> {
    let source = expand_user_path(path)?;
    if !source.exists() { return Err(format!("{} does not exist", source.display())); }
    let data = data_root()?;
    ensure_private_dir(&data)?;
    let staging = data.join(format!(".staging-{}", Uuid::new_v4()));
    fs::create_dir_all(&staging).map_err(|error| error.to_string())?;
    if source.is_dir() { copy_tree(&source, &staging)?; } else { extract_package(&source, &staging)?; }
    let (manifest_root, manifest_name) = find_manifest_root(&staging)?;
    let descriptor = parse_descriptor(&manifest_root, &manifest_name)?;
    let destination = data.join(format!("{}.sdPlugin", descriptor.uuid));
    if destination.exists() { fs::remove_dir_all(&destination).map_err(|error| error.to_string())?; }
    fs::rename(&manifest_root, &destination).or_else(|_| { copy_tree(&manifest_root, &destination)?; fs::remove_dir_all(&staging).ok(); Ok::<(), String>(()) })?;
    if staging.exists() { let _ = fs::remove_dir_all(&staging); }
    let mut installed = parse_descriptor(&destination, &manifest_name)?;
    installed.root = destination.display().to_string();
    installed.entry_path = destination.join(Path::new(&installed.entry_path).file_name().unwrap_or_default()).display().to_string();
    // Reparse entry from manifest relative to installed root to avoid a staging path.
    installed = parse_descriptor(&destination, &manifest_name)?;
    Ok(installed)
}

fn controller_for_request(request: &PluginDispatchRequest) -> &'static str {
    if request.control_kind == "dial" || request.control_kind == "touch" { "Encoder" } else { "Keypad" }
}

fn remember_context(service: &Service, request: &PluginDispatchRequest) -> Result<(), String> {
    let metadata = ContextMetadata {
        action: request.action_uuid.clone(),
        device: if request.device.is_empty() { DEVICE_ID.into() } else { request.device.clone() },
        controller: controller_for_request(request).into(),
        position: request.position,
        is_in_multi_action: request.is_in_multi_action,
    };
    let mut state = service.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
    state.contexts.metadata.insert(request.context.clone(), metadata);
    Service::save_locked(&state)
}

fn current_context_state(service: &Service, context: &str) -> Result<u8, String> {
    let state = service.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
    Ok(*state.contexts.states.get(context).unwrap_or(&0))
}

fn set_context_state(service: &Service, context: &str, value: u8) -> Result<(), String> {
    let mut state = service.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
    state.contexts.states.insert(context.to_string(), value);
    Service::save_locked(&state)
}

fn maybe_toggle_automatic_state(service: &Service, plugin: &PluginDescriptor, request: &PluginDispatchRequest, event: &str) -> Result<u8, String> {
    let action = plugin.actions.iter().find(|candidate| candidate.uuid == request.action_uuid);
    let current = current_context_state(service, &request.context)?;
    let should_toggle = event == "keyDown"
        && !request.is_in_multi_action
        && action.is_some_and(|action| action.states.len() == 2 && !action.disable_automatic_states);
    if should_toggle {
        let next = if current == 0 { 1 } else { 0 };
        set_context_state(service, &request.context, next)?;
        Ok(next)
    } else {
        Ok(current)
    }
}

fn context_response_payload(service: &Service, context: &str, settings: Value, resources: Value) -> Result<Value, String> {
    let state = service.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
    let metadata = state.contexts.metadata.get(context).cloned().unwrap_or_default();
    let action_state = *state.contexts.states.get(context).unwrap_or(&0);
    Ok(json!({
        "controller": if metadata.controller.is_empty() { "Keypad" } else { metadata.controller.as_str() },
        "coordinates": {"column": metadata.position % 4, "row": metadata.position / 4},
        "isInMultiAction": metadata.is_in_multi_action,
        "resources": resources,
        "settings": settings,
        "state": action_state,
    }))
}

fn registration_payload(plugin: &PluginDescriptor, request: &PluginDispatchRequest, event: &str, extra: Value, settings: Value, resources: Value) -> Value {
    let controller = controller_for_request(request);
    let mut payload = json!({
        "controller": controller,
        "coordinates": {"column": request.position % 4, "row": if request.control_kind == "key" { request.position / 4 } else { 0 }},
        "isInMultiAction": request.is_in_multi_action,
        "resources": resources,
        "settings": settings,
    });
    if let (Some(target), Some(extra)) = (payload.as_object_mut(), extra.as_object()) {
        for (key, value) in extra { target.insert(key.clone(), value.clone()); }
    }
    json!({"action":request.action_uuid,"context":request.context,"device":if request.device.is_empty(){DEVICE_ID}else{&request.device},"event":event,"payload":payload,"plugin":plugin.uuid})
}

fn context_values(service: &Service, context: &str) -> Result<(Value, Value), String> {
    let state = service.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
    let settings = state.contexts.settings.get(context).cloned().unwrap_or_else(|| json!({}));
    let resources = serde_json::to_value(state.contexts.resources.get(context).cloned().unwrap_or_default()).map_err(|error| error.to_string())?;
    Ok((settings, resources))
}

fn dispatch_messages(service: &Service, request: &PluginDispatchRequest) -> Result<Vec<Value>, String> {
    let plugin = service.plugin(&request.plugin_uuid)?;
    remember_context(service, request)?;
    let (settings, resources) = context_values(service, &request.context)?;
    let build = |event: &str, mut extra: Value, settings: Value, resources: Value| -> Result<Value, String> {
        let action_state = maybe_toggle_automatic_state(service, &plugin, request, event)?;
        if let Some(object) = extra.as_object_mut() {
            object.entry("state".to_string()).or_insert(json!(action_state));
            if request.is_in_multi_action
                && let Some(desired) = request.user_desired_state
            {
                object.entry("userDesiredState".to_string()).or_insert(json!(desired));
            }
        }
        Ok(registration_payload(&plugin, request, event, extra, settings, resources))
    };
    if let Some(raw) = request.hardware_event.as_ref() {
        let kind = raw.get("kind").and_then(Value::as_str).unwrap_or_default();
        let message = match kind {
            "keyDown" => build("keyDown", json!({}), settings, resources)?,
            "keyUp" => build("keyUp", json!({}), settings, resources)?,
            "dialDown" => build("dialDown", json!({}), settings, resources)?,
            "dialUp" => build("dialUp", json!({}), settings, resources)?,
            "dialRotate" => build("dialRotate", json!({"ticks":raw.get("ticks").cloned().unwrap_or(json!(0)),"pressed":raw.get("pressed").cloned().unwrap_or(json!(false))}), settings, resources)?,
            "touchTap" | "touchPress" => build("touchTap", json!({"hold":kind=="touchPress","tapPos":[raw.get("x").cloned().unwrap_or(json!(0)),raw.get("y").cloned().unwrap_or(json!(0))]}), settings, resources)?,
            _ => return Ok(Vec::new()),
        };
        return Ok(vec![message]);
    }
    let interaction = request.interaction.as_deref().unwrap_or("press");
    let messages = match (request.control_kind.as_str(), interaction) {
        ("key", "press" | "longPress") => vec![
            build("keyDown", json!({}), settings.clone(), resources.clone())?,
            build("keyUp", json!({}), settings, resources)?,
        ],
        ("dial", "press") => vec![build("dialDown", json!({}), settings.clone(), resources.clone())?, build("dialUp", json!({}), settings, resources)?],
        ("dial", "rotateLeft" | "pressRotateLeft") => vec![build("dialRotate", json!({"ticks":-1,"pressed":interaction.starts_with("press")}), settings, resources)?],
        ("dial", "rotateRight" | "pressRotateRight") => vec![build("dialRotate", json!({"ticks":1,"pressed":interaction.starts_with("press")}), settings, resources)?],
        ("touch", _) => vec![build("touchTap", json!({"hold":interaction=="longPress","tapPos":[0,0]}), settings, resources)?],
        _ => Vec::new(),
    };
    Ok(messages)
}

#[tauri::command]
pub(crate) fn plugin_list(service: State<'_, Service>) -> Result<Vec<PluginDescriptor>, String> {
    Ok(service.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?.registry.plugins.clone())
}

#[tauri::command]
pub(crate) fn plugin_host_status(service: State<'_, Service>) -> Result<PluginHostStatus, String> {
    let state = service.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
    Ok(PluginHostStatus {
        protocol_version: HOST_PROTOCOL_VERSION.into(),
        stream_deck_compatibility_target: STREAM_DECK_COMPATIBILITY_TARGET.into(),
        websocket_host: "127.0.0.1".into(),
        installed: state.registry.plugins.len(),
        active: state.runtimes.len(),
        enabled: state.registry.plugins.iter().filter(|plugin| plugin.enabled).count(),
    })
}

#[tauri::command]
pub(crate) async fn plugin_install(path: String, service: State<'_, Service>) -> Result<PluginDescriptor, String> {
    let descriptor = tauri::async_runtime::spawn_blocking(move || install_plugin_from_path(&path)).await.map_err(|error| error.to_string())??;
    {
        let mut state = service.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
        state.registry.plugins.retain(|plugin| plugin.uuid != descriptor.uuid);
        state.registry.plugins.push(descriptor.clone());
        state.registry.plugins.sort_by_key(|plugin| plugin.name.to_lowercase());
        Service::save_locked(&state)?;
    }
    service.emit_catalog();
    Ok(descriptor)
}

#[tauri::command]
pub(crate) async fn plugin_remove(plugin_uuid: String, service: State<'_, Service>) -> Result<(), String> {
    let cloned = service.inner().clone();
    let _ = cloned.stop_plugin(&plugin_uuid).await;
    let root = {
        let mut state = cloned.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
        let root = state.registry.plugins.iter().find(|plugin| plugin.uuid == plugin_uuid).map(|plugin| plugin.root.clone());
        state.registry.plugins.retain(|plugin| plugin.uuid != plugin_uuid);
        Service::save_locked(&state)?;
        root
    };
    if let Some(root) = root { let _ = fs::remove_dir_all(root); }
    cloned.emit_catalog();
    Ok(())
}

#[tauri::command]
pub(crate) async fn plugin_set_enabled(plugin_uuid: String, enabled: bool, service: State<'_, Service>) -> Result<(), String> {
    let cloned = service.inner().clone();
    if !enabled { let _ = cloned.stop_plugin(&plugin_uuid).await; }
    {
        let mut state = cloned.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
        let plugin = state.registry.plugins.iter_mut().find(|plugin| plugin.uuid == plugin_uuid).ok_or_else(|| format!("plugin {plugin_uuid} not found"))?;
        plugin.enabled = enabled;
        if !enabled { plugin.active = false; plugin.process_state = "inactive".into(); }
        Service::save_locked(&state)?;
    }
    cloned.emit_catalog();
    Ok(())
}

#[tauri::command]
pub(crate) async fn plugin_set_active(plugin_uuid: String, active: bool, service: State<'_, Service>) -> Result<(), String> {
    let cloned = service.inner().clone();
    if active {
        {
            let mut state = cloned.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
            let plugin = state.registry.plugins.iter_mut().find(|plugin| plugin.uuid == plugin_uuid).ok_or_else(|| format!("plugin {plugin_uuid} not found"))?;
            plugin.enabled = true;
            Service::save_locked(&state)?;
        }
        cloned.start_plugin(&plugin_uuid).await
    } else {
        cloned.stop_plugin(&plugin_uuid).await
    }
}

#[tauri::command]
pub(crate) async fn plugin_restart(plugin_uuid: String, service: State<'_, Service>) -> Result<(), String> {
    let cloned = service.inner().clone();
    let _ = cloned.stop_plugin(&plugin_uuid).await;
    {
        let mut state = cloned.state.lock().map_err(|_| "plugin host state lock poisoned".to_string())?;
        if let Some(plugin) = state.registry.plugins.iter_mut().find(|plugin| plugin.uuid == plugin_uuid) { plugin.active = true; }
        Service::save_locked(&state)?;
    }
    cloned.start_plugin(&plugin_uuid).await
}

#[tauri::command]
pub(crate) async fn plugin_dispatch_action(request: PluginDispatchRequest, service: State<'_, Service>) -> Result<(), String> {
    let cloned = service.inner().clone();
    if cloned.outbound(&request.plugin_uuid).is_err() { cloned.start_plugin(&request.plugin_uuid).await?; }
    let outbound = cloned.outbound(&request.plugin_uuid)?;
    for message in dispatch_messages(&cloned, &request)? { let _ = outbound.send(message.to_string()); }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginLifecycleRequest {
    plugin_uuid: String,
    action_uuid: String,
    context: String,
    device: String,
    control_kind: String,
    position: usize,
    event: String,
    #[serde(default)]
    is_in_multi_action: bool,
}

#[tauri::command]
pub(crate) async fn plugin_dispatch_lifecycle(request: PluginLifecycleRequest, service: State<'_, Service>) -> Result<(), String> {
    let cloned = service.inner().clone();
    if cloned.outbound(&request.plugin_uuid).is_err() { cloned.start_plugin(&request.plugin_uuid).await?; }
    let action_request = PluginDispatchRequest {
        plugin_uuid: request.plugin_uuid.clone(),
        action_uuid: request.action_uuid,
        context: request.context,
        device: request.device,
        control_kind: request.control_kind,
        position: request.position,
        interaction: None,
        hardware_event: None,
        is_in_multi_action: request.is_in_multi_action,
        user_desired_state: None,
    };
    let plugin = cloned.plugin(&request.plugin_uuid)?;
    remember_context(&cloned, &action_request)?;
    let (settings, resources) = context_values(&cloned, &action_request.context)?;
    let action_state = current_context_state(&cloned, &action_request.context)?;
    let payload = registration_payload(&plugin, &action_request, &request.event, json!({"state": action_state}), settings, resources);
    let _ = cloned.outbound(&request.plugin_uuid)?.send(payload.to_string());
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PluginHostEventRequest {
    event: String,
    #[serde(default)]
    payload: Value,
    device: Option<String>,
    application: Option<String>,
}

#[tauri::command]
pub(crate) fn plugin_dispatch_host_event(request: PluginHostEventRequest, service: State<'_, Service>) -> Result<(), String> {
    let mut message = json!({"event": request.event, "payload": request.payload});
    if let Some(device) = request.device { message["device"] = Value::String(device); }
    if let Some(application) = request.application { message["application"] = Value::String(application); }
    service.broadcast_to_active(&message)
}

#[tauri::command]
pub(crate) fn plugin_import_bundled_profile(plugin_uuid: String, profile_name: String, service: State<'_, Service>) -> Result<editor::Profile, String> {
    let plugin = service.plugin(&plugin_uuid)?;
    let profile = plugin.profiles.iter().find(|candidate| candidate.name == profile_name).ok_or_else(|| format!("bundled profile {profile_name} is not declared by {plugin_uuid}"))?;
    if profile.device_type != DEVICE_TYPE_STREAM_DECK_PLUS {
        return Err(format!("bundled profile targets Stream Deck device type {}, not Stream Deck + ({})", profile.device_type, DEVICE_TYPE_STREAM_DECK_PLUS));
    }
    let relative = format!("{}.streamDeckProfile", profile.name);
    let path = PathBuf::from(&plugin.root).join(relative);
    if !path.is_file() { return Err(format!("bundled profile is missing: {}", path.display())); }
    let mut imported = editor::import_streamdeck_profile(&path, Some(&plugin.uuid))?;
    imported.plugin_owner_uuid = Some(plugin.uuid.clone());
    imported.plugin_readonly = profile.readonly;
    Ok(imported)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PropertyInspectorRequest {
    plugin_uuid: String,
    action_uuid: String,
    context: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PropertyInspectorSession {
    plugin_uuid: String,
    action_uuid: String,
    context: String,
    url: String,
}

#[tauri::command]
pub(crate) async fn plugin_property_inspector_session(request: PropertyInspectorRequest, service: State<'_, Service>) -> Result<PropertyInspectorSession, String> {
    let cloned = service.inner().clone();
    if cloned.outbound(&request.plugin_uuid).is_err() { cloned.start_plugin(&request.plugin_uuid).await?; }
    let plugin = cloned.plugin(&request.plugin_uuid)?;
    let action = plugin.actions.iter().find(|action| action.uuid == request.action_uuid).ok_or_else(|| "plugin action not found".to_string())?;
    let relative = action.property_inspector_path.as_deref().or(plugin.property_inspector_path.as_deref()).ok_or_else(|| "plugin action has no Property Inspector".to_string())?;
    let source = PathBuf::from(&plugin.root).join(relative);
    let pi_root = cache_root()?.join("property-inspectors").join(&plugin.uuid).join("root");
    if pi_root.exists() { fs::remove_dir_all(&pi_root).map_err(|error| error.to_string())?; }
    copy_tree(Path::new(&plugin.root), &pi_root)?;
    let relative_source = source.strip_prefix(Path::new(&plugin.root)).map_err(|error| error.to_string())?;
    let output = pi_root.join(relative_source);
    let mut html = fs::read_to_string(&output).map_err(|error| format!("read Property Inspector {}: {error}", output.display()))?;
    let port = cloned.runtime_port(&request.plugin_uuid)?;
    let (pi_settings, pi_resources) = context_values(&cloned, &request.context)?;
    let pi_payload = context_response_payload(&cloned, &request.context, pi_settings, pi_resources)?;
    let action_info = json!({"action":request.action_uuid,"context":request.context,"device":DEVICE_ID,"payload":pi_payload});
    let bootstrap = format!(r#"<script>(()=>{{const connect=()=>{{if(typeof window.connectElgatoStreamDeckSocket==='function'){{window.connectElgatoStreamDeckSocket({port}, {uuid:?}, 'registerPropertyInspector', {info:?}, {action_info:?});}}else{{setTimeout(connect,25);}}}};connect();}})();</script>"#, uuid=plugin.uuid, info=registration_info(&plugin).to_string(), action_info=action_info.to_string());
    if let Some(index) = html.rfind("</body>") { html.insert_str(index, &bootstrap); } else { html.push_str(&bootstrap); }
    fs::write(&output, html).map_err(|error| error.to_string())?;
    Ok(PropertyInspectorSession { plugin_uuid: request.plugin_uuid, action_uuid: request.action_uuid, context: request.context, url: output.display().to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_stream_deck_plus_controllers_to_open_deck_controls() {
        assert_eq!(supported_kinds(&["Keypad".into()]), vec!["key"]);
        assert_eq!(supported_kinds(&["Encoder".into()]), vec!["dial", "touch"]);
    }

    #[test]
    fn registration_info_reports_stream_deck_plus() {
        let plugin = PluginDescriptor {
            uuid: "com.test.plugin".into(), name: "Test".into(), version: "1.0.0.0".into(), author: "Test".into(), description: String::new(), source_kind: "elgato".into(), root: "/tmp/test".into(), enabled: true, active: false, process_state: "inactive".into(), compatibility: "node".into(), runtime_kind: Some("node".into()), last_error: None, sdk_version: 3, minimum_software_version: Some("7.6".into()), property_inspector_path: None, actions: vec![], profiles: vec![], entry_path: "/tmp/test/index.js".into(), node_version: Some("20".into()), secrets_path: None,
        };
        let info = registration_info(&plugin);
        assert_eq!(info["devices"][0]["type"], DEVICE_TYPE_STREAM_DECK_PLUS);
        assert_eq!(info["application"]["version"], STREAM_DECK_COMPATIBILITY_TARGET);
    }
}
