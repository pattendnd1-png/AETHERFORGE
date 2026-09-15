use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Appearance {
    pub title: String,
    pub title_visible: bool,
    pub font_family: String,
    pub font_size: u16,
    pub font_weight: u16,
    pub text_color: String,
    pub horizontal_align: String,
    pub vertical_align: String,
    pub icon_asset_id: Option<String>,
    pub background_asset_id: Option<String>,
    pub background_color: String,
    pub fit_mode: String,
    pub icon_opacity: f32,
    pub title_offset_x: i16,
    pub title_offset_y: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct ControlSlot {
    pub id: String,
    pub kind: String,
    pub position: usize,
    #[serde(default)]
    pub bindings: serde_json::Map<String, serde_json::Value>,
    pub appearance: Appearance,
    #[serde(default)]
    pub states: serde_json::Map<String, serde_json::Value>,
    pub folder_target: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PageSlots {
    pub keys: Vec<ControlSlot>,
    pub dials: Vec<ControlSlot>,
    pub touch_regions: Vec<ControlSlot>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Page {
    pub id: String,
    pub name: String,
    pub slots: PageSlots,
    pub parent_folder_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Profile {
    pub id: String,
    pub name: String,
    pub device_id: String,
    pub pages: Vec<Page>,
    pub active_page_id: String,
    #[serde(default)]
    pub app_match: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct EditorPreferences {
    pub action_panel_width: u16,
    pub inspector_height: u16,
    pub action_panel_collapsed: bool,
    pub inspector_collapsed: bool,
    pub zoom: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct AssetRecord {
    pub id: String,
    pub name: String,
    pub path: String,
    pub source: String,
    pub mime: String,
    pub sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Workspace {
    pub schema_version: u32,
    pub active_device_id: String,
    pub active_profile_id: String,
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub assets: Vec<AssetRecord>,
    pub preferences: EditorPreferences,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct WorkspaceLoadResult {
    pub workspace: Workspace,
    pub source: String,
    pub warning: Option<String>,
}

fn id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::new_v4())
}

fn default_appearance() -> Appearance {
    Appearance {
        title: String::new(),
        title_visible: true,
        font_family: "Inter, system-ui, sans-serif".into(),
        font_size: 14,
        font_weight: 500,
        text_color: "#ffffff".into(),
        horizontal_align: "center".into(),
        vertical_align: "bottom".into(),
        icon_asset_id: None,
        background_asset_id: None,
        background_color: "#16181d".into(),
        fit_mode: "contain".into(),
        icon_opacity: 1.0,
        title_offset_x: 0,
        title_offset_y: 0,
    }
}

fn slots(kind: &str, count: usize) -> Vec<ControlSlot> {
    (0..count)
        .map(|position| ControlSlot {
            id: id(kind),
            kind: kind.into(),
            position,
            bindings: serde_json::Map::new(),
            appearance: default_appearance(),
            states: serde_json::Map::new(),
            folder_target: None,
        })
        .collect()
}

pub(crate) fn default_workspace() -> Workspace {
    let page_id = id("page");
    let profile_id = id("profile");
    Workspace {
        schema_version: SCHEMA_VERSION,
        active_device_id: "stream-deck-plus".into(),
        active_profile_id: profile_id.clone(),
        profiles: vec![Profile {
            id: profile_id,
            name: "Default Profile".into(),
            device_id: "stream-deck-plus".into(),
            pages: vec![Page {
                id: page_id.clone(),
                name: "Page 1".into(),
                parent_folder_id: None,
                slots: PageSlots {
                    keys: slots("key", 8),
                    dials: slots("dial", 4),
                    touch_regions: slots("touch", 4),
                },
            }],
            active_page_id: page_id,
            app_match: Vec::new(),
        }],
        assets: Vec::new(),
        preferences: EditorPreferences {
            action_panel_width: 270,
            inspector_height: 225,
            action_panel_collapsed: false,
            inspector_collapsed: false,
            zoom: 1.0,
        },
    }
}

fn validate_slot_group(
    slots: &[ControlSlot],
    expected_kind: &str,
    expected_count: usize,
    ids: &mut HashSet<String>,
) -> Result<(), String> {
    if slots.len() != expected_count {
        return Err(format!(
            "expected {expected_count} {expected_kind} controls"
        ));
    }
    for (position, slot) in slots.iter().enumerate() {
        if slot.kind != expected_kind || slot.position != position {
            return Err(format!("invalid {expected_kind} slot layout"));
        }
        if slot.id.trim().is_empty() || !ids.insert(slot.id.clone()) {
            return Err("duplicate or empty control id".into());
        }
        if !(0.0..=1.0).contains(&slot.appearance.icon_opacity) {
            return Err("icon opacity out of range".into());
        }
    }
    Ok(())
}

pub(crate) fn validate_workspace(workspace: &Workspace) -> Result<(), String> {
    if workspace.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema version {}",
            workspace.schema_version
        ));
    }
    if workspace.profiles.is_empty() {
        return Err("workspace requires at least one profile".into());
    }
    if !workspace
        .profiles
        .iter()
        .any(|profile| profile.id == workspace.active_profile_id)
    {
        return Err("active profile does not exist".into());
    }
    let mut ids = HashSet::new();
    for profile in &workspace.profiles {
        if profile.id.trim().is_empty() || !ids.insert(profile.id.clone()) {
            return Err("duplicate or empty profile id".into());
        }
        if profile.pages.is_empty() {
            return Err("profile requires at least one page".into());
        }
        if !profile
            .pages
            .iter()
            .any(|page| page.id == profile.active_page_id)
        {
            return Err("active page does not exist".into());
        }
        for page in &profile.pages {
            if page.id.trim().is_empty() || !ids.insert(page.id.clone()) {
                return Err("duplicate or empty page id".into());
            }
            validate_slot_group(&page.slots.keys, "key", 8, &mut ids)?;
            validate_slot_group(&page.slots.dials, "dial", 4, &mut ids)?;
            validate_slot_group(&page.slots.touch_regions, "touch", 4, &mut ids)?;
        }
    }
    Ok(())
}

fn editor_root() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME").ok_or_else(|| "HOME is not set".to_string())?;
    Ok(PathBuf::from(home).join(".config/opendeck-v2/editor"))
}

fn workspace_paths() -> Result<(PathBuf, PathBuf), String> {
    let root = editor_root()?;
    Ok((
        root.join("workspace.json"),
        root.join("workspace.backup.json"),
    ))
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "invalid destination".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temp = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        Uuid::new_v4()
    ));
    let mut file = File::create(&temp).map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    fs::rename(&temp, path).map_err(|error| error.to_string())?;
    Ok(())
}

