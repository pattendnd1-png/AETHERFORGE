#![forbid(unsafe_code)]
//! Typed creator-service adapters. Tokens are represented only by secret-store references.

pub mod vendor_packages;

pub const TWITCH_AUTH_URL: &str = "https://id.twitch.tv/oauth2/authorize";
pub const TWITCH_API_BASE: &str = "https://api.twitch.tv/helix";
pub const TWITCH_EVENTSUB_WEBSOCKET: &str = "wss://eventsub.wss.twitch.tv/ws";
pub const TWITCH_WEB_URL: &str = "https://www.twitch.tv/";
pub const STREAMLABS_AUTH_URL: &str = "https://streamlabs.com/api/v2.0/authorize";
pub const STREAMLABS_API_BASE: &str = "https://streamlabs.com/api/v2.0";
pub const STREAMLABS_SOCKET_URL: &str = "https://sockets.streamlabs.com";
pub const STREAMLABS_WEB_URL: &str = "https://streamlabs.com/";
pub const STREAMLABS_CAPTURE_SAFETY_POLICY: &str = "STRICT_NO_RECURSION";
pub const STREAMELEMENTS_WEB_URL: &str = "https://streamelements.com/";
pub const STREAMELEMENTS_API_DOCS_URL: &str = "https://docs.streamelements.com/";
pub const STREAMELEMENTS_ASTRO_URL: &str = "wss://astro.streamelements.com/";
pub const BLUESKY_WEB_URL: &str = "https://bsky.app/";
pub const BLUESKY_PUBLIC_API: &str = "https://public.api.bsky.app";
pub const ATPROTO_OAUTH_PROTECTED_RESOURCE_PATH: &str = "/.well-known/oauth-protected-resource";
pub const ATPROTO_OAUTH_AUTH_SERVER_METADATA_PATH: &str = "/.well-known/oauth-authorization-server";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CreatorProviderKind {
    Twitch,
    Streamlabs,
    StreamElements,
    Bluesky,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecretRef(pub String);

impl SecretRef {
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderConnectionState {
    Disconnected,
    RequiresClientConfiguration,
    AuthorizationPending,
    Connected,
    Degraded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CreatorCapability {
    StreamInfo,
    Chat,
    Moderation,
    Clips,
    Raids,
    Polls,
    Predictions,
    ChannelPoints,
    Followers,
    Subscribers,
    Cheers,
    StreamHealth,
    Alerts,
    Donations,
    MediaShare,
    BrowserSources,
    Overlays,
    Chatbot,
    RealtimeEvents,
    SocialPost,
    SocialFeed,
    DirectMessages,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatorProviderDescriptor {
    pub kind: CreatorProviderKind,
    pub display_name: &'static str,
    pub official_web_url: &'static str,
    pub auth_url: &'static str,
    pub api_base: &'static str,
    pub realtime_url: Option<&'static str>,
    pub token_secret: SecretRef,
    pub capabilities: &'static [CreatorCapability],
}

const TWITCH_CAPABILITIES: &[CreatorCapability] = &[
    CreatorCapability::StreamInfo,
    CreatorCapability::Chat,
    CreatorCapability::Moderation,
    CreatorCapability::Clips,
    CreatorCapability::Raids,
    CreatorCapability::Polls,
    CreatorCapability::Predictions,
    CreatorCapability::ChannelPoints,
    CreatorCapability::Followers,
    CreatorCapability::Subscribers,
    CreatorCapability::Cheers,
    CreatorCapability::StreamHealth,
    CreatorCapability::RealtimeEvents,
];

#[must_use]
pub fn twitch_descriptor() -> CreatorProviderDescriptor {
    CreatorProviderDescriptor {
        kind: CreatorProviderKind::Twitch,
        display_name: "Twitch Creator",
        official_web_url: TWITCH_WEB_URL,
        auth_url: TWITCH_AUTH_URL,
        api_base: TWITCH_API_BASE,
        realtime_url: Some(TWITCH_EVENTSUB_WEBSOCKET),
        token_secret: SecretRef::new("creator/twitch/oauth-token"),
        capabilities: TWITCH_CAPABILITIES,
    }
}

pub fn twitch_authorize_url(
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    scopes: &[&str],
) -> Result<url::Url, url::ParseError> {
    let mut url = url::Url::parse(TWITCH_AUTH_URL)?;
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("state", state)
        .append_pair("scope", &scopes.join(" "));
    Ok(url)
}

const STREAMLABS_CAPABILITIES: &[CreatorCapability] = &[
    CreatorCapability::Alerts,
    CreatorCapability::Donations,
    CreatorCapability::MediaShare,
    CreatorCapability::BrowserSources,
    CreatorCapability::Overlays,
    CreatorCapability::Chatbot,
    CreatorCapability::RealtimeEvents,
    CreatorCapability::StreamHealth,
];

#[must_use]
pub fn streamlabs_descriptor() -> CreatorProviderDescriptor {
    CreatorProviderDescriptor {
        kind: CreatorProviderKind::Streamlabs,
        display_name: "Streamlabs Suite",
        official_web_url: STREAMLABS_WEB_URL,
        auth_url: STREAMLABS_AUTH_URL,
        api_base: STREAMLABS_API_BASE,
        realtime_url: Some(STREAMLABS_SOCKET_URL),
        token_secret: SecretRef::new("creator/streamlabs/oauth-token"),
        capabilities: STREAMLABS_CAPABILITIES,
    }
}

pub const STREAMELEMENTS_TOPICS: &[&str] = &[
    "channel.activities",
    "channel.stream.status",
    "channel.overlay.update",
    "channel.overlay.broadcast",
    "channel.chat.message",
    "channel.tips",
];

const STREAMELEMENTS_CAPABILITIES: &[CreatorCapability] = &[
    CreatorCapability::Alerts,
    CreatorCapability::Donations,
    CreatorCapability::BrowserSources,
    CreatorCapability::Overlays,
    CreatorCapability::Chatbot,
    CreatorCapability::RealtimeEvents,
    CreatorCapability::Chat,
];

#[must_use]
pub fn stream_elements_descriptor() -> CreatorProviderDescriptor {
    CreatorProviderDescriptor {
        kind: CreatorProviderKind::StreamElements,
        display_name: "StreamElements Suite",
        official_web_url: STREAMELEMENTS_WEB_URL,
        auth_url: STREAMELEMENTS_WEB_URL,
        api_base: STREAMELEMENTS_API_DOCS_URL,
        realtime_url: Some(STREAMELEMENTS_ASTRO_URL),
        token_secret: SecretRef::new("creator/streamelements/oauth-or-channel-token"),
        capabilities: STREAMELEMENTS_CAPABILITIES,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntegrationOwnership {
    /// AetherBrowser/Aether Studio owns the local UX and production state.
    BrowserAuthoritative,
    /// The provider owns remote account data or a cloud-only business feature.
    ProviderCloud,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CreatorImportKind {
    ObsSceneCollectionImport,
    StreamlabsDesktopImport,
    StreamElementsOverlayImport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CreatorIntegrationSurface {
    pub provider: CreatorProviderKind,
    pub ownership: IntegrationOwnership,
    pub import_kind: Option<CreatorImportKind>,
    pub persistent_compatibility_session: bool,
}

pub const CREATOR_INTEGRATION_SURFACES: &[CreatorIntegrationSurface] = &[
    CreatorIntegrationSurface {
        provider: CreatorProviderKind::Twitch,
        ownership: IntegrationOwnership::BrowserAuthoritative,
        import_kind: None,
        persistent_compatibility_session: true,
    },
    CreatorIntegrationSurface {
        provider: CreatorProviderKind::Streamlabs,
        ownership: IntegrationOwnership::BrowserAuthoritative,
        import_kind: Some(CreatorImportKind::StreamlabsDesktopImport),
        persistent_compatibility_session: true,
    },
    CreatorIntegrationSurface {
        provider: CreatorProviderKind::StreamElements,
        ownership: IntegrationOwnership::BrowserAuthoritative,
        import_kind: Some(CreatorImportKind::StreamElementsOverlayImport),
        persistent_compatibility_session: true,
    },
];

#[must_use]
pub const fn creator_integration_surfaces() -> &'static [CreatorIntegrationSurface] {
    CREATOR_INTEGRATION_SURFACES
}

#[must_use]
pub const fn obs_scene_collection_import_kind() -> CreatorImportKind {
    CreatorImportKind::ObsSceneCollectionImport
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnifiedCreatorAction {
    StartStream,
    StopStream,
    StartRecording,
    StopRecording,
    SaveReplay,
    ActivateScene(String),
    SetSourceVisibility {
        scene: String,
        scene_item_id: i64,
        enabled: bool,
    },
    ToggleMute(String),
    TriggerTransition,
    ToggleStudioMode,
    ToggleVirtualCamera,
    ShareLatestClip,
    PostLiveAnnouncement,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CreatorEvent {
    StreamOnline,
    StreamOffline,
    Follow {
        provider: CreatorProviderKind,
        user: String,
    },
    Subscription {
        provider: CreatorProviderKind,
        user: String,
    },
    Donation {
        provider: CreatorProviderKind,
        display: String,
    },
    Alert {
        provider: CreatorProviderKind,
        kind: String,
    },
    ChatMessage {
        provider: CreatorProviderKind,
        user: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CreatorTelemetrySource {
    NativeStreamStudio,
    ExternalObsCompatibility,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CreatorProviderRegistry {
    providers: Vec<CreatorProviderKind>,
}

impl CreatorProviderRegistry {
    #[must_use]
    pub fn canonical() -> Self {
        Self {
            providers: vec![
                CreatorProviderKind::Twitch,
                CreatorProviderKind::Streamlabs,
                CreatorProviderKind::StreamElements,
                CreatorProviderKind::Bluesky,
            ],
        }
    }

    #[must_use]
    pub fn providers(&self) -> &[CreatorProviderKind] {
        &self.providers
    }

    #[must_use]
    pub const fn telemetry_source(&self) -> CreatorTelemetrySource {
        CreatorTelemetrySource::NativeStreamStudio
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BlueskyOAuthProfile {
    pub requires_pkce: bool,
    pub requires_par: bool,
    pub requires_dpop: bool,
    pub client_secret_allowed: bool,
}

impl BlueskyOAuthProfile {
    #[must_use]
    pub const fn canonical() -> Self {
        Self {
            requires_pkce: true,
            requires_par: true,
            requires_dpop: true,
            client_secret_allowed: false,
        }
    }
}

pub const BLUESKY_LEXICONS: &[&str] = &[
    "app.bsky.feed.post",
    "app.bsky.feed.like",
    "app.bsky.feed.repost",
    "app.bsky.actor.getProfile",
    "app.bsky.notification.listNotifications",
    "chat.bsky.convo.getConvoList",
    "chat.bsky.convo.sendMessage",
];

const BLUESKY_CAPABILITIES: &[CreatorCapability] = &[
    CreatorCapability::SocialPost,
    CreatorCapability::SocialFeed,
    CreatorCapability::DirectMessages,
    CreatorCapability::RealtimeEvents,
];

#[must_use]
pub fn bluesky_descriptor() -> CreatorProviderDescriptor {
    CreatorProviderDescriptor {
        kind: CreatorProviderKind::Bluesky,
        display_name: "Bluesky / AT Protocol",
        official_web_url: BLUESKY_WEB_URL,
        auth_url: BLUESKY_WEB_URL,
        api_base: BLUESKY_PUBLIC_API,
        realtime_url: None,
        token_secret: SecretRef::new("creator/bluesky/atproto-oauth-session"),
        capabilities: BLUESKY_CAPABILITIES,
    }
}
