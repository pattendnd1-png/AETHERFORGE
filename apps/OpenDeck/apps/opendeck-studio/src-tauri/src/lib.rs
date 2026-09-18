mod assets;
mod editor;
mod plugin_host;
pub mod qualification;
mod startup;
mod streamdeck;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};
use tauri::Manager;
use tokio::net::TcpStream;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use uuid::Uuid;
use walkdir::WalkDir;

const DEFAULT_TWITCH_CLIENT_ID: &str = "2cbsji4iqqroym707o2bx1832h1d0z";
const TWITCH_SCOPES: &str = "user:read:chat";

type ObsSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ObsConfig {
    host: String,
    port: u16,
    password: String,
}

impl Default for ObsConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 4455,
            password: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TwitchDeviceCode {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TwitchIdentity {
    login: String,
    user_id: String,
    expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TwitchCredentials {
    access_token: String,
    refresh_token: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TwitchTokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TwitchValidateResponse {
    login: Option<String>,
    user_id: Option<String>,
    expires_in: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
struct MarketplaceItem {
    name: String,
    path: String,
    kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActiveApplicationContext {
    app_id: Option<String>,
    provider: String,
}

fn query_active_application(tool: &str) -> Option<String> {
    let output = Command::new(tool)
        .args(["getactivewindow", "getwindowclassname"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!value.is_empty()).then_some(value)
}

#[tauri::command]
fn active_application_context() -> ActiveApplicationContext {
    if let Ok(value) = std::env::var("OPENDECK_ACTIVE_APP") {
        let value = value.trim();
        if !value.is_empty() {
            return ActiveApplicationContext {
                app_id: Some(value.to_string()),
                provider: "environment".into(),
            };
        }
    }
    for tool in ["kdotool", "xdotool"] {
        if let Some(app_id) = query_active_application(tool) {
            return ActiveApplicationContext {
                app_id: Some(app_id),
                provider: tool.into(),
            };
        }
    }
    ActiveApplicationContext {
        app_id: None,
        provider: "unavailable".into(),
    }
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn config_dir() -> Result<PathBuf, String> {
    Ok(home_dir()?.join(".config/opendeck-v2"))
}

fn ensure_private_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("create {}: {e}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|e| format!("permissions {}: {e}", path.display()))?;
    Ok(())
}

fn write_private_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        ensure_private_dir(parent)?;
    }
    let data = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    fs::write(path, data).map_err(|e| format!("write {}: {e}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|e| format!("permissions {}: {e}", path.display()))?;
    Ok(())
}

fn obs_config_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("obs.json"))
}

fn load_obs_config() -> Result<ObsConfig, String> {
    let path = obs_config_path()?;
    if !path.exists() {
        return Ok(ObsConfig::default());
    }
    let data = fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_slice(&data).map_err(|e| format!("parse {}: {e}", path.display()))
}

#[tauri::command]
fn obs_save_config(config: ObsConfig) -> Result<(), String> {
    write_private_json(&obs_config_path()?, &config)
}

fn obs_authentication(password: &str, salt: &str, challenge: &str) -> String {
    let secret_hash = Sha256::digest(format!("{password}{salt}").as_bytes());
    let secret = STANDARD.encode(secret_hash);
    let auth_hash = Sha256::digest(format!("{secret}{challenge}").as_bytes());
    STANDARD.encode(auth_hash)
}

async fn next_obs_json(ws: &mut ObsSocket) -> Result<Value, String> {
    loop {
        let item = timeout(Duration::from_secs(8), ws.next())
            .await
            .map_err(|_| "OBS WebSocket timed out".to_string())?
            .ok_or_else(|| "OBS WebSocket closed".to_string())?
            .map_err(|e| format!("OBS WebSocket read: {e}"))?;
        match item {
            Message::Text(text) => {
                return serde_json::from_str(text.as_ref()).map_err(|e| format!("OBS JSON: {e}"));
            }
            Message::Binary(data) => {
                return serde_json::from_slice(&data).map_err(|e| format!("OBS JSON: {e}"));
            }
            Message::Close(frame) => return Err(format!("OBS WebSocket closed: {frame:?}")),
            _ => {}
        }
    }
}

async fn obs_rpc(request_type: &str, request_data: Value) -> Result<Value, String> {
    let config = load_obs_config()?;
    let url = format!("ws://{}:{}", config.host, config.port);
    let (mut ws, _) = timeout(Duration::from_secs(8), connect_async(&url))
        .await
        .map_err(|_| format!("Timed out connecting to OBS at {url}"))?
        .map_err(|e| format!("Connect to OBS at {url}: {e}"))?;

    let hello = next_obs_json(&mut ws).await?;
    if hello.get("op").and_then(Value::as_i64) != Some(0) {
        return Err("OBS did not send Hello".into());
    }
    let hello_data = hello.get("d").cloned().unwrap_or(Value::Null);
    let rpc_version = hello_data
        .get("rpcVersion")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .min(1);
    let mut identify_data = json!({ "rpcVersion": rpc_version, "eventSubscriptions": 0 });
    if let Some(auth) = hello_data.get("authentication") {
        let challenge = auth
            .get("challenge")
            .and_then(Value::as_str)
            .ok_or("OBS challenge missing")?;
        let salt = auth
            .get("salt")
            .and_then(Value::as_str)
            .ok_or("OBS salt missing")?;
        let response = obs_authentication(&config.password, salt, challenge);
        identify_data["authentication"] = Value::String(response);
    }
    ws.send(Message::Text(
        json!({"op":1,"d":identify_data}).to_string().into(),
    ))
    .await
    .map_err(|e| format!("OBS identify send: {e}"))?;

    let identified = next_obs_json(&mut ws).await?;
    if identified.get("op").and_then(Value::as_i64) != Some(2) {
        return Err(format!("OBS identification failed: {identified}"));
    }

    let request_id = Uuid::new_v4().to_string();
    ws.send(Message::Text(
        json!({
            "op": 6,
            "d": {
                "requestType": request_type,
                "requestId": request_id,
                "requestData": request_data
            }
        })
        .to_string()
        .into(),
    ))
    .await
    .map_err(|e| format!("OBS request send: {e}"))?;

    loop {
        let response = next_obs_json(&mut ws).await?;
        if response.get("op").and_then(Value::as_i64) != Some(7) {
            continue;
        }
        let data = response.get("d").cloned().unwrap_or(Value::Null);
        if data.get("requestId").and_then(Value::as_str) != Some(request_id.as_str()) {
            continue;
        }
        let status = data.get("requestStatus").cloned().unwrap_or(Value::Null);
        if status.get("result").and_then(Value::as_bool) != Some(true) {
            let comment = status
                .get("comment")
                .and_then(Value::as_str)
                .unwrap_or("OBS request failed");
            let code = status.get("code").and_then(Value::as_i64).unwrap_or(0);
            return Err(format!("{comment} (OBS code {code})"));
        }
        return Ok(data
            .get("responseData")
            .cloned()
            .unwrap_or_else(|| json!({})));
    }
}

#[tauri::command]
async fn obs_status() -> Result<String, String> {
    let data = obs_rpc("GetVersion", json!({})).await?;
    let obs = data
        .get("obsVersion")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let websocket = data
        .get("obsWebSocketVersion")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    Ok(format!("OBS {obs} / WebSocket {websocket}"))
}

#[tauri::command]
async fn obs_scene_names() -> Result<Vec<String>, String> {
    let data = obs_rpc("GetSceneList", json!({})).await?;
    let mut names = Vec::new();
    if let Some(scenes) = data.get("scenes").and_then(Value::as_array) {
        for scene in scenes {
            if let Some(name) = scene.get("sceneName").and_then(Value::as_str) {
                names.push(name.to_string());
            }
        }
    }
    Ok(names)
}

#[tauri::command]
async fn obs_set_scene(scene_name: String) -> Result<(), String> {
    obs_rpc("SetCurrentProgramScene", json!({"sceneName": scene_name})).await?;
    Ok(())
}

#[tauri::command]
async fn obs_toggle_stream() -> Result<(), String> {
    obs_rpc("ToggleStream", json!({})).await?;
    Ok(())
}

#[tauri::command]
async fn obs_toggle_record() -> Result<(), String> {
    obs_rpc("ToggleRecord", json!({})).await?;
    Ok(())
}

#[tauri::command]
async fn obs_toggle_mute(input_name: String) -> Result<(), String> {
    obs_rpc("ToggleInputMute", json!({"inputName": input_name})).await?;
    Ok(())
}

fn twitch_client_id() -> String {
    std::env::var("OPENDECK_TWITCH_CLIENT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_TWITCH_CLIENT_ID.to_string())
}

fn twitch_credentials_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("twitch.json"))
}

fn load_twitch_credentials() -> Result<Option<TwitchCredentials>, String> {
    let path = twitch_credentials_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let credentials =
        serde_json::from_slice(&data).map_err(|e| format!("parse {}: {e}", path.display()))?;
    Ok(Some(credentials))
}

fn save_twitch_credentials(credentials: &TwitchCredentials) -> Result<(), String> {
    write_private_json(&twitch_credentials_path()?, credentials)
}

async fn validate_twitch_token(token: &str) -> Result<Option<TwitchIdentity>, String> {
    let response = reqwest::Client::new()
        .get("https://id.twitch.tv/oauth2/validate")
        .header("Authorization", format!("OAuth {token}"))
        .send()
        .await
        .map_err(|e| format!("Twitch validate: {e}"))?;
    if response.status().as_u16() == 401 {
        return Ok(None);
    }
    if !response.status().is_success() {
        return Err(format!("Twitch validate returned {}", response.status()));
    }
    let value: TwitchValidateResponse = response
        .json()
        .await
        .map_err(|e| format!("Twitch validate JSON: {e}"))?;
    match (value.login, value.user_id) {
        (Some(login), Some(user_id)) => Ok(Some(TwitchIdentity {
            login,
            user_id,
            expires_in: value.expires_in.unwrap_or(0),
        })),
        _ => Ok(None),
    }
}

async fn refresh_twitch(
    credentials: &TwitchCredentials,
) -> Result<Option<TwitchCredentials>, String> {
    if credentials.refresh_token.is_empty() {
        return Ok(None);
    }
    let client_id = twitch_client_id();
    let response = reqwest::Client::new()
        .post("https://id.twitch.tv/oauth2/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", credentials.refresh_token.as_str()),
        ])
        .send()
        .await
        .map_err(|e| format!("Twitch refresh: {e}"))?;
    if !response.status().is_success() {
        return Ok(None);
    }
    let token: TwitchTokenResponse = response
        .json()
        .await
        .map_err(|e| format!("Twitch refresh JSON: {e}"))?;
    Ok(Some(TwitchCredentials {
        access_token: token.access_token,
        refresh_token: if token.refresh_token.is_empty() {
            credentials.refresh_token.clone()
        } else {
            token.refresh_token
        },
    }))
}

#[tauri::command]
async fn twitch_status() -> Result<Option<TwitchIdentity>, String> {
    let Some(mut credentials) = load_twitch_credentials()? else {
        return Ok(None);
    };
    if let Some(identity) = validate_twitch_token(&credentials.access_token).await? {
        return Ok(Some(identity));
    }
    let Some(refreshed) = refresh_twitch(&credentials).await? else {
        return Ok(None);
    };
    save_twitch_credentials(&refreshed)?;
    credentials = refreshed;
    validate_twitch_token(&credentials.access_token).await
}

#[tauri::command]
async fn twitch_begin_auth() -> Result<TwitchDeviceCode, String> {
    let client_id = twitch_client_id();
    let response = reqwest::Client::new()
        .post("https://id.twitch.tv/oauth2/device")
        .form(&[("client_id", client_id.as_str()), ("scopes", TWITCH_SCOPES)])
        .send()
        .await
        .map_err(|e| format!("Start Twitch sign-in: {e}"))?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Twitch device authorization returned {status}: {body}"
        ));
    }
    response
        .json()
        .await
        .map_err(|e| format!("Twitch device authorization JSON: {e}"))
}

