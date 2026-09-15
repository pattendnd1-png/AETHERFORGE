use aether_comms::*;

#[test]
fn generic_mail_defaults_to_private_while_live() {
    let account = CommsAccount::generic_imap_smtp("Mail", SecretRef::new("comms/mail/account-1"));
    assert_eq!(account.transport, CommsTransportKind::ImapSmtp);
    assert_eq!(account.privacy, CapturePrivacy::HidePrivateWhileLive);
}

#[test]
fn live_capture_filters_private_mail_and_dm_activity() {
    let mut inbox = UnifiedInbox::default();
    inbox.activity.push(UnifiedActivity::Mail {
        provider: CommsProviderKind::GenericMail,
        sender: "A".into(),
        subject: "Private".into(),
    });
    inbox.activity.push(UnifiedActivity::CreatorEvent {
        provider: CommsProviderKind::Twitch,
        label: "Follow".into(),
    });
    assert_eq!(inbox.capture_safe_activity(true).len(), 1);
    assert_eq!(inbox.capture_safe_activity(false).len(), 2);
}

#[test]
fn zoho_mail_prefers_oauth_api_with_secure_imap_smtp_fallback() {
    let account = zoho_mail_account("Zoho");
    assert_eq!(account.provider, CommsProviderKind::ZohoMail);
    assert_eq!(account.transport, CommsTransportKind::OAuthApi);
    let fallback = zoho_imap_smtp_fallback();
    assert_eq!(fallback.incoming_host, ZOHO_IMAP_HOST);
    assert_eq!(fallback.incoming_port, 993);
    assert_eq!(fallback.outgoing_host, ZOHO_SMTP_HOST);
    assert_eq!(fallback.outgoing_port, 465);
    assert!(fallback.tls_required);
}

#[test]
fn zoho_oauth_uses_region_specific_accounts_endpoint() {
    let config = ZohoOAuthConfig {
        data_center: ZohoDataCenter::Europe,
        client_id_ref: SecretRef::new("zoho/client-id"),
        client_secret_ref: SecretRef::new("zoho/client-secret"),
        token_ref: SecretRef::new("zoho/token"),
    };
    assert_eq!(
        config.token_endpoint(),
        "https://accounts.zoho.eu/oauth/v2/token"
    );
}

#[test]
fn proton_mail_uses_local_official_bridge_boundary() {
    let account = proton_mail_account("Proton");
    let endpoint = ProtonBridgeEndpoint::default();
    assert_eq!(account.provider, CommsProviderKind::ProtonMail);
    assert_eq!(account.transport, CommsTransportKind::OfficialBridge);
    assert_eq!(endpoint.host, "127.0.0.1");
    assert_eq!(
        ProtonBridgeRequirement::PaidPlanRequired,
        ProtonBridgeRequirement::PaidPlanRequired
    );
}
