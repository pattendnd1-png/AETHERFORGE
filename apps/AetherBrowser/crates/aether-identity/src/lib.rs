#![forbid(unsafe_code)]
//! Multi-provider Aether profile identity contracts.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IdentityProvider {
    Twitch,
    Google,
    Apple,
    Local,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalIdentity {
    pub provider: IdentityProvider,
    pub subject: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
}

impl ExternalIdentity {
    #[must_use]
    pub fn new(provider: IdentityProvider, subject: impl Into<String>) -> Self {
        Self {
            provider,
            subject: subject.into(),
            display_name: None,
            email: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AetherIdentityProfile {
    pub profile_id: String,
    linked: Vec<ExternalIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IdentityLinkError {
    EmptySubject,
    AlreadyLinked,
}

impl AetherIdentityProfile {
    #[must_use]
    pub fn new(profile_id: impl Into<String>) -> Self {
        Self {
            profile_id: profile_id.into(),
            linked: Vec::new(),
        }
    }

    #[must_use]
    pub fn linked_identities(&self) -> &[ExternalIdentity] {
        &self.linked
    }

    pub fn link_identity(&mut self, identity: ExternalIdentity) -> Result<(), IdentityLinkError> {
        if identity.subject.trim().is_empty() {
            return Err(IdentityLinkError::EmptySubject);
        }
        if self.linked.iter().any(|existing| {
            existing.provider == identity.provider && existing.subject == identity.subject
        }) {
            return Err(IdentityLinkError::AlreadyLinked);
        }
        self.linked.push(identity);
        Ok(())
    }
}

pub fn link_identity(
    profile: &mut AetherIdentityProfile,
    identity: ExternalIdentity,
) -> Result<(), IdentityLinkError> {
    profile.link_identity(identity)
}
