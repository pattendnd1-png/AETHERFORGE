use aether_identity::{
    AetherIdentityProfile, ExternalIdentity, IdentityLinkError, IdentityProvider, link_identity,
};

#[test]
fn duplicate_provider_subject_is_rejected() {
    let mut profile = AetherIdentityProfile::new("profile-1");
    let identity = ExternalIdentity::new(IdentityProvider::Twitch, "twitch-subject");
    link_identity(&mut profile, identity.clone()).unwrap();
    assert_eq!(
        link_identity(&mut profile, identity),
        Err(IdentityLinkError::AlreadyLinked)
    );
}

#[test]
fn same_email_does_not_auto_merge_distinct_providers() {
    let mut twitch = ExternalIdentity::new(IdentityProvider::Twitch, "tw-1");
    twitch.email = Some("person@example.com".into());
    let mut google = ExternalIdentity::new(IdentityProvider::Google, "go-1");
    google.email = Some("person@example.com".into());
    let mut profile = AetherIdentityProfile::new("profile-1");
    link_identity(&mut profile, twitch).unwrap();
    link_identity(&mut profile, google).unwrap();
    assert_eq!(profile.linked_identities().len(), 2);
}

#[test]
fn apple_and_local_are_supported_identity_methods() {
    let mut profile = AetherIdentityProfile::new("profile-1");
    link_identity(
        &mut profile,
        ExternalIdentity::new(IdentityProvider::Apple, "apple-subject"),
    )
    .unwrap();
    link_identity(
        &mut profile,
        ExternalIdentity::new(IdentityProvider::Local, "local-subject"),
    )
    .unwrap();
    assert_eq!(profile.linked_identities().len(), 2);
}
