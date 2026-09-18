use crate::qualification;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;
use zip::ZipArchive;

const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Appearance {
    pub title: String,
    #[serde(alias = "title_visible")]
    pub title_visible: bool,
    #[serde(alias = "font_family")]
    pub font_family: String,
    #[serde(alias = "font_size")]
    pub font_size: u16,
    #[serde(alias = "font_weight")]
    pub font_weight: u16,
    #[serde(alias = "text_color")]
    pub text_color: String,
    #[serde(alias = "horizontal_align")]
    pub horizontal_align: String,
    #[serde(alias = "vertical_align")]
    pub vertical_align: String,
    #[serde(alias = "icon_asset_id")]
    pub icon_asset_id: Option<String>,
    #[serde(alias = "background_asset_id")]
    pub background_asset_id: Option<String>,
    #[serde(alias = "background_color")]
    pub background_color: String,
    #[serde(alias = "fit_mode")]
    pub fit_mode: String,
    #[serde(alias = "icon_opacity")]
    pub icon_opacity: f32,
    #[serde(alias = "title_offset_x")]
    pub title_offset_x: i16,
    #[serde(alias = "title_offset_y")]
    pub title_offset_y: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DialStackEntry {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub bindings: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DialStack {
    pub behavior: String,
    pub active_index: usize,
    pub entries: Vec<DialStackEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActionWheelEntry {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub bindings: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActionWheel {
    pub behavior: String,
    pub active_index: usize,
    pub entries: Vec<ActionWheelEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ControlSlot {
    pub id: String,
    pub kind: String,
    pub position: usize,
    #[serde(default)]
    pub bindings: serde_json::Map<String, serde_json::Value>,
    pub appearance: Appearance,
    #[serde(default)]
    pub states: serde_json::Map<String, serde_json::Value>,
    #[serde(alias = "folder_target")]
    pub folder_target: Option<String>,
    #[serde(default, alias = "dial_stack")]
    pub dial_stack: Option<DialStack>,
    #[serde(default, alias = "action_wheel")]
    pub action_wheel: Option<ActionWheel>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PageSlots {
    pub keys: Vec<ControlSlot>,
    pub dials: Vec<ControlSlot>,
    pub touch_regions: Vec<ControlSlot>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TouchStripRule {
    pub pattern: String,
    pub presentation: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TouchStripConfig {
    pub mode: String,
    pub adaptive_fallback: String,
    #[serde(default)]
    pub adaptive_rules: Vec<TouchStripRule>,
    pub unified_slot: ControlSlot,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Page {
    pub id: String,
    pub name: String,
    pub slots: PageSlots,
    #[serde(default = "default_touch_strip_config")]
    pub touch_strip: TouchStripConfig,
    #[serde(alias = "parent_folder_id")]
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
    #[serde(default)]
    pub plugin_owner_uuid: Option<String>,
    #[serde(default)]
    pub plugin_readonly: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EditorPreferences {
    #[serde(alias = "action_panel_width")]
    pub action_panel_width: u16,
    #[serde(alias = "inspector_height")]
    pub inspector_height: u16,
    #[serde(alias = "action_panel_collapsed")]
    pub action_panel_collapsed: bool,
    #[serde(alias = "inspector_collapsed")]
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
            dial_stack: None,
            action_wheel: None,
        })
        .collect()
}

fn default_touch_strip_config() -> TouchStripConfig {
    let mut unified_slot = slots("touch", 1).remove(0);
    unified_slot.id = id("touch-unified");
    unified_slot.appearance.title = "Touch Strip".into();
    TouchStripConfig {
        mode: "segmented".into(),
        adaptive_fallback: "segmented".into(),
        adaptive_rules: Vec::new(),
        unified_slot,
    }
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
                touch_strip: default_touch_strip_config(),
            }],
            active_page_id: page_id,
            app_match: Vec::new(),
            plugin_owner_uuid: None,
            plugin_readonly: false,
        }],
        assets: Vec::new(),
        preferences: EditorPreferences {
            action_panel_width: 330,
            inspector_height: 270,
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
        if let Some(stack) = &slot.dial_stack {
            if expected_kind != "dial" {
                return Err("dial stack may only be attached to dial controls".into());
            }
            if stack.behavior != "pressCycle" {
                return Err("invalid dial stack behavior".into());
            }
            if stack.entries.is_empty() || stack.entries.len() > 32 {
                return Err("dial stack must contain between 1 and 32 entries".into());
            }
            if stack.active_index >= stack.entries.len() {
                return Err("dial stack active index out of range".into());
            }
            for entry in &stack.entries {
                if entry.id.trim().is_empty() || !ids.insert(entry.id.clone()) {
                    return Err("duplicate or empty dial stack entry id".into());
                }
                if entry.label.trim().is_empty() {
                    return Err("dial stack entry label may not be empty".into());
                }
            }
        }
        if let Some(wheel) = &slot.action_wheel {
            if expected_kind != "dial" {
                return Err("action wheel may only be attached to dial controls".into());
            }
            if slot.dial_stack.is_some() {
                return Err(
                    "dial control may not contain both a dial stack and action wheel".into(),
                );
            }
            if wheel.behavior != "rotateSelectPressExecute" {
                return Err("invalid action wheel behavior".into());
            }
            if wheel.entries.is_empty() || wheel.entries.len() > 32 {
                return Err("action wheel must contain between 1 and 32 entries".into());
            }
            if wheel.active_index >= wheel.entries.len() {
                return Err("action wheel active index out of range".into());
            }
            for entry in &wheel.entries {
                if entry.id.trim().is_empty() || !ids.insert(entry.id.clone()) {
                    return Err("duplicate or empty action wheel entry id".into());
                }
                if entry.label.trim().is_empty() {
                    return Err("action wheel entry label may not be empty".into());
                }
                if entry.bindings.keys().any(|key| key != "press") {
                    return Err("action wheel entries support press bindings only".into());
                }
            }
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
            let unified = &page.touch_strip.unified_slot;
            if unified.kind != "touch"
                || unified.position != 0
                || unified.id.trim().is_empty()
                || !ids.insert(unified.id.clone())
            {
                return Err("invalid unified touch slot".into());
            }
            if !matches!(
                page.touch_strip.mode.as_str(),
                "segmented" | "unified" | "adaptive"
            ) {
                return Err("invalid touch strip mode".into());
            }
            if !matches!(
                page.touch_strip.adaptive_fallback.as_str(),
                "segmented" | "unified"
            ) {
                return Err("invalid touch strip adaptive fallback".into());
            }
            for rule in &page.touch_strip.adaptive_rules {
                if rule.pattern.trim().is_empty()
                    || !matches!(rule.presentation.as_str(), "segmented" | "unified")
                {
                    return Err("invalid touch strip adaptive rule".into());
                }
            }
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
    let started = std::time::Instant::now();
    let result = if qualification::enabled() {
        validate_workspace(&workspace)?;
        let bytes = serde_json::to_vec_pretty(&workspace).map_err(|error| error.to_string())?;
        write_atomic(&qualification::workspace_benchmark_path()?, &bytes)
    } else {
        let (primary, backup) = workspace_paths()?;
        save_workspace_at(&primary, &backup, &workspace)
    };
    qualification::record_runtime_sample(
        "persistenceWriteMs",
        started.elapsed().as_secs_f64() * 1000.0,
    );
    result
}

#[tauri::command]
pub(crate) fn editor_export_profile(profile: Profile, path: String) -> Result<(), String> {
    validate_profile(&profile)?;
    let path = PathBuf::from(path);
    safe_export_path(&path)?;
    let bytes = serde_json::to_vec_pretty(&profile).map_err(|error| error.to_string())?;
    write_atomic(&path, &bytes)
}

fn streamdeck_folder_id(uuid: &str) -> Result<String, String> {
    let mut hex = uuid.replace('-', "");
    if hex.len() != 32 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(format!("invalid Stream Deck page UUID: {uuid}"));
    }
    hex.push_str("000");
    let alphabet = b"0123456789abcdefghijklmnopqrstuv";
    let mut encoded = String::new();
    for chunk in hex.as_bytes().chunks(5) {
        let chunk = std::str::from_utf8(chunk).map_err(|error| error.to_string())?;
        let mut value = u32::from_str_radix(chunk, 16).map_err(|error| error.to_string())?;
        let mut digits = ['0'; 4];
        for index in (0..4).rev() {
            digits[index] = alphabet[(value % 32) as usize] as char;
            value /= 32;
        }
        encoded.extend(digits);
    }
    encoded.truncate(26);
    let encoded: String = encoded
        .to_ascii_uppercase()
        .chars()
        .map(|ch| match ch {
            'U' => 'V',
            'V' => 'W',
            other => other,
        })
        .collect();
    Ok(format!("{encoded}Z"))
}

fn read_zip_json(archive: &mut ZipArchive<File>, name: &str) -> Result<serde_json::Value, String> {
    let mut file = archive
        .by_name(name)
        .map_err(|error| format!("read {name}: {error}"))?;
    let mut text = String::new();
    file.read_to_string(&mut text)
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| format!("parse {name}: {error}"))
}

fn profile_action_instance(
    action: &serde_json::Value,
    bundled_plugin_uuid: Option<&str>,
) -> Option<serde_json::Value> {
    let action_uuid = action.get("UUID")?.as_str()?;
    let definition_id = match action_uuid {
        "com.elgato.streamdeck.page.next" => "editor.nextPage".to_string(),
        "com.elgato.streamdeck.page.previous" => "editor.previousPage".to_string(),
        _ => {
            let action_plugin_uuid = action
                .pointer("/Plugin/UUID")
                .and_then(serde_json::Value::as_str)
                .or_else(|| bundled_plugin_uuid.filter(|plugin| action_uuid.starts_with(*plugin)));
            if let Some(plugin_uuid) = action_plugin_uuid {
                format!("plugin:{plugin_uuid}:{action_uuid}")
            } else {
                format!("plugin:elgato-profile:{action_uuid}")
            }
        }
    };
    let state = action
        .get("State")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let settings = action
        .get("Settings")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    Some(serde_json::json!({
        "definitionId": definition_id,
        "config": {
            "context": format!("plugin-context-{}", Uuid::new_v4()),
            "settings": settings,
            "resources": {},
            "state": state,
        }
    }))
}

fn apply_profile_action(
    slot: &mut ControlSlot,
    action: &serde_json::Value,
    binding: serde_json::Value,
) {
    slot.bindings.insert("press".into(), binding);
    let state_index = action
        .get("State")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0) as usize;
    if let Some(states) = action.get("States").and_then(serde_json::Value::as_array) {
        let state = states.get(state_index).or_else(|| states.first());
        if let Some(state) = state {
            if let Some(title) = state.get("Title").and_then(serde_json::Value::as_str) {
                slot.appearance.title = title.into();
            }
            if let Some(show) = state.get("ShowTitle").and_then(serde_json::Value::as_bool) {
                slot.appearance.title_visible = show;
            }
            if let Some(color) = state
                .get("TitleColor")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.is_empty())
            {
                slot.appearance.text_color = color.into();
            }
            if let Some(font) = state.get("FontFamily").and_then(serde_json::Value::as_str) {
                slot.appearance.font_family = font.into();
            }
            if let Some(size) = state.get("FontSize").and_then(serde_json::Value::as_u64) {
                slot.appearance.font_size = size.clamp(6, 96) as u16;
            }
            if let Some(align) = state
                .get("TitleAlignment")
                .and_then(serde_json::Value::as_str)
            {
                slot.appearance.vertical_align = match align {
                    "top" => "top",
                    "middle" => "center",
                    _ => "bottom",
                }
                .into();
            }
        }
    }
}

fn page_from_streamdeck_manifest(
    value: &serde_json::Value,
    name: String,
    bundled_plugin_uuid: Option<&str>,
) -> Result<Page, String> {
    let mut page = Page {
        id: id("page"),
        name,
        slots: PageSlots {
            keys: slots("key", 8),
            dials: slots("dial", 4),
            touch_regions: slots("touch", 4),
        },
        touch_strip: default_touch_strip_config(),
        parent_folder_id: None,
    };
    let controllers = value
        .get("Controllers")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    for controller in controllers {
        let controller_type = controller
            .get("Type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Keypad");
        let Some(actions) = controller
            .get("Actions")
            .and_then(serde_json::Value::as_object)
        else {
            continue;
        };
        for (coordinate, action) in actions {
            let mut pieces = coordinate.split(',');
            let col = pieces
                .next()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            let row = pieces
                .next()
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            let Some(binding) = profile_action_instance(action, bundled_plugin_uuid) else {
                continue;
            };
            if controller_type == "Encoder" {
                if col >= page.slots.dials.len() {
                    continue;
                }
                let mut dial_binding = binding.clone();
                if let Some(config) = dial_binding
                    .get_mut("config")
                    .and_then(serde_json::Value::as_object_mut)
                {
                    let shared_context = format!("plugin-context-{}", Uuid::new_v4());
                    config.insert(
                        "context".into(),
                        serde_json::Value::String(shared_context.clone()),
                    );
                    for interaction in [
                        "press",
                        "rotateLeft",
                        "rotateRight",
                        "pressRotateLeft",
                        "pressRotateRight",
                    ] {
                        let mut clone = dial_binding.clone();
                        if let Some(clone_config) = clone
                            .get_mut("config")
                            .and_then(serde_json::Value::as_object_mut)
                        {
                            clone_config.insert(
                                "context".into(),
                                serde_json::Value::String(shared_context.clone()),
                            );
                        }
                        page.slots.dials[col]
                            .bindings
                            .insert(interaction.into(), clone);
                    }
                    if col < page.slots.touch_regions.len() {
                        let mut touch = dial_binding.clone();
                        if let Some(touch_config) = touch
                            .get_mut("config")
                            .and_then(serde_json::Value::as_object_mut)
                        {
                            touch_config.insert(
                                "context".into(),
                                serde_json::Value::String(shared_context),
                            );
                        }
                        page.slots.touch_regions[col]
                            .bindings
                            .insert("touch".into(), touch);
                    }
                }
                apply_profile_action(&mut page.slots.dials[col], action, binding);
            } else {
                let position = row.saturating_mul(4).saturating_add(col);
                if position >= page.slots.keys.len() {
                    continue;
                }
                apply_profile_action(&mut page.slots.keys[position], action, binding);
            }
        }
    }
    Ok(page)
}

pub(crate) fn import_streamdeck_profile(
    path: &Path,
    bundled_plugin_uuid: Option<&str>,
) -> Result<Profile, String> {
    let file = File::open(path).map_err(|error| format!("open {}: {error}", path.display()))?;
    let mut archive = ZipArchive::new(file).map_err(|error| {
        format!(
            "{} is not a readable .streamDeckProfile: {error}",
            path.display()
        )
    })?;
    let names = (0..archive.len())
        .filter_map(|index| {
            archive
                .by_index(index)
                .ok()
                .map(|file| file.name().to_string())
        })
        .collect::<Vec<_>>();
    let root_manifest_name = names
        .iter()
        .find(|name| name.ends_with(".sdProfile/manifest.json") && !name.contains("/Profiles/"))
        .cloned()
        .ok_or_else(|| "Stream Deck profile root manifest not found".to_string())?;
    let root_prefix = root_manifest_name
        .strip_suffix("manifest.json")
        .unwrap_or_default()
        .to_string();
    let root = read_zip_json(&mut archive, &root_manifest_name)?;
    let profile_name = root
        .get("Name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("Imported Stream Deck Profile")
        .to_string();
    let mut pages = Vec::new();
    let mut source_page_ids = Vec::new();
    if let Some(page_ids) = root
        .pointer("/Pages/Pages")
        .and_then(serde_json::Value::as_array)
    {
        for page_uuid in page_ids.iter().filter_map(serde_json::Value::as_str) {
            let folder = streamdeck_folder_id(page_uuid)?;
            let manifest_name = format!("{root_prefix}Profiles/{folder}/manifest.json");
            if names.iter().any(|name| name == &manifest_name) {
                let page_value = read_zip_json(&mut archive, &manifest_name)?;
                let page_name = page_value
                    .get("Name")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Page")
                    .to_string();
                pages.push(page_from_streamdeck_manifest(
                    &page_value,
                    page_name,
                    bundled_plugin_uuid,
                )?);
                source_page_ids.push(page_uuid.to_string());
            }
        }
    }
    if pages.is_empty() {
        let page_manifests = names
            .iter()
            .filter(|name| {
                name.starts_with(&format!("{root_prefix}Profiles/"))
                    && name.ends_with("/manifest.json")
            })
            .cloned()
            .collect::<Vec<_>>();
        for (index, manifest_name) in page_manifests.iter().enumerate() {
            let page_value = read_zip_json(&mut archive, manifest_name)?;
            let page_name = page_value
                .get("Name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .unwrap_or_else(|| format!("Page {}", index + 1));
            pages.push(page_from_streamdeck_manifest(
                &page_value,
                page_name,
                bundled_plugin_uuid,
            )?);
        }
    }
    if pages.is_empty() && root.get("Actions").is_some() {
        let synthetic = serde_json::json!({"Controllers":[{"Type":"Keypad","Actions":root.get("Actions").cloned().unwrap_or_else(|| serde_json::json!({}))}]});
        pages.push(page_from_streamdeck_manifest(
            &synthetic,
            "Page 1".into(),
            bundled_plugin_uuid,
        )?);
    }
    if pages.is_empty() {
        return Err("Stream Deck profile contains no importable pages".into());
    }
    let current_source = root
        .pointer("/Pages/Current")
        .and_then(serde_json::Value::as_str);
    let current_index = current_source
        .and_then(|current| {
            source_page_ids
                .iter()
                .position(|candidate| candidate == current)
        })
        .unwrap_or(0)
        .min(pages.len() - 1);
    let active_page_id = pages[current_index].id.clone();
    Ok(Profile {
        id: id("profile"),
        name: profile_name,
        device_id: "stream-deck-plus".into(),
        pages,
        active_page_id,
        app_match: Vec::new(),
        plugin_owner_uuid: None,
        plugin_readonly: false,
    })
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
    if path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("streamDeckProfile"))
    {
        let profile = import_streamdeck_profile(&path, None)?;
        validate_profile(&profile)?;
        return Ok(profile);
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
    fn workspace_json_matches_frontend_camel_case_and_reads_legacy_nested_snake_case() {
        let workspace = default_workspace();
        let value = serde_json::to_value(&workspace).unwrap();
        let appearance = &value["profiles"][0]["pages"][0]["slots"]["keys"][0]["appearance"];
        assert!(appearance.get("fontFamily").is_some());
        assert!(appearance.get("font_family").is_none());
        assert!(value["preferences"].get("actionPanelWidth").is_some());
        assert!(
            value["profiles"][0]["pages"][0]
                .get("parentFolderId")
                .is_some()
        );

        let legacy = serde_json::to_string(&workspace)
            .unwrap()
            .replace("titleVisible", "title_visible")
            .replace("fontFamily", "font_family")
            .replace("fontSize", "font_size")
            .replace("fontWeight", "font_weight")
            .replace("textColor", "text_color")
            .replace("horizontalAlign", "horizontal_align")
            .replace("verticalAlign", "vertical_align")
            .replace("iconAssetId", "icon_asset_id")
            .replace("backgroundAssetId", "background_asset_id")
            .replace("backgroundColor", "background_color")
            .replace("fitMode", "fit_mode")
            .replace("iconOpacity", "icon_opacity")
            .replace("titleOffsetX", "title_offset_x")
            .replace("titleOffsetY", "title_offset_y")
            .replace("folderTarget", "folder_target")
            .replace("parentFolderId", "parent_folder_id")
            .replace("actionPanelWidth", "action_panel_width")
            .replace("inspectorHeight", "inspector_height")
            .replace("actionPanelCollapsed", "action_panel_collapsed")
            .replace("inspectorCollapsed", "inspector_collapsed");
        let parsed: Workspace = serde_json::from_str(&legacy).unwrap();
        validate_workspace(&parsed).unwrap();
    }

    #[test]
    fn schema_one_workspace_without_touch_strip_defaults_to_segmented() {
        let workspace = default_workspace();
        let mut value = serde_json::to_value(&workspace).unwrap();
        value["profiles"][0]["pages"][0]
            .as_object_mut()
            .unwrap()
            .remove("touchStrip");

        let parsed: Workspace = serde_json::from_value(value).unwrap();
        validate_workspace(&parsed).unwrap();
        let touch_strip = &parsed.profiles[0].pages[0].touch_strip;
        assert_eq!(touch_strip.mode, "segmented");
        assert_eq!(touch_strip.adaptive_fallback, "segmented");
        assert_eq!(touch_strip.unified_slot.kind, "touch");
        assert_eq!(touch_strip.unified_slot.position, 0);
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

    #[test]
    fn dial_stack_round_trip_preserves_active_entry_and_bindings() {
        let mut workspace = default_workspace();
        workspace.profiles[0].pages[0].slots.dials[0].dial_stack = Some(DialStack {
            behavior: "pressCycle".into(),
            active_index: 1,
            entries: vec![
                DialStackEntry {
                    id: "stack-one".into(),
                    label: "Volume".into(),
                    bindings: serde_json::Map::new(),
                },
                DialStackEntry {
                    id: "stack-two".into(),
                    label: "OBS".into(),
                    bindings: serde_json::Map::from_iter([(
                        "rotateRight".into(),
                        serde_json::json!({"definitionId":"editor.nextPage","config":{}}),
                    )]),
                },
            ],
        });
        validate_workspace(&workspace).unwrap();
        let json = serde_json::to_string(&workspace).unwrap();
        let parsed: Workspace = serde_json::from_str(&json).unwrap();
        let stack = parsed.profiles[0].pages[0].slots.dials[0]
            .dial_stack
            .as_ref()
            .unwrap();
        assert_eq!(stack.active_index, 1);
        assert_eq!(stack.entries[1].label, "OBS");
        assert!(stack.entries[1].bindings.contains_key("rotateRight"));
    }

    #[test]
    fn rejects_dial_stack_on_non_dial_control() {
        let mut workspace = default_workspace();
        workspace.profiles[0].pages[0].slots.keys[0].dial_stack = Some(DialStack {
            behavior: "pressCycle".into(),
            active_index: 0,
            entries: vec![DialStackEntry {
                id: "bad-stack".into(),
                label: "Bad".into(),
                bindings: serde_json::Map::new(),
            }],
        });
        assert!(
            validate_workspace(&workspace)
                .unwrap_err()
                .contains("dial stack")
        );
    }

    #[test]
    fn action_wheel_round_trip_preserves_selection_and_press_binding() {
        let mut workspace = default_workspace();
        workspace.profiles[0].pages[0].slots.dials[1].action_wheel = Some(ActionWheel {
            behavior: "rotateSelectPressExecute".into(),
            active_index: 1,
            entries: vec![
                ActionWheelEntry {
                    id: "wheel-one".into(),
                    label: "OBS".into(),
                    bindings: serde_json::Map::new(),
                },
                ActionWheelEntry {
                    id: "wheel-two".into(),
                    label: "Browser".into(),
                    bindings: serde_json::Map::from_iter([(
                        "press".into(),
                        serde_json::json!({"definitionId":"marketplace.open","config":{}}),
                    )]),
                },
            ],
        });
        validate_workspace(&workspace).unwrap();
        let json = serde_json::to_string(&workspace).unwrap();
        let parsed: Workspace = serde_json::from_str(&json).unwrap();
        let wheel = parsed.profiles[0].pages[0].slots.dials[1]
            .action_wheel
            .as_ref()
            .unwrap();
        assert_eq!(wheel.active_index, 1);
        assert_eq!(wheel.entries[1].label, "Browser");
        assert!(wheel.entries[1].bindings.contains_key("press"));
    }

    #[test]
    fn rejects_action_wheel_on_non_dial_control() {
        let mut workspace = default_workspace();
        workspace.profiles[0].pages[0].slots.keys[0].action_wheel = Some(ActionWheel {
            behavior: "rotateSelectPressExecute".into(),
            active_index: 0,
            entries: vec![ActionWheelEntry {
                id: "bad-wheel".into(),
                label: "Bad".into(),
                bindings: serde_json::Map::new(),
            }],
        });
        assert!(
            validate_workspace(&workspace)
                .unwrap_err()
                .contains("action wheel")
        );
    }
}
