use aether_media::{MediaBackend, MediaSupportMatrix, SitePlaybackVerification};

#[test]
fn gstreamer_backend_does_not_fake_site_playback_success() {
    let media = MediaSupportMatrix::gstreamer_unverified();
    assert_eq!(media.backend, MediaBackend::GStreamer);
    assert_eq!(media.youtube, SitePlaybackVerification::Unverified);
    assert_eq!(media.twitch, SitePlaybackVerification::Unverified);
    assert_eq!(media.mse, SitePlaybackVerification::Unverified);
}
