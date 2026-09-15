use crate::{ControlValue, DeviceSummary, LightingState, paths};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub product_id: Option<u16>,
    #[serde(default)]
    pub product: Option<String>,
    pub serial: Option<String>,
    pub dpi: Option<u16>,
    #[serde(default)]
    pub lighting: Option<LightingState>,
    #[serde(default)]
    pub applications: Vec<String>,
    #[serde(default)]
    pub auto_switch: bool,
    #[serde(default)]
    pub controls: BTreeMap<String, ControlValue>,
}

impl Profile {
    pub fn matches(&self, device: &DeviceSummary) -> bool {
        if let Some(product_id) = self.product_id
            && device.product_id != product_id
        {
            return false;
        }
        if let Some(product) = &self.product
            && !device.product.eq_ignore_ascii_case(product)
        {
            return false;
        }
        if let Some(serial) = &self.serial
            && device.serial.as_deref() != Some(serial.as_str())
        {
            return false;
        }
        true
    }

    pub fn matches_processes<'a>(&self, processes: impl IntoIterator<Item = &'a str>) -> bool {
        if !self.auto_switch || self.applications.is_empty() {
            return false;
        }
        let processes: Vec<String> = processes
            .into_iter()
            .map(|name| name.trim().to_ascii_lowercase())
            .collect();
        self.applications.iter().any(|rule| {
            let rule = rule.trim().to_ascii_lowercase();
            !rule.is_empty() && processes.iter().any(|process| process == &rule)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileStore {
    pub version: u32,
    pub profiles: Vec<Profile>,
}

impl Default for ProfileStore {
    fn default() -> Self {
        Self {
            version: 4,
            profiles: Vec::new(),
        }
    }
}

impl ProfileStore {
    pub fn load() -> Result<Self, String> {
        let path = paths::profile_file();
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let mut store: Self = serde_json::from_str(&contents)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        store.version = 4;
        Ok(store)
    }

    pub fn save(&self) -> Result<(), String> {
        let path = paths::profile_file();
        let parent = path
            .parent()
            .ok_or_else(|| format!("invalid profile path {}", path.display()))?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        let tmp = path.with_extension("json.tmp");
        let json = serde_json::to_string_pretty(self)
            .map_err(|error| format!("failed to serialize profiles: {error}"))?;
        fs::write(&tmp, json)
            .map_err(|error| format!("failed to write {}: {error}", tmp.display()))?;
        fs::rename(&tmp, &path).map_err(|error| {
            format!(
                "failed to replace {} with {}: {error}",
                path.display(),
                tmp.display()
            )
        })
    }

    pub fn get(&self, name: &str) -> Option<&Profile> {
        self.profiles.iter().find(|profile| profile.name == name)
    }

    fn normalize_name(name: &str) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("profile name cannot be empty".into());
        }
        Ok(name.to_owned())
    }

    fn normalize_profile(mut profile: Profile) -> Result<Profile, String> {
        profile.name = Self::normalize_name(&profile.name)?;
        profile.applications = profile
            .applications
            .into_iter()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .collect();
        Ok(profile)
    }

    pub fn upsert(&mut self, profile: Profile) {
        let Ok(profile) = Self::normalize_profile(profile) else {
            return;
        };
        if let Some(existing) = self.profiles.iter_mut().find(|item| item.name == profile.name) {
            *existing = profile;
        } else {
            self.profiles.push(profile);
            self.profiles.sort_by(|a, b| a.name.cmp(&b.name));
        }
    }

    pub fn rename(&mut self, old: &str, new: &str) -> Result<(), String> {
        let new = Self::normalize_name(new)?;
        if old != new && self.get(&new).is_some() {
            return Err(format!("profile already exists: {new}"));
        }
        let profile = self
            .profiles
            .iter_mut()
            .find(|profile| profile.name == old)
            .ok_or_else(|| format!("profile does not exist: {old}"))?;
        profile.name = new;
        self.profiles.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> Result<Profile, String> {
        let index = self
            .profiles
            .iter()
            .position(|profile| profile.name == name)
            .ok_or_else(|| format!("profile does not exist: {name}"))?;
        Ok(self.profiles.remove(index))
    }

    pub fn clone_as(&mut self, source: &str, new: &str) -> Result<Profile, String> {
        let new = Self::normalize_name(new)?;
        if self.get(&new).is_some() {
            return Err(format!("profile already exists: {new}"));
        }
        let mut profile = self
            .get(source)
            .cloned()
            .ok_or_else(|| format!("profile does not exist: {source}"))?;
        profile.name = new;
        self.profiles.push(profile.clone());
        self.profiles.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(profile)
    }

    pub fn import_json(&mut self, json: &str, replace: bool) -> Result<Vec<String>, String> {
        let imported_profiles = if let Ok(store) = serde_json::from_str::<ProfileStore>(json) {
            store.profiles
        } else if let Ok(profiles) = serde_json::from_str::<Vec<Profile>>(json) {
            profiles
        } else if let Ok(profile) = serde_json::from_str::<Profile>(json) {
            vec![profile]
        } else {
            return Err("profile import is not valid ReForge JSON".into());
        };

        let mut normalized = Vec::with_capacity(imported_profiles.len());
        for profile in imported_profiles {
            normalized.push(Self::normalize_profile(profile)?);
        }
        let mut seen = std::collections::BTreeSet::new();
        for profile in &normalized {
            if !seen.insert(profile.name.clone()) {
                return Err(format!("duplicate profile in import: {}", profile.name));
            }
            if !replace && self.get(&profile.name).is_some() {
                return Err(format!("profile already exists: {}", profile.name));
            }
        }

        let names = normalized.iter().map(|profile| profile.name.clone()).collect::<Vec<_>>();
        for profile in normalized {
            if replace {
                self.upsert(profile);
            } else {
                self.profiles.push(profile);
            }
        }
        self.profiles.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(names)
    }

    pub fn export_json(&self, names: &[String]) -> Result<String, String> {
        let profiles = if names.is_empty() {
            self.profiles.clone()
        } else {
            let mut selected = Vec::with_capacity(names.len());
            for name in names {
                selected.push(
                    self.get(name)
                        .cloned()
                        .ok_or_else(|| format!("profile does not exist: {name}"))?,
                );
            }
            selected
        };
        serde_json::to_string_pretty(&ProfileStore { version: 4, profiles })
            .map_err(|error| format!("failed to export profiles: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_profile_matches_exact_process_names_case_insensitively() {
        let profile = Profile {
            name: "Game".into(),
            product_id: None,
            product: None,
            serial: None,
            dpi: None,
            lighting: None,
            applications: vec!["Game.exe".into()],
            auto_switch: true,
            controls: BTreeMap::new(),
        };
        assert!(profile.matches_processes(["game.exe", "steam"]));
        assert!(!profile.matches_processes(["other.exe"]));
    }

    #[test]
    fn legacy_profile_json_gets_new_fields_by_default() {
        let json = r#"{"name":"Old","product_id":123,"serial":null,"dpi":800}"#;
        let profile: Profile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.lighting, None);
        assert!(profile.applications.is_empty());
        assert!(!profile.auto_switch);
        assert!(profile.controls.is_empty());
    }
    #[test]
    fn legacy_numeric_controls_load_as_typed_values() {
        let json = r#"{"name":"Old","product_id":null,"serial":null,"dpi":null,"controls":{"hidpp:report_rate":1}}"#;
        let profile: Profile = serde_json::from_str(json).unwrap();
        assert_eq!(profile.controls.get("hidpp:report_rate"), Some(&ControlValue::Int(1)));
    }

    fn profile(name: &str) -> Profile {
        Profile {
            name: name.into(), product_id: None, product: None, serial: None, dpi: None, lighting: None,
            applications: vec![], auto_switch: false, controls: BTreeMap::new(),
        }
    }

    #[test]
    fn profile_lifecycle_is_name_based() {
        let mut store = ProfileStore::default();
        store.upsert(profile("Alpha"));
        store.rename("Alpha", "Beta").unwrap();
        assert!(store.get("Beta").is_some());
        let cloned = store.clone_as("Beta", "Gamma").unwrap();
        assert_eq!(cloned.name, "Gamma");
        assert!(store.clone_as("Beta", "Gamma").is_err());
        assert_eq!(store.remove("Gamma").unwrap().name, "Gamma");
        assert!(store.rename("Beta", "   ").is_err());
    }

    #[test]
    fn import_export_preserves_legacy_compatible_profiles() {
        let legacy = r#"{"name":"Old","product_id":123,"serial":null,"dpi":800}"#;
        let mut store = ProfileStore::default();
        assert_eq!(store.import_json(legacy, false).unwrap(), vec!["Old"]);
        assert!(store.import_json(legacy, false).is_err());
        assert!(store.import_json(legacy, true).is_ok());
        let json = store.export_json(&["Old".into()]).unwrap();
        let exported: ProfileStore = serde_json::from_str(&json).unwrap();
        assert_eq!(exported.profiles.len(), 1);
        assert_eq!(exported.profiles[0].dpi, Some(800));
    }

}
