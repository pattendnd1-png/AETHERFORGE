use crate::{AetherStreamServiceBridge, AetherStreamServiceState, StudioControlIntent};
use aether_capture::{
    CaptureRegion, CaptureSafetyPolicy, clip_region_to_canvas, is_recursive_capture_target,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fmt;
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket, connect};

const DEFAULT_OBS_WEBSOCKET_URL: &str = "ws://127.0.0.1:4455";
const OBS_RPC_VERSION: u64 = 1;

type ObsSocket = WebSocket<MaybeTlsStream<TcpStream>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObsWebSocketConfig {
    pub url: String,
    pub password: Option<String>,
}

impl Default for ObsWebSocketConfig {
    fn default() -> Self {
        Self {
            url: DEFAULT_OBS_WEBSOCKET_URL.to_owned(),
            password: None,
        }
    }
}

impl ObsWebSocketConfig {
    #[must_use]
    pub fn from_env() -> Self {
        let url = std::env::var("AETHER_OBS_WEBSOCKET_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_OBS_WEBSOCKET_URL.to_owned());
        let password = std::env::var("AETHER_OBS_WEBSOCKET_PASSWORD")
            .ok()
            .filter(|value| !value.is_empty());
        Self { url, password }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ObsStatusSnapshot {
    pub connected: bool,
    pub obs_version: String,
    pub websocket_version: String,
    pub rpc_version: u64,
    pub current_scene: Option<String>,
    pub scenes: Vec<String>,
    pub streaming: bool,
    pub recording: bool,
    pub replay_buffer: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObsError {
    Transport(String),
    Json(String),
    Protocol(String),
    AuthenticationRequired,
    RequestFailed {
        request_type: String,
        code: i64,
        comment: String,
    },
}

impl fmt::Display for ObsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport(message) => {
                write!(formatter, "OBS WebSocket transport error: {message}")
            }
            Self::Json(message) => write!(formatter, "OBS WebSocket JSON error: {message}"),
            Self::Protocol(message) => write!(formatter, "OBS WebSocket protocol error: {message}"),
            Self::AuthenticationRequired => write!(
                formatter,
                "OBS WebSocket authentication is enabled; set AETHER_OBS_WEBSOCKET_PASSWORD"
            ),
            Self::RequestFailed {
                request_type,
                code,
                comment,
            } => write!(
                formatter,
                "OBS request {request_type} failed with code {code}: {comment}"
            ),
        }
    }
}

impl std::error::Error for ObsError {}

#[must_use]
pub fn obs_authentication(password: &str, salt: &str, challenge: &str) -> String {
    let secret_hash = Sha256::digest(format!("{password}{salt}").as_bytes());
    let secret = STANDARD.encode(secret_hash);
    let auth_hash = Sha256::digest(format!("{secret}{challenge}").as_bytes());
    STANDARD.encode(auth_hash)
}

pub struct ObsWebSocketClient {
    socket: ObsSocket,
    next_request_id: u64,
}

impl ObsWebSocketClient {
    pub fn connect(config: ObsWebSocketConfig) -> Result<Self, ObsError> {
        let (mut socket, _) =
            connect(config.url.as_str()).map_err(|error| ObsError::Transport(error.to_string()))?;
        let hello = read_json_message(&mut socket)?;
        if hello.get("op").and_then(Value::as_u64) != Some(0) {
            return Err(ObsError::Protocol("expected Hello opcode 0".to_owned()));
        }
        let data = hello
            .get("d")
            .and_then(Value::as_object)
            .ok_or_else(|| ObsError::Protocol("Hello payload is missing d object".to_owned()))?;
        let server_rpc_version = data
            .get("rpcVersion")
            .and_then(Value::as_u64)
            .ok_or_else(|| ObsError::Protocol("Hello payload is missing rpcVersion".to_owned()))?;
        let rpc_version = server_rpc_version.min(OBS_RPC_VERSION);

        let authentication = match data.get("authentication") {
            Some(Value::Object(authentication)) => {
                let salt = authentication
                    .get("salt")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ObsError::Protocol("authentication salt is missing".to_owned())
                    })?;
                let challenge = authentication
                    .get("challenge")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ObsError::Protocol("authentication challenge is missing".to_owned())
                    })?;
                let password = config
                    .password
                    .as_deref()
                    .ok_or(ObsError::AuthenticationRequired)?;
                Some(obs_authentication(password, salt, challenge))
            }
            Some(_) => {
                return Err(ObsError::Protocol(
                    "Hello authentication payload is not an object".to_owned(),
                ));
            }
            None => None,
        };

        let identify = authentication.map_or_else(
            || json!({"op": 1, "d": {"rpcVersion": rpc_version}}),
            |authentication| {
                json!({"op": 1, "d": {"rpcVersion": rpc_version, "authentication": authentication}})
            },
        );
        send_json_message(&mut socket, &identify)?;

        loop {
            let message = read_json_message(&mut socket)?;
            match message.get("op").and_then(Value::as_u64) {
                Some(2) => break,
                Some(5) => continue,
                Some(opcode) => {
                    return Err(ObsError::Protocol(format!(
                        "expected Identified opcode 2, received opcode {opcode}"
                    )));
                }
                None => return Err(ObsError::Protocol("message opcode is missing".to_owned())),
            }
        }

        Ok(Self {
            socket,
            next_request_id: 1,
        })
    }

    pub fn status(&mut self) -> Result<ObsStatusSnapshot, ObsError> {
        let version = self.request_raw("GetVersion", None)?;
        let scene_list = self.request_raw("GetSceneList", None)?;
        let stream = self.request_raw("GetStreamStatus", None)?;
        let record = self.request_raw("GetRecordStatus", None)?;
        let replay_buffer = self.replay_buffer_active()?;

        let scenes = scene_list
            .get("scenes")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|scene| scene.get("sceneName").and_then(Value::as_str))
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();

        Ok(ObsStatusSnapshot {
            connected: true,
            obs_version: version
                .get("obsVersion")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            websocket_version: version
                .get("obsWebSocketVersion")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            rpc_version: version
                .get("rpcVersion")
                .and_then(Value::as_u64)
                .unwrap_or(OBS_RPC_VERSION),
            current_scene: scene_list
                .get("currentProgramSceneName")
                .and_then(Value::as_str)
                .map(str::to_owned),
            scenes,
            streaming: output_active(&stream),
            recording: output_active(&record),
            replay_buffer,
        })
    }

    fn replay_buffer_active(&mut self) -> Result<bool, ObsError> {
        match self.request_raw("GetReplayBufferStatus", None) {
            Ok(response) => Ok(output_active(&response)),
            Err(ObsError::RequestFailed { .. }) => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub fn send_intent(&mut self, intent: StudioControlIntent) -> Result<(), ObsError> {
        let (request_type, request_data) = obs_request_for_intent(intent);
        self.request_raw(request_type, request_data).map(|_| ())
    }

    pub fn enforce_embedded_capture_safety(&mut self) -> Result<ObsCaptureSafetyReport, ObsError> {
        let _policy = CaptureSafetyPolicy::StrictNoRecursion;
        let video = self.request_raw("GetVideoSettings", None)?;
        let canvas_width = video
            .get("baseWidth")
            .and_then(Value::as_u64)
            .unwrap_or(1920) as u32;
        let canvas_height = video
            .get("baseHeight")
            .and_then(Value::as_u64)
            .unwrap_or(1080) as u32;
        let scenes = self.request_raw("GetSceneList", None)?;
        let scene_name = scenes
            .get("currentProgramSceneName")
            .and_then(Value::as_str)
            .ok_or_else(|| ObsError::Protocol("OBS has no current program scene".to_owned()))?
            .to_owned();
        let scene_items = self.request_raw(
            "GetSceneItemList",
            Some(json!({"sceneName": scene_name.as_str()})),
        )?;
        let mut report = ObsCaptureSafetyReport::default();
        for item in scene_items
            .get("sceneItems")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(source_name) = item.get("sourceName").and_then(Value::as_str) else {
                continue;
            };
            let Some(scene_item_id) = item.get("sceneItemId").and_then(Value::as_i64) else {
                continue;
            };
            report.checked_items = report.checked_items.saturating_add(1);
            let input =
                self.request_raw("GetInputSettings", Some(json!({"inputName": source_name})));
            let (input_kind, descriptor) = match input {
                Ok(input) => {
                    let input_kind = input
                        .get("inputKind")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned();
                    let settings = input.get("inputSettings").cloned().unwrap_or(Value::Null);
                    (input_kind, format!("{source_name} {settings}"))
                }
                Err(ObsError::RequestFailed { .. }) => (String::new(), source_name.to_owned()),
                Err(error) => return Err(error),
            };
            let recursive = source_name.eq_ignore_ascii_case(&scene_name)
                || is_recursive_capture_target(&input_kind, &descriptor);
            if recursive {
                self.request_raw(
                    "SetSceneItemEnabled",
                    Some(json!({
                        "sceneName": scene_name.as_str(),
                        "sceneItemId": scene_item_id,
                        "sceneItemEnabled": false,
                    })),
                )?;
                report
                    .blocked_recursive_sources
                    .push(source_name.to_owned());
                println!("AETHER_BROWSER_OBS_CAPTURE_SAFETY=BLOCKED:{source_name}");
                continue;
            }

            let transform = self.request_raw(
                "GetSceneItemTransform",
                Some(json!({"sceneName": scene_name.as_str(), "sceneItemId": scene_item_id})),
            )?;
            let transform = transform.get("sceneItemTransform").unwrap_or(&transform);
            let x = transform
                .get("positionX")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                .max(0.0) as u32;
            let y = transform
                .get("positionY")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                .max(0.0) as u32;
            let width = transform
                .get("width")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                .max(0.0) as u32;
            let height = transform
                .get("height")
                .and_then(Value::as_f64)
                .unwrap_or(0.0)
                .max(0.0) as u32;
            let original = CaptureRegion {
                x,
                y,
                width,
                height,
            };
            if let Some(clipped) = clip_region_to_canvas(original, canvas_width, canvas_height)
                && clipped != original
            {
                self.request_raw(
                    "SetSceneItemTransform",
                    Some(json!({
                        "sceneName": scene_name.as_str(),
                        "sceneItemId": scene_item_id,
                        "sceneItemTransform": {
                            "positionX": clipped.x,
                            "positionY": clipped.y,
                            "alignment": 5,
                            "boundsType": "OBS_BOUNDS_MAX_ONLY",
                            "boundsAlignment": 5,
                            "boundsWidth": clipped.width,
                            "boundsHeight": clipped.height
                        }
                    })),
                )?;
                report.clipped_items.push(source_name.to_owned());
                println!("AETHER_BROWSER_OBS_SCENE_OVERFLOW=CLIPPED:{source_name}");
            }
        }
        println!(
            "AETHER_BROWSER_OBS_CAPTURE_SAFETY=PASS:blocked={}:clipped={}",
            report.blocked_recursive_sources.len(),
            report.clipped_items.len()
        );
        Ok(report)
    }

    /// Send any OBS WebSocket v5 request while preserving AetherBrowser's
    /// request-id matching and error handling. This is the generic control
    /// plane used by the browser-authoritative Aether Studio UI.
    pub fn request_raw(
        &mut self,
        request_type: &str,
        request_data: Option<Value>,
    ) -> Result<Value, ObsError> {
        let request_id = format!("aether-{}", self.next_request_id);
        self.next_request_id = self.next_request_id.saturating_add(1);
        let mut data = json!({
            "requestType": request_type,
            "requestId": request_id.clone(),
        });
        if let Some(request_data) = request_data {
            data["requestData"] = request_data;
        }
        let message = json!({"op": 6, "d": data});
        send_json_message(&mut self.socket, &message)?;

        loop {
            let response = read_json_message(&mut self.socket)?;
            match response.get("op").and_then(Value::as_u64) {
                Some(5) => continue,
                Some(7) => {
                    let data = response.get("d").ok_or_else(|| {
                        ObsError::Protocol("RequestResponse payload is missing d".to_owned())
                    })?;
                    if data.get("requestId").and_then(Value::as_str) != Some(request_id.as_str()) {
                        continue;
                    }
                    let status = data.get("requestStatus").ok_or_else(|| {
                        ObsError::Protocol("RequestResponse is missing requestStatus".to_owned())
                    })?;
                    if status.get("result").and_then(Value::as_bool) != Some(true) {
                        return Err(ObsError::RequestFailed {
                            request_type: request_type.to_owned(),
                            code: status
                                .get("code")
                                .and_then(Value::as_i64)
                                .unwrap_or_default(),
                            comment: status
                                .get("comment")
                                .and_then(Value::as_str)
                                .unwrap_or("request rejected")
                                .to_owned(),
                        });
                    }
                    return Ok(data.get("responseData").cloned().unwrap_or(Value::Null));
                }
                Some(opcode) => {
                    return Err(ObsError::Protocol(format!(
                        "expected RequestResponse opcode 7, received opcode {opcode}"
                    )));
                }
                None => return Err(ObsError::Protocol("message opcode is missing".to_owned())),
            }
        }
    }
}

