use aether_native_pages::{NativePageRoute, native_page_for_url, render_native_page};

#[test]
fn native_routes_are_parsed_without_touching_public_web_urls() {
    assert_eq!(
        NativePageRoute::from_url("aether://stream"),
        Some(NativePageRoute::StreamStudio)
    );
    assert_eq!(NativePageRoute::from_url("https://twitch.tv/"), None);
}

#[test]
fn stream_studio_and_vault_are_visible_first_party_surfaces() {
    let stream = render_native_page(NativePageRoute::StreamStudio);
    assert!(stream.html.contains("Aether Stream Studio"));
    assert!(stream.html.contains("OBS WebSocket"));
    assert!(
        stream
            .html
            .contains("Aether Studio // BrowserAuthoritative")
    );
    assert!(stream.html.contains("Scenes"));
    assert!(stream.html.contains("Mixer"));
    let vault = render_native_page(NativePageRoute::Vault);
    assert!(vault.html.contains("Aether Vault"));
    assert!(vault.html.contains("XChaCha20-Poly1305"));
}

#[test]
fn account_and_provider_pages_include_requested_integrations() {
    let accounts = native_page_for_url("aether://accounts").expect("native accounts page");
    assert!(accounts.html.contains("Sign in with Twitch"));
    assert!(accounts.html.contains("Sign in with Google"));
    assert!(accounts.html.contains("Sign in with Apple"));
    let providers = render_native_page(NativePageRoute::Providers);
    for name in [
        "OBS Studio",
        "Streamlabs",
        "StreamElements",
        "Twitch",
        "YouTube Live",
    ] {
        assert!(providers.html.contains(name));
    }
}

#[test]
fn native_watch_page_is_dark_and_never_uses_servo_html_video() {
    let page = native_page_for_url(
        "aether://watch?provider=youtube&resource=dQw4w9WgXcQ&url=https%3A%2F%2Fwww.youtube.com%2Fwatch%3Fv%3DdQw4w9WgXcQ",
    )
    .expect("native watch page");
    assert!(page.html.contains("Aether Native Player"));
    assert!(page.html.contains("id=\"aether-native-video\""));
    assert!(page.html.contains("color-scheme:dark"));
    assert!(!page.html.contains("<video"));
}

#[test]
fn creator_hub_exposes_twitch_without_embedding_tokens() {
    let page = native_page_for_url("aether://creator").expect("creator page");
    assert_eq!(page.route, NativePageRoute::Creator);
    assert!(page.html.contains("Twitch Creator"));
    assert!(page.html.contains("EventSub"));
    assert!(!page.html.contains("client_secret"));
}

#[test]
fn comms_hub_is_private_by_default_while_live() {
    let page = native_page_for_url("aether://comms").expect("comms page");
    assert_eq!(page.route, NativePageRoute::Comms);
    assert!(page.html.contains("Aether Comms"));
    assert!(page.html.contains("HidePrivateWhileLive"));
}

#[test]
fn library_is_a_native_stateful_dragonglass_surface() {
    use aether_native_pages::native_page_for_url_with_library;
    use aether_storage::{BookmarkRow, HistoryRow, LibrarySnapshot, RecentlyClosedRow};
    let snapshot = LibrarySnapshot {
        history: vec![HistoryRow {
            id: 1,
            url: "https://servo.org/".into(),
            title: "Servo".into(),
            visited_unix_seconds: 1,
        }],
        bookmarks: vec![BookmarkRow {
            id: 2,
            url: "https://example.com/".into(),
            title: "Example".into(),
            folder: "Favorites".into(),
            tags: vec!["test".into()],
            favorite: true,
            created_unix_seconds: 1,
            updated_unix_seconds: 1,
        }],
        downloads: Vec::new(),
        recently_closed: vec![RecentlyClosedRow {
            id: 3,
            url: "https://closed.invalid/".into(),
            title: "Closed".into(),
            closed_unix_seconds: 1,
        }],
    };
    let page = native_page_for_url_with_library("aether://library?q=servo", &snapshot)
        .expect("library page");
    assert_eq!(page.route, NativePageRoute::Library);
    assert!(page.html.contains("Aether Library"));
    assert!(page.html.contains("History"));
    assert!(page.html.contains("Bookmarks"));
    assert!(page.html.contains("Downloads"));
    assert!(page.html.contains("Recently Closed"));
    assert!(page.html.contains("Servo"));
}
