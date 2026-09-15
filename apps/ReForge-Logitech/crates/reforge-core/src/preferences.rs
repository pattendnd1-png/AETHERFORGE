use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiPreferences {
    pub selected_device_key: Option<String>,
    pub selected_profile_name: Option<String>,
    pub page: String,
    pub theme: String,
    pub live_preview: bool,
    pub window_width: Option<u32>,
    pub window_height: Option<u32>,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            selected_device_key: None,
            selected_profile_name: None,
            page: "dashboard".into(),
            theme: "system".into(),
            live_preview: true,
            window_width: None,
            window_height: None,
        }
    }
}

impl UiPreferences {
    pub fn load() -> Result<Self, String> {
        Self::load_from(&crate::paths::ui_preferences_file())
    }

    pub fn load_or_default() -> (Self, Option<String>) {
        match Self::load() {
            Ok(value) => (value, None),
            Err(error) => (Self::default(), Some(error)),
        }
    }

    pub fn load_from(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        serde_json::from_str(&text)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))
    }

    pub fn save(&self) -> Result<(), String> {
        self.save_to(&crate::paths::ui_preferences_file())
    }

    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        let parent = path.parent().ok_or_else(|| format!("invalid preference path {}", path.display()))?;
        fs::create_dir_all(parent).map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(self).map_err(|error| format!("failed to encode UI preferences: {error}"))?;
        fs::write(&tmp, text).map_err(|error| format!("failed to write {}: {error}", tmp.display()))?;
        fs::rename(&tmp, path).map_err(|error| format!("failed to replace {}: {error}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("reforge-{name}-{}-{}.json", std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()))
    }

    #[test]
    fn missing_file_uses_defaults() {
        let path = temp_file("missing");
        assert_eq!(UiPreferences::load_from(&path).unwrap(), UiPreferences::default());
    }

    #[test]
    fn preferences_round_trip() {
        let path = temp_file("prefs");
        let prefs = UiPreferences {
            selected_device_key: Some("hidpp:abc".into()), selected_profile_name: Some("Game".into()),
            page: "diagnostics".into(), theme: "dark".into(), live_preview: false,
            window_width: Some(1200), window_height: Some(800),
        };
        prefs.save_to(&path).unwrap();
        assert_eq!(UiPreferences::load_from(&path).unwrap(), prefs);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn corrupt_file_is_an_error() {
        let path = temp_file("corrupt");
        fs::write(&path, "{").unwrap();
        assert!(UiPreferences::load_from(&path).is_err());
        let _ = fs::remove_file(path);
    }
}