fn save_workspace_at(primary: &Path, backup: &Path, workspace: &Workspace) -> Result<(), String> {
    validate_workspace(workspace)?;
    if primary.exists()
        && fs::read_to_string(primary)
            .ok()
            .and_then(|text| serde_json::from_str::<Workspace>(&text).ok())
            .and_then(|value| validate_workspace(&value).ok())
            .is_some()
    {
        fs::copy(primary, backup).map_err(|error| error.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(workspace).map_err(|error| error.to_string())?;
    write_atomic(primary, &bytes)
}

fn load_workspace_at(primary: &Path, backup: &Path) -> WorkspaceLoadResult {
    let read = |path: &Path| -> Result<Workspace, String> {
        let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
        let workspace =
            serde_json::from_str::<Workspace>(&text).map_err(|error| error.to_string())?;
        validate_workspace(&workspace)?;
        Ok(workspace)
    };
    if let Ok(workspace) = read(primary) {
        return WorkspaceLoadResult {
            workspace,
            source: "primary".into(),
            warning: None,
        };
    }
    if let Ok(workspace) = read(backup) {
        return WorkspaceLoadResult {
            workspace,
            source: "backup".into(),
            warning: Some("Primary editor workspace was invalid; recovered backup.".into()),
        };
    }
    WorkspaceLoadResult {
        workspace: default_workspace(),
        source: "default".into(),
        warning: None,
    }
}

fn safe_export_path(path: &Path) -> Result<(), String> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("export path may not contain '..'".into());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "export parent is missing".to_string())?;
    if !parent.exists() || !parent.is_dir() {
        return Err("export parent does not exist".into());
    }
    Ok(())
}

