use aether_stream_providers::{
    ProviderAuthMode, ProviderRegistry, ProviderRegistryError, built_in_providers,
};

#[test]
fn canonical_provider_set_is_present() {
    let providers = built_in_providers();
    let ids: Vec<_> = providers
        .iter()
        .map(|provider| provider.id.as_str())
        .collect();
    for expected in [
        "twitch",
        "youtube-live",
        "velora",
        "obs",
        "streamlabs",
        "streamelements",
        "kick",
        "custom-rtmp",
        "custom-srt",
    ] {
        assert!(ids.contains(&expected), "missing {expected}");
    }
}

#[test]
fn provider_auth_modes_are_isolated_and_explicit() {
    let providers = built_in_providers();
    let auth = |id: &str| {
        providers
            .iter()
            .find(|provider| provider.id == id)
            .unwrap()
            .auth
    };
    assert_eq!(auth("youtube-live"), ProviderAuthMode::GoogleOAuth);
    assert_eq!(auth("velora"), ProviderAuthMode::WebSession);
    assert_eq!(auth("obs"), ProviderAuthMode::LocalIpc);
    assert_eq!(auth("streamlabs"), ProviderAuthMode::TwitchDelegated);
    assert_eq!(auth("streamelements"), ProviderAuthMode::TwitchDelegated);
}

#[test]
fn registry_rejects_duplicate_provider_ids() {
    let mut registry = ProviderRegistry::default();
    let twitch = built_in_providers().into_iter().next().unwrap();
    registry.install(twitch.clone()).unwrap();
    assert_eq!(
        registry.install(twitch),
        Err(ProviderRegistryError::DuplicateId)
    );
}
