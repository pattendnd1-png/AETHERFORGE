#![forbid(unsafe_code)]
//! Media-session, playback, and site-verification contracts.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaPlaybackState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaSession {
    pub title: String,
    pub artist: Option<String>,
    pub state: MediaPlaybackState,
    pub source_url: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaBackend {
    Dummy,
    GStreamer,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaCapability {
    Audio,
    Video,
    WebRtc,
    Fullscreen,
    PictureInPicture,
    MediaSourceExtensions,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MediaSite {
    YouTube,
    Twitch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SitePlaybackVerification {
    Unverified,
    Pass,
    Fail,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaSupportMatrix {
    pub backend: MediaBackend,
    pub backend_capabilities: Vec<MediaCapability>,
    pub youtube: SitePlaybackVerification,
    pub twitch: SitePlaybackVerification,
    pub mse: SitePlaybackVerification,
}

impl MediaSupportMatrix {
    #[must_use]
    pub fn gstreamer_unverified() -> Self {
        Self {
            backend: MediaBackend::GStreamer,
            backend_capabilities: vec![
                MediaCapability::Audio,
                MediaCapability::Video,
                MediaCapability::WebRtc,
                MediaCapability::Fullscreen,
                MediaCapability::PictureInPicture,
            ],
            youtube: SitePlaybackVerification::Unverified,
            twitch: SitePlaybackVerification::Unverified,
            mse: SitePlaybackVerification::Unverified,
        }
    }
}