fn validate_profile(profile: &Profile) -> Result<(), String> {
    if profile.pages.is_empty() {
        return Err("profile requires at least one page".into());
    }
    if !profile
        .pages
        .iter()
        .any(|page| page.id == profile.active_page_id)
    {
        return Err("active page does not exist".into());
    }
    let mut ids = HashSet::new();
    for page in &profile.pages {
        validate_slot_group(&page.slots.keys, "key", 8, &mut ids)?;
        validate_slot_group(&page.slots.dials, "dial", 4, &mut ids)?;
        validate_slot_group(&page.slots.touch_regions, "touch", 4, &mut ids)?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn editor_load_workspace() -> Result<WorkspaceLoadResult, String> {
    let (primary, backup) = workspace_paths()?;
    Ok(load_workspace_at(&primary, &backup))
}

#[tauri::command]
pub(crate) fn editor_save_workspace(workspace: Workspace) -> Result<(), String> {
    let (primary, backup) = workspace_paths()?;
    save_workspace_at(&primary, &backup, &workspace)
}

#[tauri::command]
pub(crate) fn editor_export_profile(profile: Profile, path: String) -> Result<(), String> {
    validate_profile(&profile)?;
    let path = PathBuf::from(path);
    safe_export_path(&path)?;
    let bytes = serde_json::to_vec_pretty(&profile).map_err(|error| error.to_string())?;
    write_atomic(&path, &bytes)
}

#[tauri::command]
pub(crate) fn editor_import_profile(path: String) -> Result<Profile, String> {
    let path = PathBuf::from(path);
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("import path may not contain '..'".into());
    }
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let profile = serde_json::from_str::<Profile>(&text).map_err(|error| error.to_string())?;
    validate_profile(&profile)?;
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!("opendeck-editor-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn rejects_future_schema() {
        let mut workspace = default_workspace();
        workspace.schema_version = 99;
        assert!(
            validate_workspace(&workspace)
                .unwrap_err()
                .contains("schema version")
        );
    }

    #[test]
    fn default_workspace_validates() {
        validate_workspace(&default_workspace()).unwrap();
    }

    #[test]
    fn atomic_save_and_load_round_trip() {
        let root = temp_dir();
        let primary = root.join("workspace.json");
        let backup = root.join("workspace.backup.json");
        let workspace = default_workspace();
        save_workspace_at(&primary, &backup, &workspace).unwrap();
        let loaded = load_workspace_at(&primary, &backup);
        assert_eq!(loaded.source, "primary");
        assert_eq!(loaded.workspace.schema_version, 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recovers_backup_after_corrupt_primary() {
        let root = temp_dir();
        let primary = root.join("workspace.json");
        let backup = root.join("workspace.backup.json");
        let workspace = default_workspace();
        save_workspace_at(&primary, &backup, &workspace).unwrap();
        fs::copy(&primary, &backup).unwrap();
        fs::write(&primary, b"not-json").unwrap();
        let loaded = load_workspace_at(&primary, &backup);
        assert_eq!(loaded.source, "backup");
        assert!(loaded.warning.is_some());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_zero_page_profile() {
        let mut profile = default_workspace().profiles.remove(0);
        profile.pages.clear();
        assert!(validate_profile(&profile).is_err());
    }

    #[test]
    fn rejects_export_traversal() {
        assert!(safe_export_path(Path::new("../profile.json")).is_err());
    }
}
