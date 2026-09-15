#![forbid(unsafe_code)]
//! Capability-based Aether extension manifest boundary.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExtensionCapability {
    ContentScript,
    PageAction,
    Storage,
    DeclarativeRequestFilter,
    Tabs,
    NavigationEvents,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub capabilities: Vec<ExtensionCapability>,
}