#[tauri::command]
async fn twitch_poll_auth(device_code: String) -> Result<Option<TwitchIdentity>, String> {
    let client_id = twitch_client_id();
    let response = reqwest::Client::new()
        .post("https://id.twitch.tv/oauth2/token")
        .form(&[
            ("client_id", client_id.as_str()),
            ("scopes", TWITCH_SCOPES),
            ("device_code", device_code.as_str()),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await
        .map_err(|e| format!("Twitch sign-in poll: {e}"))?;
    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        if body.contains("authorization_pending") {
            return Ok(None);
        }
        return Err(format!("Twitch sign-in failed: {body}"));
    }
    let token: TwitchTokenResponse = response
        .json()
        .await
        .map_err(|e| format!("Twitch token JSON: {e}"))?;
    let credentials = TwitchCredentials {
        access_token: token.access_token,
        refresh_token: token.refresh_token,
    };
    save_twitch_credentials(&credentials)?;
    validate_twitch_token(&credentials.access_token).await
}

#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    let allowed = url.starts_with("https://www.twitch.tv/activate")
        || url == "https://marketplace.elgato.com"
        || url == "https://marketplace.elgato.com/";
    if !allowed {
        return Err("External URL is not on the OpenDeck allowlist".into());
    }
    for (program, args) in [
        ("xdg-open", vec![url.as_str()]),
        ("gio", vec!["open", url.as_str()]),
    ] {
        if matches!(
            Command::new(program).args(args).status(),
            Ok(status) if status.success()
        ) {
            return Ok(());
        }
    }
    Err("Could not open the default browser".into())
}

