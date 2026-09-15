use serde::{Deserialize, Serialize};
use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

use crate::ProcessSnapshot;

const ACCOUNT_PROFILE_SCHEMA: u32 = 1;
pub const OFFICIAL_ACCOUNT_URL: &str = "https://account.battle.net/";
pub const OAUTH_CLIENT_ID_ENV: &str = "OPENSANCTUARY_BNET_CLIENT_ID";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AccountProfile {
    pub schema_version: u32,
    pub email_hint: Option<String>,
    pub battletag: Option<String>,
    pub region: String,
}

impl Default for AccountProfile {
    fn default() -> Self {
        Self {
            schema_version: ACCOUNT_PROFILE_SCHEMA,
            email_hint: None,
            battletag: None,
            region: "us".into(),
        }
    }
}

impl AccountProfile {
    pub fn sanitize(&mut self) {
        self.schema_version = ACCOUNT_PROFILE_SCHEMA;
        self.email_hint = clean_optional(self.email_hint.take());
        self.battletag = clean_optional(self.battletag.take());
        self.region = normalize_region(&self.region);
    }

    pub fn masked_email(&self) -> Option<String> {
        let email = self.email_hint.as_deref()?;
        let (local, domain) = email.split_once('@')?;
        let first = local.chars().next()?;
        Some(format!("{first}••••••@{domain}"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopSessionState {
    Stopped,
    ClientRunning,
    ClientAndAgentRunning,
}

impl DesktopSessionState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Stopped => "Not running",
            Self::ClientRunning => "Battle.net open",
            Self::ClientAndAgentRunning => "Battle.net + Agent ready",
        }
    }

    pub fn interactive(self) -> bool {
        !matches!(self, Self::Stopped)
    }
}

#[derive(Debug, Error)]
pub enum AccountError {
    #[error("unsupported account profile schema {0}")]
    UnsupportedSchema(u32),
    #[error("failed to decode account profile: {0}")]
    Decode(String),
    #[error("failed to encode account profile: {0}")]
    Encode(String),
    #[error("account profile I/O failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub fn account_profile_path(home: &Path, xdg_config_home: Option<&OsStr>) -> PathBuf {
    xdg_config_home.map_or_else(
        || home.join(".config/opensanctuary/account.toml"),
        |root| PathBuf::from(root).join("opensanctuary/account.toml"),
    )
}

pub fn default_account_profile_path() -> PathBuf {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    account_profile_path(&home, env::var_os("XDG_CONFIG_HOME").as_deref())
}

pub fn load_account_profile(path: &Path) -> Result<AccountProfile, AccountError> {
    if !path.exists() {
        return Ok(AccountProfile::default());
    }
    let text = fs::read_to_string(path).map_err(|source| AccountError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut profile: AccountProfile =
        toml::from_str(&text).map_err(|error| AccountError::Decode(error.to_string()))?;
    if profile.schema_version != ACCOUNT_PROFILE_SCHEMA {
        return Err(AccountError::UnsupportedSchema(profile.schema_version));
    }
    profile.sanitize();
    Ok(profile)
}

pub fn save_account_profile(path: &Path, profile: &AccountProfile) -> Result<(), AccountError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| AccountError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let mut sanitized = profile.clone();
    sanitized.sanitize();
    let text = toml::to_string_pretty(&sanitized)
        .map_err(|error| AccountError::Encode(error.to_string()))?;
    let temporary = path.with_extension("toml.tmp");
    fs::write(&temporary, text).map_err(|source| AccountError::Io {
        path: temporary.clone(),
        source,
    })?;
    fs::rename(&temporary, path).map_err(|source| AccountError::Io {
        path: path.to_path_buf(),
        source,
    })
}

pub fn account_state_from_processes(snapshot: ProcessSnapshot) -> DesktopSessionState {
    match (snapshot.battlenet_running, snapshot.agent_running) {
        (true, true) => DesktopSessionState::ClientAndAgentRunning,
        (true, false) | (false, true) => DesktopSessionState::ClientRunning,
        (false, false) => DesktopSessionState::Stopped,
    }
}

pub fn oauth_client_id() -> Option<String> {
    env::var(OAUTH_CLIENT_ID_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn official_account_url() -> &'static str {
    OFFICIAL_ACCOUNT_URL
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_region(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "eu" => "eu".into(),
        "kr" => "kr".into(),
        "tw" => "tw".into(),
        _ => "us".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn xdg_profile_path_is_scoped_to_opensanctuary() {
        let home = Path::new("/home/tester");
        assert_eq!(
            account_profile_path(home, Some(OsStr::new("/tmp/config"))),
            PathBuf::from("/tmp/config/opensanctuary/account.toml")
        );
        assert_eq!(
            account_profile_path(home, None),
            PathBuf::from("/home/tester/.config/opensanctuary/account.toml")
        );
    }

    #[test]
    fn profile_round_trip_contains_only_non_secret_fields() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("account.toml");
        let profile = AccountProfile {
            email_hint: Some("player@example.com".into()),
            battletag: Some("Player#1234".into()),
            region: "EU".into(),
            ..AccountProfile::default()
        };
        save_account_profile(&path, &profile).unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        let lowered = raw.to_ascii_lowercase();
        assert!(!lowered.contains("password"));
        assert!(!lowered.contains("token"));
        assert!(!lowered.contains("secret"));
        assert!(!lowered.contains("authenticator"));
        let loaded = load_account_profile(&path).unwrap();
        assert_eq!(loaded.region, "eu");
        assert_eq!(
            loaded.masked_email().as_deref(),
            Some("p••••••@example.com")
        );
    }

    #[test]
    fn session_state_comes_only_from_supervised_processes() {
        assert_eq!(
            account_state_from_processes(ProcessSnapshot {
                battlenet_running: false,
                agent_running: false,
                diablo_running: false,
            }),
            DesktopSessionState::Stopped
        );
        assert_eq!(
            account_state_from_processes(ProcessSnapshot {
                battlenet_running: true,
                agent_running: true,
                diablo_running: false,
            }),
            DesktopSessionState::ClientAndAgentRunning
        );
    }
}
