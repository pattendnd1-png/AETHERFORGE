#![forbid(unsafe_code)]
//! Opaque references to credentials held by a host secret service.

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SecretKeyRef(String);

impl SecretKeyRef {
    #[must_use]
    pub fn new(reference: impl Into<String>) -> Self {
        Self(reference.into())
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
