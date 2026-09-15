#![forbid(unsafe_code)]
//! Unified communications contracts. Authentication material is always a secret-store reference.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecretRef(pub String);

impl SecretRef {
    #[must_use]
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommsProviderKind {
    GenericMail,
    ZohoMail,
    ProtonMail,
    Twitch,
    Bluesky,
    Streamlabs,
    StreamElements,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommsTransportKind {
    ImapSmtp,
    OAuthApi,
    OfficialBridge,
    OfficialWeb,
    RealtimeProvider,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MailTransportConfig {
    pub incoming_host: String,
    pub incoming_port: u16,
    pub outgoing_host: String,
    pub outgoing_port: u16,
    pub tls_required: bool,
    pub auth_secret: SecretRef,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapturePrivacy {
    HidePrivateWhileLive,
    AllowSelectedFields,
    ShowAll,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnifiedActivity {
    Mail {
        provider: CommsProviderKind,
        sender: String,
        subject: String,
    },
    DirectMessage {
        provider: CommsProviderKind,
        sender: String,
    },
    CreatorEvent {
        provider: CommsProviderKind,
        label: String,
    },
    Notification {
        provider: CommsProviderKind,
        label: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommsAccount {
    pub provider: CommsProviderKind,
    pub display_name: String,
    pub transport: CommsTransportKind,
    pub credential_ref: SecretRef,
    pub privacy: CapturePrivacy,
}

impl CommsAccount {
    #[must_use]
    pub fn generic_imap_smtp(display_name: impl Into<String>, credential_ref: SecretRef) -> Self {
        Self {
            provider: CommsProviderKind::GenericMail,
            display_name: display_name.into(),
            transport: CommsTransportKind::ImapSmtp,
            credential_ref,
            privacy: CapturePrivacy::HidePrivateWhileLive,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UnifiedInbox {
    pub accounts: Vec<CommsAccount>,
    pub activity: Vec<UnifiedActivity>,
}

impl UnifiedInbox {
    #[must_use]
    pub fn capture_safe_activity(&self, stream_live: bool) -> Vec<&UnifiedActivity> {
        if !stream_live {
            return self.activity.iter().collect();
        }
        self.activity
            .iter()
            .filter(|item| matches!(item, UnifiedActivity::CreatorEvent { .. }))
            .collect()
    }
}

pub const ZOHO_MAIL_WEB_URL: &str = "https://mail.zoho.com/";
pub const ZOHO_MAIL_API_BASE: &str = "https://mail.zoho.com/api";
pub const ZOHO_IMAP_HOST: &str = "imap.zoho.com";
pub const ZOHO_IMAP_PORT: u16 = 993;
pub const ZOHO_SMTP_HOST: &str = "smtp.zoho.com";
pub const ZOHO_SMTP_PORT: u16 = 465;
pub const ZOHO_MAIL_SCOPES: &[&str] = &[
    "ZohoMail.accounts.READ",
    "ZohoMail.folders.READ",
    "ZohoMail.messages.READ",
    "ZohoMail.messages.CREATE",
    "ZohoMail.messages.UPDATE",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ZohoDataCenter {
    UnitedStates,
    Europe,
    India,
    Australia,
    Japan,
    Canada,
    SaudiArabia,
}

impl ZohoDataCenter {
    #[must_use]
    pub const fn accounts_base(self) -> &'static str {
        match self {
            Self::UnitedStates => "https://accounts.zoho.com",
            Self::Europe => "https://accounts.zoho.eu",
            Self::India => "https://accounts.zoho.in",
            Self::Australia => "https://accounts.zoho.com.au",
            Self::Japan => "https://accounts.zoho.jp",
            Self::Canada => "https://accounts.zohocloud.ca",
            Self::SaudiArabia => "https://accounts.zoho.sa",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZohoOAuthConfig {
    pub data_center: ZohoDataCenter,
    pub client_id_ref: SecretRef,
    pub client_secret_ref: SecretRef,
    pub token_ref: SecretRef,
}

impl ZohoOAuthConfig {
    #[must_use]
    pub fn token_endpoint(&self) -> String {
        format!("{}/oauth/v2/token", self.data_center.accounts_base())
    }
}

#[must_use]
pub fn zoho_mail_account(display_name: impl Into<String>) -> CommsAccount {
    CommsAccount {
        provider: CommsProviderKind::ZohoMail,
        display_name: display_name.into(),
        transport: CommsTransportKind::OAuthApi,
        credential_ref: SecretRef::new("comms/zoho/oauth-session"),
        privacy: CapturePrivacy::HidePrivateWhileLive,
    }
}

#[must_use]
pub fn zoho_imap_smtp_fallback() -> MailTransportConfig {
    MailTransportConfig {
        incoming_host: ZOHO_IMAP_HOST.to_owned(),
        incoming_port: ZOHO_IMAP_PORT,
        outgoing_host: ZOHO_SMTP_HOST.to_owned(),
        outgoing_port: ZOHO_SMTP_PORT,
        tls_required: true,
        auth_secret: SecretRef::new("comms/zoho/imap-smtp-auth"),
    }
}

pub const PROTON_MAIL_WEB_URL: &str = "https://mail.proton.me/";
pub const PROTON_BRIDGE_URL: &str = "https://proton.me/mail/bridge";
pub const PROTON_BRIDGE_HOST: &str = "127.0.0.1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtonBridgeRequirement {
    PaidPlanRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtonBridgeState {
    NotInstalled,
    Installed { binary: String },
    Running { binary: String },
    NeedsAccountSignIn,
    Ready,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtonBridgeEndpoint {
    pub host: &'static str,
    pub imap_port: Option<u16>,
    pub smtp_port: Option<u16>,
    pub local_credential_ref: SecretRef,
}

impl Default for ProtonBridgeEndpoint {
    fn default() -> Self {
        Self {
            host: PROTON_BRIDGE_HOST,
            imap_port: None,
            smtp_port: None,
            local_credential_ref: SecretRef::new("comms/proton/bridge-local-credentials"),
        }
    }
}

#[must_use]
pub fn detect_proton_bridge_binary() -> Option<String> {
    [
        "/usr/bin/protonmail-bridge",
        "/usr/bin/protonmail-bridge-gui",
        "/usr/local/bin/protonmail-bridge",
    ]
    .into_iter()
    .find(|path| std::path::Path::new(path).is_file())
    .map(str::to_owned)
}

#[must_use]
pub fn proton_mail_account(display_name: impl Into<String>) -> CommsAccount {
    CommsAccount {
        provider: CommsProviderKind::ProtonMail,
        display_name: display_name.into(),
        transport: CommsTransportKind::OfficialBridge,
        credential_ref: SecretRef::new("comms/proton/bridge-local-credentials"),
        privacy: CapturePrivacy::HidePrivateWhileLive,
    }
}

pub const TWITCH_WEB_URL: &str = "https://www.twitch.tv/";
pub const BLUESKY_WEB_URL: &str = "https://bsky.app/";
pub const STREAMLABS_WEB_URL: &str = "https://streamlabs.com/";
pub const STREAMELEMENTS_WEB_URL: &str = "https://streamelements.com/";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommsProviderDescriptor {
    pub kind: CommsProviderKind,
    pub display_name: &'static str,
    pub transport: CommsTransportKind,
    pub official_web_url: Option<&'static str>,
    pub privacy: CapturePrivacy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommsProviderRegistry {
    providers: Vec<CommsProviderDescriptor>,
}

impl CommsProviderRegistry {
    #[must_use]
    pub fn canonical() -> Self {
        let private = CapturePrivacy::HidePrivateWhileLive;
        Self {
            providers: vec![
                CommsProviderDescriptor {
                    kind: CommsProviderKind::GenericMail,
                    display_name: "Generic Mail",
                    transport: CommsTransportKind::ImapSmtp,
                    official_web_url: None,
                    privacy: private,
                },
                CommsProviderDescriptor {
                    kind: CommsProviderKind::ZohoMail,
                    display_name: "Zoho Mail",
                    transport: CommsTransportKind::OAuthApi,
                    official_web_url: Some(ZOHO_MAIL_WEB_URL),
                    privacy: private,
                },
                CommsProviderDescriptor {
                    kind: CommsProviderKind::ProtonMail,
                    display_name: "Proton Mail",
                    transport: CommsTransportKind::OfficialBridge,
                    official_web_url: Some(PROTON_MAIL_WEB_URL),
                    privacy: private,
                },
                CommsProviderDescriptor {
                    kind: CommsProviderKind::Twitch,
                    display_name: "Twitch",
                    transport: CommsTransportKind::RealtimeProvider,
                    official_web_url: Some(TWITCH_WEB_URL),
                    privacy: private,
                },
                CommsProviderDescriptor {
                    kind: CommsProviderKind::Bluesky,
                    display_name: "Bluesky",
                    transport: CommsTransportKind::RealtimeProvider,
                    official_web_url: Some(BLUESKY_WEB_URL),
                    privacy: private,
                },
                CommsProviderDescriptor {
                    kind: CommsProviderKind::Streamlabs,
                    display_name: "Streamlabs",
                    transport: CommsTransportKind::RealtimeProvider,
                    official_web_url: Some(STREAMLABS_WEB_URL),
                    privacy: private,
                },
                CommsProviderDescriptor {
                    kind: CommsProviderKind::StreamElements,
                    display_name: "StreamElements",
                    transport: CommsTransportKind::RealtimeProvider,
                    official_web_url: Some(STREAMELEMENTS_WEB_URL),
                    privacy: private,
                },
            ],
        }
    }

    #[must_use]
    pub fn providers(&self) -> &[CommsProviderDescriptor] {
        &self.providers
    }

    #[must_use]
    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }
}
