#![forbid(unsafe_code)]
//! Browser profile and private-session identity contracts.

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ProfileId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileKind {
    Persistent,
    PrivateEphemeral,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Profile {
    pub id: ProfileId,
    pub name: String,
    pub kind: ProfileKind,
}

#[derive(Debug)]
pub struct ProfileStorage {
    root: PathBuf,
    ephemeral: bool,
}

impl ProfileStorage {
    pub fn for_kind(kind: ProfileKind) -> io::Result<Self> {
        match kind {
            ProfileKind::Persistent => {
                let root = persistent_profile_root();
                fs::create_dir_all(&root)?;
                Ok(Self {
                    root,
                    ephemeral: false,
                })
            }
            ProfileKind::PrivateEphemeral => {
                let nonce = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                let root = env::temp_dir().join(format!(
                    "aether-browser-private-{}-{nonce}",
                    std::process::id()
                ));
                fs::create_dir_all(&root)?;
                Ok(Self {
                    root,
                    ephemeral: true,
                })
            }
        }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub const fn is_ephemeral(&self) -> bool {
        self.ephemeral
    }

    /// Stable compatibility-engine storage. Normal sessions reuse the same
    /// directory across window close, reboot, and upgrades; private sessions
    /// remain inside the already-ephemeral profile root.
    #[must_use]
    pub fn compat_root(&self) -> PathBuf {
        let root = if self.ephemeral {
            self.root.join("profiles/compat")
        } else {
            persistent_compat_profile_root()
        };
        let _ = fs::create_dir_all(&root);
        root
    }
}

impl Drop for ProfileStorage {
    fn drop(&mut self) {
        if self.ephemeral {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

fn persistent_compat_profile_root() -> PathBuf {
    if let Some(data_home) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(data_home).join("aetherforge/aether-browser/profiles/compat");
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".local/share/aetherforge/aether-browser/profiles/compat");
    }
    env::temp_dir().join("aetherforge/aether-browser/profiles/compat")
}

fn persistent_profile_root() -> PathBuf {
    if let Some(data_home) = env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(data_home).join("aetherforge/aether-browser/profiles/default");
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".local/share/aetherforge/aether-browser/profiles/default");
    }
    env::temp_dir().join("aetherforge/aether-browser/profiles/default")
}

#[cfg(test)]
mod tests {
    use super::{ProfileKind, ProfileStorage};

    #[test]
    fn private_profiles_are_ephemeral_and_unique() {
        let first = ProfileStorage::for_kind(ProfileKind::PrivateEphemeral).unwrap();
        let second = ProfileStorage::for_kind(ProfileKind::PrivateEphemeral).unwrap();
        assert!(first.is_ephemeral());
        assert!(second.is_ephemeral());
        assert_ne!(first.root(), second.root());
    }

    #[test]
    fn persistent_profile_is_not_ephemeral() {
        let profile = ProfileStorage::for_kind(ProfileKind::Persistent).unwrap();
        assert!(!profile.is_ephemeral());
        assert!(
            profile
                .root()
                .ends_with("aetherforge/aether-browser/profiles/default")
        );
        assert!(
            profile
                .compat_root()
                .ends_with("aetherforge/aether-browser/profiles/compat")
        );
    }

    #[test]
    fn private_compat_profile_stays_under_ephemeral_root() {
        let profile = ProfileStorage::for_kind(ProfileKind::PrivateEphemeral).unwrap();
        let compat = profile.compat_root();
        assert!(compat.starts_with(profile.root()));
        assert!(compat.ends_with("profiles/compat"));
    }
}
