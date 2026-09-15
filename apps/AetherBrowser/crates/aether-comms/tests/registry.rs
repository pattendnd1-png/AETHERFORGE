use aether_comms::{CapturePrivacy, CommsProviderKind, CommsProviderRegistry};

#[test]
fn canonical_registry_contains_every_communications_provider() {
    let registry = CommsProviderRegistry::canonical();
    assert_eq!(registry.provider_count(), 7);
    let kinds = registry
        .providers()
        .iter()
        .map(|provider| provider.kind)
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            CommsProviderKind::GenericMail,
            CommsProviderKind::ZohoMail,
            CommsProviderKind::ProtonMail,
            CommsProviderKind::Twitch,
            CommsProviderKind::Bluesky,
            CommsProviderKind::Streamlabs,
            CommsProviderKind::StreamElements,
        ]
    );
    assert!(
        registry
            .providers()
            .iter()
            .all(|provider| { provider.privacy == CapturePrivacy::HidePrivateWhileLive })
    );
}
