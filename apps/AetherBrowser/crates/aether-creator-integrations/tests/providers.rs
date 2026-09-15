use aether_creator_integrations::*;

#[test]
fn twitch_descriptor_uses_official_oauth_helix_and_eventsub() {
    let provider = twitch_descriptor();
    assert_eq!(provider.kind, CreatorProviderKind::Twitch);
    assert_eq!(provider.auth_url, TWITCH_AUTH_URL);
    assert_eq!(provider.api_base, TWITCH_API_BASE);
    assert_eq!(provider.realtime_url, Some(TWITCH_EVENTSUB_WEBSOCKET));
    assert!(provider.capabilities.contains(&CreatorCapability::Chat));
    assert!(
        provider
            .capabilities
            .contains(&CreatorCapability::Moderation)
    );
}

#[test]
fn oauth_url_encodes_requested_scopes_without_embedding_a_secret() {
    let url = twitch_authorize_url(
        "client",
        "http://127.0.0.1/callback",
        "csrf",
        &["chat:read", "clips:edit"],
    )
    .unwrap();
    assert!(url.as_str().starts_with(TWITCH_AUTH_URL));
    assert!(url.query().unwrap().contains("client_id=client"));
    assert!(!url.as_str().contains("client_secret"));
}

#[test]
fn streamlabs_descriptor_preserves_official_oauth_api_and_socket_boundaries() {
    let provider = streamlabs_descriptor();
    assert_eq!(provider.kind, CreatorProviderKind::Streamlabs);
    assert_eq!(provider.auth_url, STREAMLABS_AUTH_URL);
    assert_eq!(provider.api_base, STREAMLABS_API_BASE);
    assert_eq!(provider.realtime_url, Some(STREAMLABS_SOCKET_URL));
    assert!(provider.capabilities.contains(&CreatorCapability::Alerts));
    assert!(
        provider
            .capabilities
            .contains(&CreatorCapability::Donations)
    );
    assert!(
        provider
            .capabilities
            .contains(&CreatorCapability::MediaShare)
    );
}

#[test]
fn streamelements_uses_astro_and_typed_topics() {
    let provider = stream_elements_descriptor();
    assert_eq!(provider.kind, CreatorProviderKind::StreamElements);
    assert_eq!(provider.realtime_url, Some(STREAMELEMENTS_ASTRO_URL));
    assert!(STREAMELEMENTS_TOPICS.contains(&"channel.activities"));
    assert!(STREAMELEMENTS_TOPICS.contains(&"channel.overlay.broadcast"));
    assert!(provider.capabilities.contains(&CreatorCapability::Overlays));
    assert!(provider.capabilities.contains(&CreatorCapability::Chatbot));
}

#[test]
fn canonical_registry_uses_native_stream_studio_as_telemetry_owner() {
    let registry = CreatorProviderRegistry::canonical();
    assert_eq!(registry.providers().len(), 4);
    assert_eq!(
        registry.telemetry_source(),
        CreatorTelemetrySource::NativeStreamStudio
    );
}

#[test]
fn bluesky_uses_atproto_oauth_profile_without_client_secret() {
    let provider = bluesky_descriptor();
    let oauth = BlueskyOAuthProfile::canonical();
    assert_eq!(provider.kind, CreatorProviderKind::Bluesky);
    assert!(oauth.requires_pkce && oauth.requires_par && oauth.requires_dpop);
    assert!(!oauth.client_secret_allowed);
    assert!(
        BLUESKY_LEXICONS
            .iter()
            .any(|value| value.starts_with("app.bsky"))
    );
    assert!(
        BLUESKY_LEXICONS
            .iter()
            .any(|value| value.starts_with("chat.bsky"))
    );
}