impl AetherStreamServiceBridge for ObsWebSocketClient {
    type Error = ObsError;

    fn state(&mut self) -> Result<AetherStreamServiceState, Self::Error> {
        let status = self.status()?;
        Ok(AetherStreamServiceState {
            connected: status.connected,
            live: status.streaming,
            recording: status.recording,
            replay_buffer: status.replay_buffer,
        })
    }

    fn send(&mut self, intent: StudioControlIntent) -> Result<(), Self::Error> {
        self.send_intent(intent)
    }
}

pub fn execute_obs_intent(
    config: ObsWebSocketConfig,
    intent: StudioControlIntent,
) -> Result<ObsStatusSnapshot, ObsError> {
    let mut client = ObsWebSocketClient::connect(config)?;
    client.send_intent(intent)?;
    client.status()
}

pub fn enforce_obs_embedded_capture_safety(
    config: ObsWebSocketConfig,
) -> Result<ObsCaptureSafetyReport, ObsError> {
    let mut client = ObsWebSocketClient::connect(config)?;
    client.enforce_embedded_capture_safety()
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ObsCaptureSafetyReport {
    pub checked_items: usize,
    pub blocked_recursive_sources: Vec<String>,
    pub clipped_items: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObsVerificationReport {
    pub obs_version: String,
    pub websocket_version: String,
    pub original_scene: String,
    pub scratch_scene: String,
    pub scenes_ok: bool,
    pub sources_ok: bool,
    pub inputs_ok: bool,
    pub filters_ok: bool,
    pub transitions_ok: bool,
    pub outputs_ok: bool,
    pub stats_ok: bool,
    pub profiles_ok: bool,
    pub scene_collections_ok: bool,
    pub virtual_camera_ok: bool,
    pub virtual_camera_available: bool,
    pub scratch_scene_ok: bool,
    pub scene_restore_ok: bool,
}

pub fn verify_obs_control_plane(
    config: ObsWebSocketConfig,
) -> Result<ObsVerificationReport, ObsError> {
    let _ = ObsBackendSupervisor::ensure_running(&config)?;
    let mut client = ObsWebSocketClient::connect(config)?;
    let status = client.status()?;
    let original_scene = status
        .current_scene
        .clone()
        .ok_or_else(|| ObsError::Protocol("OBS has no current program scene".to_owned()))?;
    let scratch_scene = format!("AetherBrowser Verify {}", std::process::id());

    let inputs_ok = client.request_raw("GetInputList", None).is_ok();
    let transitions_ok = client.request_raw("GetSceneTransitionList", None).is_ok();
    let outputs_ok = client.request_raw("GetOutputList", None).is_ok();
    let stats_ok = client.request_raw("GetStats", None).is_ok();
    let profiles_ok = client.request_raw("GetProfileList", None).is_ok();
    let scene_collections_ok = client.request_raw("GetSceneCollectionList", None).is_ok();
    let (virtual_camera_available, virtual_camera_ok) =
        match client.request_raw("GetVirtualCamStatus", None) {
            Ok(_) => (true, true),
            Err(ObsError::RequestFailed { code, comment, .. })
                if code == 604
                    || comment.to_ascii_lowercase().contains("not available")
                    || comment.to_ascii_lowercase().contains("not supported")
                    || comment
                        .to_ascii_lowercase()
                        .contains("invalid resource state") =>
            {
                (false, true)
            }
            Err(_) => (true, false),
        };
    let sources_ok = client
        .request_raw(
            "GetSceneItemList",
            Some(json!({"sceneName": original_scene.as_str()})),
        )
        .is_ok();
    let filters_ok = client
        .request_raw(
            "GetSourceFilterList",
            Some(json!({"sourceName": original_scene.as_str()})),
        )
        .is_ok();

    client.request_raw(
        "CreateScene",
        Some(json!({"sceneName": scratch_scene.as_str()})),
    )?;
    let mut scratch_scene_ok = false;
    let mut scene_restore_ok = false;
    let exercise = (|| -> Result<(), ObsError> {
        client.request_raw(
            "SetCurrentProgramScene",
            Some(json!({"sceneName": scratch_scene.as_str()})),
        )?;
        let current = client.request_raw("GetCurrentProgramScene", None)?;
        scratch_scene_ok = current
            .get("currentProgramSceneName")
            .and_then(Value::as_str)
            == Some(scratch_scene.as_str());
        client.request_raw(
            "SetCurrentProgramScene",
            Some(json!({"sceneName": original_scene.as_str()})),
        )?;
        let current = client.request_raw("GetCurrentProgramScene", None)?;
        scene_restore_ok = current
            .get("currentProgramSceneName")
            .and_then(Value::as_str)
            == Some(original_scene.as_str());
        Ok(())
    })();

    if !scene_restore_ok {
        let _ = client.request_raw(
            "SetCurrentProgramScene",
            Some(json!({"sceneName": original_scene.as_str()})),
        );
    }
    let cleanup = client.request_raw(
        "RemoveScene",
        Some(json!({"sceneName": scratch_scene.as_str()})),
    );
    exercise?;
    cleanup?;

    Ok(ObsVerificationReport {
        obs_version: status.obs_version,
        websocket_version: status.websocket_version,
        original_scene,
        scratch_scene,
        scenes_ok: !status.scenes.is_empty(),
        sources_ok,
        inputs_ok,
        filters_ok,
        transitions_ok,
        outputs_ok,
        stats_ok,
        profiles_ok,
        scene_collections_ok,
        virtual_camera_ok,
        virtual_camera_available,
        scratch_scene_ok,
        scene_restore_ok,
    })
}

pub fn query_obs_status(config: ObsWebSocketConfig) -> Result<ObsStatusSnapshot, ObsError> {
    let mut client = ObsWebSocketClient::connect(config)?;
    client.status()
}

/// Browser-owned lifecycle boundary for the local OBS compatibility engine.
///
/// Aether Studio remains the user-facing UI. When the canonical local OBS
/// WebSocket endpoint is offline, this supervisor starts OBS minimized to the
/// tray and waits for the built-in OBS WebSocket server to accept connections.
/// It never kills OBS on AetherBrowser window close; active stream/recording
/// state therefore survives a browser-UI restart unless the user explicitly
/// stops the backend.
pub struct ObsBackendSupervisor;

impl ObsBackendSupervisor {
    pub fn ensure_running(config: &ObsWebSocketConfig) -> Result<bool, ObsError> {
        if config.url != DEFAULT_OBS_WEBSOCKET_URL {
            return Ok(false);
        }
        let endpoint: SocketAddr = "127.0.0.1:4455".parse().map_err(|error| {
            ObsError::Transport(format!("invalid OBS loopback endpoint: {error}"))
        })?;
        if TcpStream::connect_timeout(&endpoint, Duration::from_millis(180)).is_ok() {
            return Ok(false);
        }

        let executable = std::env::var_os("AETHER_OBS_BINARY")
            .map(PathBuf::from)
            .filter(|path| path.is_file())
            .or_else(|| {
                let path = PathBuf::from("/usr/bin/obs");
                path.is_file().then_some(path)
            })
            .unwrap_or_else(|| PathBuf::from("obs"));

        let mut command = Command::new(executable);
        command
            .arg("--minimize-to-tray")
            .arg("--disable-missing-files-check")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command.spawn().map_err(|error| {
            ObsError::Transport(format!("unable to start OBS backend: {error}"))
        })?;
        println!("AETHER_BROWSER_OBS_BACKEND=STARTED:minimized");

        for _ in 0..120 {
            if TcpStream::connect_timeout(&endpoint, Duration::from_millis(180)).is_ok() {
                println!("AETHER_BROWSER_OBS_BACKEND=READY:127.0.0.1:4455");
                return Ok(true);
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err(ObsError::Transport(
            "OBS started but its WebSocket endpoint did not become ready".to_owned(),
        ))
    }
}

pub fn query_obs_status_with_autostart(
    config: ObsWebSocketConfig,
) -> Result<ObsStatusSnapshot, ObsError> {
    match query_obs_status(config.clone()) {
        Ok(snapshot) => Ok(snapshot),
        Err(first_error) => match ObsBackendSupervisor::ensure_running(&config) {
            Ok(true) => query_obs_status(config),
            Ok(false) => Err(first_error),
            Err(start_error) => Err(start_error),
        },
    }
}

fn obs_request_for_intent(intent: StudioControlIntent) -> (&'static str, Option<Value>) {
    match intent {
        StudioControlIntent::ActivateScene(scene_name) => (
            "SetCurrentProgramScene",
            Some(json!({"sceneName": scene_name})),
        ),
        StudioControlIntent::SetSourceVisibility {
            scene,
            scene_item_id,
            enabled,
        } => (
            "SetSceneItemEnabled",
            Some(
                json!({"sceneName": scene, "sceneItemId": scene_item_id, "sceneItemEnabled": enabled}),
            ),
        ),
        StudioControlIntent::SetInputMute { input, muted } => (
            "SetInputMute",
            Some(json!({"inputName": input, "inputMuted": muted})),
        ),
        StudioControlIntent::SetMixerGain {
            channel,
            gain_millidb,
        } => (
            "SetInputVolume",
            Some(json!({
                "inputName": channel,
                "inputVolumeDb": f64::from(gain_millidb) / 1000.0,
            })),
        ),
        StudioControlIntent::SetTransition(transition_name) => (
            "SetCurrentSceneTransition",
            Some(json!({"transitionName": transition_name})),
        ),
        StudioControlIntent::TriggerTransition => ("TriggerStudioModeTransition", None),
        StudioControlIntent::StartStream => ("StartStream", None),
        StudioControlIntent::StopStream => ("StopStream", None),
        StudioControlIntent::StartRecording => ("StartRecord", None),
        StudioControlIntent::StopRecording => ("StopRecord", None),
        StudioControlIntent::PauseRecording => ("PauseRecord", None),
        StudioControlIntent::ResumeRecording => ("ResumeRecord", None),
        StudioControlIntent::StartReplayBuffer => ("StartReplayBuffer", None),
        StudioControlIntent::StopReplayBuffer => ("StopReplayBuffer", None),
        StudioControlIntent::SaveReplay => ("SaveReplayBuffer", None),
        StudioControlIntent::StartVirtualCamera => ("StartVirtualCam", None),
        StudioControlIntent::StopVirtualCamera => ("StopVirtualCam", None),
        StudioControlIntent::SetStudioMode(enabled) => (
            "SetStudioModeEnabled",
            Some(json!({"studioModeEnabled": enabled})),
        ),
        StudioControlIntent::SetProfile(profile_name) => (
            "SetCurrentProfile",
            Some(json!({"profileName": profile_name})),
        ),
        StudioControlIntent::SetSceneCollection(scene_collection_name) => (
            "SetCurrentSceneCollection",
            Some(json!({"sceneCollectionName": scene_collection_name})),
        ),
    }
}

fn output_active(response: &Value) -> bool {
    response
        .get("outputActive")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn send_json_message(socket: &mut ObsSocket, value: &Value) -> Result<(), ObsError> {
    socket
        .send(Message::text(value.to_string()))
        .map_err(|error| ObsError::Transport(error.to_string()))
}

fn read_json_message(socket: &mut ObsSocket) -> Result<Value, ObsError> {
    loop {
        let message = socket
            .read()
            .map_err(|error| ObsError::Transport(error.to_string()))?;
        match message {
            Message::Text(text) => {
                return serde_json::from_str(text.as_str())
                    .map_err(|error| ObsError::Json(error.to_string()));
            }
            Message::Binary(bytes) => {
                return serde_json::from_slice(&bytes)
                    .map_err(|error| ObsError::Json(error.to_string()));
            }
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => continue,
            Message::Close(frame) => {
                return Err(ObsError::Transport(format!(
                    "OBS WebSocket closed the connection: {frame:?}"
                )));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_authentication_example_matches_obs_protocol() {
        assert_eq!(
            obs_authentication(
                "supersecretpassword",
                "lM1GncleQOaCu9lT1yeUZhFYnqhsLLP1G5lAGo3ixaI=",
                "+IxH4CnCiqpX1rM9scsNynZzbOe4KhDeYcTNS3PDaeY=",
            ),
            "1Ct943GAT+6YQUUX47Ia/ncufilbe6+oD6lY+5kaCu4="
        );
    }

    #[test]
    fn intent_mapping_uses_obs_websocket_v5_request_names() {
        assert_eq!(
            obs_request_for_intent(StudioControlIntent::StartStream).0,
            "StartStream"
        );
        assert_eq!(
            obs_request_for_intent(StudioControlIntent::StartRecording).0,
            "StartRecord"
        );
        assert_eq!(
            obs_request_for_intent(StudioControlIntent::SaveReplay).0,
            "SaveReplayBuffer"
        );
    }
}
