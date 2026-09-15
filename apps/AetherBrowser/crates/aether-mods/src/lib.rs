#![forbid(unsafe_code)]
//! Capability-based Aether Mods manifest contracts with deny-by-default permissions.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModPermission {
    ThemeTokens,
    Wallpaper,
    Icons,
    Cursor,
    BrowserSounds,
    BackgroundMusic,
    PageShader,
    LayoutExtension,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModManifest {
    id: String,
    name: String,
    version: String,
    permissions: Vec<ModPermission>,
}

impl ModManifest {
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            permissions: Vec::new(),
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }
    #[must_use]
    pub fn permissions(&self) -> &[ModPermission] {
        &self.permissions
    }

    pub fn grant(&mut self, permission: ModPermission) {
        if !self.permissions.contains(&permission) {
            self.permissions.push(permission);
        }
    }
}
