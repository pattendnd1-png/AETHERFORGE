#![forbid(unsafe_code)]
//! Browser-native Aether Stream Studio control and takeover contracts.

pub mod obs;
pub use obs::{
    ObsBackendSupervisor, ObsError, ObsStatusSnapshot, ObsVerificationReport, ObsWebSocketClient,
    ObsWebSocketConfig, execute_obs_intent, obs_authentication, query_obs_status,
    query_obs_status_with_autostart, verify_obs_control_plane,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StandaloneGuiPolicy {
    Retired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StudioAuthority {
    BrowserAuthoritative,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StudioCapability {
    Scenes,
    Sources,
    Inputs,
    Filters,
    Transitions,
    Mixer,
    Streaming,
    Recording,
    ReplayBuffer,
    VirtualCamera,
    Profiles,
    SceneCollections,
    Outputs,
    Stats,
}

pub const STUDIO_CAPABILITIES: &[StudioCapability] = &[
    StudioCapability::Scenes,
    StudioCapability::Sources,
    StudioCapability::Inputs,
    StudioCapability::Filters,
    StudioCapability::Transitions,
    StudioCapability::Mixer,
    StudioCapability::Streaming,
    StudioCapability::Recording,
    StudioCapability::ReplayBuffer,
    StudioCapability::VirtualCamera,
    StudioCapability::Profiles,
    StudioCapability::SceneCollections,
    StudioCapability::Outputs,
    StudioCapability::Stats,
];

#[must_use]
pub const fn studio_capabilities() -> &'static [StudioCapability] {
    STUDIO_CAPABILITIES
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StudioRoute {
    Dashboard,
    Scenes,
    Sources,
    Mixer,
    Dsp,
    Recording,
    Replay,
    Providers,
    Chat,
    MediaLibrary,
    Editor,
    Settings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AetherStreamServiceState {
    pub connected: bool,
    pub live: bool,
    pub recording: bool,
    pub replay_buffer: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StudioControlIntent {
    ActivateScene(String),
    SetSourceVisibility {
        scene: String,
        scene_item_id: i64,
        enabled: bool,
    },
    SetInputMute {
        input: String,
        muted: bool,
    },
    SetMixerGain {
        channel: String,
        gain_millidb: i32,
    },
    SetTransition(String),
    TriggerTransition,
    StartStream,
    StopStream,
    StartRecording,
    StopRecording,
    PauseRecording,
    ResumeRecording,
    StartReplayBuffer,
    StopReplayBuffer,
    SaveReplay,
    StartVirtualCamera,
    StopVirtualCamera,
    SetStudioMode(bool),
    SetProfile(String),
    SetSceneCollection(String),
}

pub trait AetherStreamServiceBridge {
    type Error;
    fn state(&mut self) -> Result<AetherStreamServiceState, Self::Error>;
    fn send(&mut self, intent: StudioControlIntent) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamStudioState {
    pub route: StudioRoute,
    pub service: AetherStreamServiceState,
    pub authority: StudioAuthority,
    pub standalone_gui: StandaloneGuiPolicy,
}

impl StreamStudioState {
    #[must_use]
    pub fn canonical() -> Self {
        Self {
            route: StudioRoute::Dashboard,
            service: AetherStreamServiceState {
                connected: false,
                live: false,
                recording: false,
                replay_buffer: false,
            },
            authority: StudioAuthority::BrowserAuthoritative,
            standalone_gui: StandaloneGuiPolicy::Retired,
        }
    }

    pub fn disconnect_ui(&mut self) {
        self.service.connected = false;
    }
}

#[must_use]
pub fn supervisor_socket_path() -> std::path::PathBuf {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(dir).join("aetherstream/supervisor.sock")
    } else {
        std::path::PathBuf::from(format!(
            "/tmp/aetherstream-{}/supervisor.sock",
            std::env::var("UID").unwrap_or_else(|_| "user".into())
        ))
    }
}

#[cfg(unix)]
#[must_use]
pub fn supervisor_socket_present() -> bool {
    use std::os::unix::fs::FileTypeExt;

    std::fs::metadata(supervisor_socket_path())
        .map(|metadata| metadata.file_type().is_socket())
        .unwrap_or(false)
}

#[cfg(not(unix))]
#[must_use]
pub fn supervisor_socket_present() -> bool {
    false
}
