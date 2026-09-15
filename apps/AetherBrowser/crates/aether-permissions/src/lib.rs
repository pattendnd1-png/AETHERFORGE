#![forbid(unsafe_code)]
//! Site-permission policy contracts with sensitive capabilities asking by default.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PermissionKind {
    Camera,
    Microphone,
    Location,
    Notifications,
    ClipboardRead,
    ClipboardWrite,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PermissionDecision {
    Allow,
    Deny,
    #[default]
    Ask,
}