fn classify_marketplace_path(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match ext.as_str() {
        "streamdeckiconpack" => Some("icon_pack"),
        "streamdeckplugin" => Some("plugin"),
        "streamdeckprofile" => Some("profile"),
        "streamdeckaction" => Some("action"),
        "streamdeckprofilesbackup" => Some("profiles_backup"),
        "streamdeckaudio" => Some("audio"),
        _ => None,
    }
}

#[tauri::command]
fn scan_marketplace_downloads() -> Result<Vec<MarketplaceItem>, String> {
    let home = home_dir()?;
    let roots = [
        home.join("Downloads"),
        home.join(".local/share/opendeck/icon-packs"),
    ];
    let mut items = Vec::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(&root)
            .max_depth(4)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if let Some(kind) = classify_marketplace_path(path) {
                items.push(MarketplaceItem {
                    name: path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    path: path.display().to_string(),
                    kind: kind.into(),
                });
            }
        }
    }
    items.sort_by(|a, b| a.path.cmp(&b.path));
    items.dedup_by(|a, b| a.path == b.path);
    Ok(items)
}

pub fn run() {
    tauri::Builder::default()
        .manage(streamdeck::Service::new())
        .manage(plugin_host::Service::new())
        .setup(|app| {
            qualification::mark_tauri_setup();
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title("OpenDeck");
            }
            startup::prepare_main_window(app).map_err(std::io::Error::other)?;
            app.state::<streamdeck::Service>()
                .start(app.handle().clone())
                .map_err(std::io::Error::other)?;
            app.state::<plugin_host::Service>()
                .initialize(app.handle().clone())
                .map_err(std::io::Error::other)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                window.app_handle().state::<streamdeck::Service>().stop();
                window
                    .app_handle()
                    .state::<plugin_host::Service>()
                    .stop_all();
            }
        })
        .invoke_handler(tauri::generate_handler![
            obs_save_config,
            obs_status,
            obs_scene_names,
            obs_set_scene,
            obs_toggle_stream,
            obs_toggle_record,
            obs_toggle_mute,
            twitch_status,
            twitch_begin_auth,
            twitch_poll_auth,
            open_external,
            scan_marketplace_downloads,
            active_application_context,
            plugin_host::plugin_list,
            plugin_host::plugin_host_status,
            plugin_host::plugin_install,
            plugin_host::plugin_remove,
            plugin_host::plugin_set_enabled,
            plugin_host::plugin_set_active,
            plugin_host::plugin_restart,
            plugin_host::plugin_dispatch_action,
            plugin_host::plugin_dispatch_lifecycle,
            plugin_host::plugin_dispatch_host_event,
            plugin_host::plugin_import_bundled_profile,
            plugin_host::plugin_property_inspector_session,
            startup::startup_visible_ack,
            qualification::qualification_context,
            qualification::qualification_focus_window,
            qualification::qualification_record_ui_metrics,
            editor::editor_load_workspace,
            editor::editor_save_workspace,
            editor::editor_export_profile,
            editor::editor_import_profile,
            assets::editor_import_asset,
            assets::editor_list_assets,
            assets::editor_asset_data_urls,
            streamdeck::streamdeck_status,
            streamdeck::streamdeck_sync_workspace,
            streamdeck::streamdeck_set_brightness,
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenDeck");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn obs_auth_is_stable() {
        let auth = obs_authentication(
            "supersecretpassword",
            "lM1GncleQOaCu9lT1yeUZhFYnqhsLLP1G5lAGo3ixaI=",
            "+IxH4CnCiqpX1rM9scsNynZzbOe4KhDeYcTNS3PDaeY=",
        );
        assert!(!auth.is_empty());
        assert_ne!(auth, "supersecretpassword");
    }

    #[test]
    fn marketplace_classifier_is_explicit() {
        assert_eq!(
            classify_marketplace_path(Path::new("x.streamDeckIconPack")),
            Some("icon_pack")
        );
        assert_eq!(
            classify_marketplace_path(Path::new("x.streamDeckPlugin")),
            Some("plugin")
        );
        assert_eq!(classify_marketplace_path(Path::new("x.zip")), None);
    }
}
