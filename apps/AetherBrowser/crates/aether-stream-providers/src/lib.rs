#![forbid(unsafe_code)]
//! Installable streaming and production-provider contracts.

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativePlaybackProvider {
    YouTube,
    Twitch,
}

impl NativePlaybackProvider {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::YouTube => "youtube",
            Self::Twitch => "twitch",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativePlaybackRequest {
    pub provider: NativePlaybackProvider,
    pub resource: String,
    pub original_url: String,
}

impl NativePlaybackRequest {
    #[must_use]
    pub fn native_player_url(&self) -> String {
        let query = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("provider", self.provider.as_str())
            .append_pair("resource", &self.resource)
            .append_pair("url", &self.original_url)
            .finish();
        format!("aether://watch?{query}")
    }
}

#[must_use]
pub fn classify_native_playback_url(raw_url: &str) -> Option<NativePlaybackRequest> {
    let parsed = url::Url::parse(raw_url.trim()).ok()?;
    if parsed
        .query_pairs()
        .any(|(key, value)| key == "aether_web" && value == "1")
    {
        return None;
    }
    let host = parsed
        .host_str()?
        .trim_start_matches("www.")
        .to_ascii_lowercase();
    let segments: Vec<_> = parsed
        .path_segments()
        .map(|parts| parts.filter(|part| !part.is_empty()).collect())
        .unwrap_or_default();

    if host == "youtube.com" || host == "m.youtube.com" || host == "music.youtube.com" {
        let resource = if segments.first().copied() == Some("watch") {
            parsed
                .query_pairs()
                .find(|(key, _)| key == "v")
                .map(|(_, value)| value.into_owned())
        } else if matches!(segments.first().copied(), Some("shorts" | "live" | "embed")) {
            segments.get(1).map(|value| (*value).to_owned())
        } else {
            None
        }?;
        if resource.is_empty() {
            return None;
        }
        return Some(NativePlaybackRequest {
            provider: NativePlaybackProvider::YouTube,
            resource,
            original_url: raw_url.to_owned(),
        });
    }

    if host == "youtu.be" {
        let resource = segments.first()?.to_string();
        if resource.is_empty() {
            return None;
        }
        return Some(NativePlaybackRequest {
            provider: NativePlaybackProvider::YouTube,
            resource,
            original_url: raw_url.to_owned(),
        });
    }

    if host == "twitch.tv" {
        let first = segments.first()?.to_ascii_lowercase();
        const RESERVED: &[&str] = &[
            "activate",
            "directory",
            "downloads",
            "jobs",
            "login",
            "oauth2",
            "p",
            "search",
            "settings",
            "signup",
            "subscriptions",
            "inventory",
            "wallet",
        ];
        if RESERVED.contains(&first.as_str()) {
            return None;
        }
        if first == "videos" {
            // Keep Twitch VOD playback in the authoritative Chromium web surface so
            // Twitch's authenticated normal-profile session remains intact.
            // The native transport is diagnostic/explicit only.
            return None;
        }
        return Some(NativePlaybackRequest {
            provider: NativePlaybackProvider::Twitch,
            resource: first,
            original_url: raw_url.to_owned(),
        });
    }

    None
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderKind {
    Twitch,
    YouTubeLive,
    Velora,
    ObsStudio,
    Streamlabs,
    StreamElements,
    Kick,
    CustomRtmp,
    CustomSrt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderCapability {
    Auth,
    Stream,
    Chat,
    Moderation,
    Clips,
    Analytics,
    Scenes,
    Sources,
    Recording,
    Replay,
    Alerts,
    Overlays,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderAuthMode {
    TwitchOidc,
    TwitchDelegated,
    GoogleOAuth,
    ProviderOAuth,
    WebSession,
    LocalIpc,
    NoAuth,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderManifest {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    pub auth: ProviderAuthMode,
    pub capabilities: Vec<ProviderCapability>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderRegistryError {
    DuplicateId,
    Missing,
}

#[derive(Clone, Debug, Default)]
pub struct ProviderRegistry {
    installed: BTreeMap<String, ProviderManifest>,
}

impl ProviderRegistry {
    pub fn install(&mut self, manifest: ProviderManifest) -> Result<(), ProviderRegistryError> {
        if self.installed.contains_key(&manifest.id) {
            return Err(ProviderRegistryError::DuplicateId);
        }
        self.installed.insert(manifest.id.clone(), manifest);
        Ok(())
    }

    pub fn uninstall(&mut self, id: &str) -> Result<ProviderManifest, ProviderRegistryError> {
        self.installed
            .remove(id)
            .ok_or(ProviderRegistryError::Missing)
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&ProviderManifest> {
        self.installed.get(id)
    }

    pub fn installed(&self) -> impl Iterator<Item = &ProviderManifest> {
        self.installed.values()
    }
}

pub fn built_in_providers() -> Vec<ProviderManifest> {
    use ProviderAuthMode::{
        GoogleOAuth, LocalIpc, NoAuth, ProviderOAuth, TwitchDelegated, TwitchOidc, WebSession,
    };
    use ProviderCapability::{
        Alerts, Analytics, Auth, Chat, Clips, Moderation, Overlays, Recording, Replay, Scenes,
        Sources, Stream,
    };
    vec![
        ProviderManifest {
            id: "twitch".into(),
            name: "Twitch".into(),
            kind: ProviderKind::Twitch,
            auth: TwitchOidc,
            capabilities: vec![Auth, Stream, Chat, Moderation, Clips, Analytics],
        },
        ProviderManifest {
            id: "youtube-live".into(),
            name: "YouTube Live".into(),
            kind: ProviderKind::YouTubeLive,
            auth: GoogleOAuth,
            capabilities: vec![Auth, Stream, Chat, Analytics],
        },
        ProviderManifest {
            id: "velora".into(),
            name: "Velora.tv".into(),
            kind: ProviderKind::Velora,
            auth: WebSession,
            capabilities: vec![Auth, Stream, Chat],
        },
        ProviderManifest {
            id: "obs".into(),
            name: "OBS Studio".into(),
            kind: ProviderKind::ObsStudio,
            auth: LocalIpc,
            capabilities: vec![Scenes, Sources, Stream, Recording, Replay],
        },
        ProviderManifest {
            id: "streamlabs".into(),
            name: "Streamlabs".into(),
            kind: ProviderKind::Streamlabs,
            auth: TwitchDelegated,
            capabilities: vec![Auth, Alerts, Overlays, Chat, Stream],
        },
        ProviderManifest {
            id: "streamelements".into(),
            name: "StreamElements".into(),
            kind: ProviderKind::StreamElements,
            auth: TwitchDelegated,
            capabilities: vec![Auth, Alerts, Overlays, Chat],
        },
        ProviderManifest {
            id: "kick".into(),
            name: "Kick".into(),
            kind: ProviderKind::Kick,
            auth: ProviderOAuth,
            capabilities: vec![Auth, Stream, Chat],
        },
        ProviderManifest {
            id: "custom-rtmp".into(),
            name: "Custom RTMP".into(),
            kind: ProviderKind::CustomRtmp,
            auth: NoAuth,
            capabilities: vec![Stream],
        },
        ProviderManifest {
            id: "custom-srt".into(),
            name: "Custom SRT".into(),
            kind: ProviderKind::CustomSrt,
            auth: NoAuth,
            capabilities: vec![Stream],
        },
    ]
}

#[cfg(test)]
mod native_playback_tests {
    use super::{NativePlaybackProvider, classify_native_playback_url};

    #[test]
    fn youtube_music_watch_uses_native_youtube_backend() {
        let request = classify_native_playback_url("https://music.youtube.com/watch?v=abc123")
            .expect("YouTube Music watch URL should use the native media backend");
        assert_eq!(request.provider, NativePlaybackProvider::YouTube);
        assert_eq!(request.resource, "abc123");
    }

    #[test]
    fn twitch_vod_stays_in_authenticated_servo_web_player() {
        assert!(classify_native_playback_url("https://www.twitch.tv/videos/123456").is_none());
    }

    #[test]
    fn twitch_live_channel_still_uses_native_backend() {
        let request = classify_native_playback_url("https://www.twitch.tv/example_channel")
            .expect("live Twitch channel should use native media backend");
        assert_eq!(request.provider, NativePlaybackProvider::Twitch);
        assert_eq!(request.resource, "example_channel");
    }
}
