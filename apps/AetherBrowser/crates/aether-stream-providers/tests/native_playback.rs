use aether_stream_providers::{NativePlaybackProvider, classify_native_playback_url};

#[test]
fn youtube_watch_short_and_music_urls_use_native_provider() {
    for url in [
        "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "https://youtube.com/shorts/dQw4w9WgXcQ",
        "https://youtu.be/dQw4w9WgXcQ",
        "https://music.youtube.com/watch?v=dQw4w9WgXcQ",
    ] {
        let request = classify_native_playback_url(url).expect("youtube native provider");
        assert_eq!(request.provider, NativePlaybackProvider::YouTube);
        assert_eq!(request.original_url, url);
        assert!(!request.resource.is_empty());
    }
}

#[test]
fn twitch_live_channels_use_native_provider_but_vods_stay_web() {
    let url = "https://www.twitch.tv/monstercat";
    let request = classify_native_playback_url(url).expect("twitch native provider");
    assert_eq!(request.provider, NativePlaybackProvider::Twitch);
    assert_eq!(request.original_url, url);
    assert_eq!(request.resource, "monstercat");

    assert!(classify_native_playback_url("https://twitch.tv/videos/1234567890").is_none());
}

#[test]
fn ordinary_web_urls_are_not_native_playback_requests() {
    assert!(classify_native_playback_url("https://servo.org/").is_none());
    assert!(classify_native_playback_url("https://example.com/watch?v=x").is_none());
}

#[test]
fn explicit_web_override_disables_native_provider() {
    assert!(
        classify_native_playback_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ&aether_web=1")
            .is_none()
    );
    assert!(
        classify_native_playback_url("https://music.youtube.com/watch?v=dQw4w9WgXcQ&aether_web=1")
            .is_none()
    );
}

#[test]
fn twitch_login_and_auth_pages_remain_web_content() {
    for url in [
        "https://www.twitch.tv/login",
        "https://www.twitch.tv/signup",
        "https://www.twitch.tv/oauth2/authorize",
    ] {
        assert!(
            classify_native_playback_url(url).is_none(),
            "{url} must remain Chromium web content"
        );
    }
}
