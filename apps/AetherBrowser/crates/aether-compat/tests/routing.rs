use aether_compat::{CompatibilityGeometry, ContentEngine, classify_content_engine};

#[test]
fn provider_heavy_origins_use_compatibility_engine() {
    for url in [
        "https://www.youtube.com/watch?v=abc",
        "https://music.youtube.com/",
        "https://m.youtube.com/watch?v=abc",
        "https://www.twitch.tv/example",
        "https://accounts.google.com/",
        "https://streamlabs.com/dashboard",
        "https://streamelements.com/dashboard",
    ] {
        assert_eq!(
            classify_content_engine(url),
            ContentEngine::Compatibility,
            "{url}"
        );
    }
}

#[test]
fn http_and_https_use_chromium_engine() {
    assert_eq!(
        classify_content_engine("aether://home"),
        ContentEngine::Native
    );
    for url in [
        "https://example.org/",
        "http://example.net/path",
        "https://docs.rust-lang.org/",
    ] {
        assert_eq!(
            classify_content_engine(url),
            ContentEngine::Compatibility,
            "{url}"
        );
    }
}

#[test]
fn compatibility_geometry_is_explicit_and_stable() {
    let geometry = CompatibilityGeometry::new(52, 108, 1132, 630);
    assert_eq!(geometry.x, 52);
    assert_eq!(geometry.y, 108);
    assert_eq!(geometry.width, 1132);
    assert_eq!(geometry.height, 630);
}
