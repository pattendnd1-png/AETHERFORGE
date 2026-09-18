use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;
use walkdir::WalkDir;
use zip::ZipArchive;

const META_FILE: &str = ".opendeck-pack.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IconPackDescriptor {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub root: String,
    pub active: bool,
    pub item_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredPackMeta {
    id: String,
    name: String,
    version: String,
    author: String,
    description: String,
}

#[derive(Clone, Debug, Deserialize)]
struct IconMeta {
    path: String,
    name: String,
    #[serde(default)]
    tags: Vec<String>,
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())
}

fn active_root() -> Result<PathBuf, String> {
    Ok(home_dir()?.join(".local/share/opendeck/icon-packs"))
}

fn inactive_root() -> Result<PathBuf, String> {
    Ok(home_dir()?.join(".local/share/opendeck/icon-packs-disabled"))
}

fn safe_id(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut last_dash = false;
    for ch in value.chars() {
        let mapped = if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            ch.to_ascii_lowercase()
        } else {
            '-'
        };
        if mapped == '-' {
            if last_dash {
                continue;
            }
            last_dash = true;
        } else {
            last_dash = false;
        }
        out.push(mapped);
    }
    out.trim_matches('-').to_string()
}

fn validate_pack_id(pack_id: &str) -> Result<(), String> {
    if pack_id.is_empty()
        || pack_id.len() > 160
        || pack_id.contains("..")
        || pack_id.contains('/')
        || pack_id.contains('\\')
        || pack_id
            .chars()
            .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-')))
    {
        return Err("invalid icon-pack id".into());
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| format!("open {}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    for entry in WalkDir::new(source)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();
        let relative = path
            .strip_prefix(source)
            .map_err(|error| error.to_string())?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target).map_err(|error| error.to_string())?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::copy(path, &target).map_err(|error| format!("copy {}: {error}", path.display()))?;
        }
    }
    Ok(())
}

fn extract_package(source: &Path, destination: &Path) -> Result<(), String> {
    let file = File::open(source).map_err(|error| format!("open {}: {error}", source.display()))?;
    let mut archive = ZipArchive::new(file).map_err(|error| {
        format!(
            "{} is not a readable .streamDeckIconPack: {error}",
            source.display()
        )
    })?;
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            continue;
        }
        let Some(relative) = entry.enclosed_name() else {
            return Err("icon-pack archive contains an unsafe path".into());
        };
        let target = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(&target).map_err(|error| error.to_string())?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut output = File::create(&target).map_err(|error| error.to_string())?;
        std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
        output.flush().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn decode_utf16_json(bytes: &[u8], big_endian: bool) -> Result<Vec<u8>, String> {
    if !bytes.len().is_multiple_of(2) {
        return Err("icon-pack JSON has an odd UTF-16 byte length".into());
    }
    let units = bytes
        .chunks_exact(2)
        .map(|pair| {
            if big_endian {
                u16::from_be_bytes([pair[0], pair[1]])
            } else {
                u16::from_le_bytes([pair[0], pair[1]])
            }
        })
        .collect::<Vec<_>>();
    String::from_utf16(&units)
        .map(|value| value.into_bytes())
        .map_err(|error| format!("decode icon-pack UTF-16 JSON: {error}"))
}

fn normalize_json_bytes(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return Ok(rest.to_vec());
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return decode_utf16_json(rest, false);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return decode_utf16_json(rest, true);
    }
    if bytes.len() >= 4 && bytes[1] == 0 && bytes[3] == 0 {
        return decode_utf16_json(bytes, false);
    }
    if bytes.len() >= 4 && bytes[0] == 0 && bytes[2] == 0 {
        return decode_utf16_json(bytes, true);
    }
    Ok(bytes.to_vec())
}

fn manifest_candidates(root: &Path) -> Vec<PathBuf> {
    let mut candidates = WalkDir::new(root)
        .max_depth(4)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            if entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("manifest.json")
            {
                Some(entry.into_path())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        let left_depth = left
            .strip_prefix(root)
            .map_or(usize::MAX, |path| path.components().count());
        let right_depth = right
            .strip_prefix(root)
            .map_or(usize::MAX, |path| path.components().count());
        left_depth
            .cmp(&right_depth)
            .then_with(|| left.to_string_lossy().cmp(&right.to_string_lossy()))
    });
    candidates
}

fn parse_json_manifest(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let normalized = normalize_json_bytes(&bytes)?;
    serde_json::from_slice(&normalized)
        .map_err(|error| format!("parse {}: {error}", path.display()))
}

fn manifest_path(root: &Path) -> Result<PathBuf, String> {
    let candidates = manifest_candidates(root);
    if candidates.is_empty() {
        return Err("icon-pack manifest.json was not found".into());
    }
    let mut failures = Vec::new();
    for candidate in candidates {
        match parse_json_manifest(&candidate) {
            Ok(value)
                if value.is_object()
                    && (value.get("Name").is_some()
                        || value.get("UUID").is_some()
                        || value.get("Identifier").is_some()) =>
            {
                return Ok(candidate);
            }
            Ok(_) => failures.push(format!(
                "{} is not an icon-pack manifest",
                candidate.display()
            )),
            Err(error) => failures.push(error),
        }
    }
    Err(format!(
        "no usable icon-pack manifest was found: {}",
        failures.join(" | ")
    ))
}

fn parse_manifest(root: &Path, fallback_hash: &str) -> Result<StoredPackMeta, String> {
    let manifest = manifest_path(root)?;
    let value = parse_json_manifest(&manifest)?;
    let name = value
        .get("Name")
        .and_then(Value::as_str)
        .unwrap_or("Stream Deck Icon Pack")
        .trim()
        .to_string();
    let version = value
        .get("Version")
        .and_then(Value::as_str)
        .unwrap_or("0.0.0")
        .trim()
        .to_string();
    let author = value
        .get("Author")
        .and_then(Value::as_str)
        .unwrap_or("Unknown")
        .trim()
        .to_string();
    let description = value
        .get("Description")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let declared_id = value
        .get("UUID")
        .or_else(|| value.get("Identifier"))
        .and_then(Value::as_str)
        .map(safe_id)
        .filter(|value| !value.is_empty());
    let derived = safe_id(&name);
    let id = declared_id
        .or_else(|| (!derived.is_empty()).then_some(derived))
        .unwrap_or_else(|| {
            format!(
                "icon-pack-{}",
                &fallback_hash[..12.min(fallback_hash.len())]
            )
        });
    validate_pack_id(&id)?;
    Ok(StoredPackMeta {
        id,
        name,
        version,
        author,
        description,
    })
}

fn is_icon_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "svg" | "png" | "jpg" | "jpeg" | "gif" | "webp"
            )
        })
        .unwrap_or(false)
}

fn icon_count(root: &Path) -> usize {
    WalkDir::new(root)
        .max_depth(8)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && is_icon_file(entry.path()))
        .filter(|entry| {
            entry.path().components().any(|component| {
                component
                    .as_os_str()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("icons")
            })
        })
        .count()
}

fn write_meta(root: &Path, meta: &StoredPackMeta) -> Result<(), String> {
    fs::write(
        root.join(META_FILE),
        serde_json::to_vec_pretty(meta).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn read_meta(root: &Path) -> Result<StoredPackMeta, String> {
    let path = root.join(META_FILE);
    if path.is_file() {
        return serde_json::from_slice(&fs::read(&path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string());
    }
    parse_manifest(root, "000000000000")
}

fn descriptor(root: PathBuf, active: bool) -> Result<IconPackDescriptor, String> {
    let meta = read_meta(&root)?;
    Ok(IconPackDescriptor {
        id: meta.id,
        name: meta.name,
        version: meta.version,
        author: meta.author,
        description: meta.description,
        item_count: icon_count(&root),
        root: root.display().to_string(),
        active,
    })
}

fn list_root(root: &Path, active: bool, output: &mut Vec<IconPackDescriptor>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Ok(item) = descriptor(path, active) {
            output.push(item);
        }
    }
}

fn normalized_tokens(value: &str) -> Vec<String> {
    let mut expanded = String::new();
    let mut previous_lower_or_digit = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() && previous_lower_or_digit {
                expanded.push(' ');
            }
            expanded.push(ch.to_ascii_lowercase());
            previous_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        } else {
            expanded.push(' ');
            previous_lower_or_digit = false;
        }
    }
    expanded
        .split_whitespace()
        .filter(|token| {
            !matches!(
                *token,
                "toggle" | "action" | "input" | "button" | "key" | "launch" | "close" | "open"
            )
        })
        .map(str::to_owned)
        .collect()
}

fn icon_candidate_score(icon: &IconMeta, action_id: Option<&str>, title: &str) -> usize {
    let title_tokens = normalized_tokens(title);
    let action_tokens = action_id.map(normalized_tokens).unwrap_or_default();
    let name_tokens = normalized_tokens(&icon.name);
    let path_tokens = normalized_tokens(&icon.path);
    let tag_tokens: Vec<String> = icon
        .tags
        .iter()
        .flat_map(|tag| normalized_tokens(tag))
        .collect();
    let mut score = 0usize;
    let normalized_title = title_tokens.join(" ");
    if !normalized_title.is_empty() && normalized_title == name_tokens.join(" ") {
        score += 200;
    }
    for token in title_tokens.iter().chain(action_tokens.iter()) {
        if token.len() < 2 {
            continue;
        }
        if name_tokens.iter().any(|candidate| candidate == token) {
            score += 40;
        }
        if tag_tokens.iter().any(|candidate| candidate == token) {
            score += 25;
        }
        if path_tokens.iter().any(|candidate| candidate == token) {
            score += 15;
        }
    }
    score
}

fn icon_metadata(root: &Path) -> Vec<IconMeta> {
    let Ok(manifest) = manifest_path(root) else {
        return Vec::new();
    };
    let Some(base) = manifest.parent() else {
        return Vec::new();
    };
    let metadata = base.join("icons.json");
    if !metadata.is_file() {
        return Vec::new();
    }
    fs::read(&metadata)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Vec<IconMeta>>(&bytes).ok())
        .unwrap_or_default()
}

pub(crate) fn active_icon_path(
    action_id: Option<&str>,
    title: &str,
    position: usize,
) -> Option<PathBuf> {
    let active = active_root().ok()?;
    let mut roots: Vec<PathBuf> = fs::read_dir(&active)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    roots.sort();
    for root in roots {
        let Ok(manifest) = manifest_path(&root) else {
            continue;
        };
        let Some(base) = manifest.parent() else {
            continue;
        };
        let icons_root = base.join("icons");
        let metadata = icon_metadata(&root);
        let best = metadata
            .iter()
            .map(|icon| (icon_candidate_score(icon, action_id, title), icon))
            .filter(|(score, _)| *score > 0)
            .max_by_key(|(score, _)| *score);
        if let Some((_, icon)) = best {
            let path = icons_root.join(&icon.path);
            if path.is_file() {
                return Some(path);
            }
        }
        let query_tokens: Vec<String> = normalized_tokens(title)
            .into_iter()
            .chain(action_id.map(normalized_tokens).unwrap_or_default())
            .collect();
        let mut fallback: Vec<PathBuf> = WalkDir::new(&icons_root)
            .max_depth(8)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file() && is_icon_file(entry.path()))
            .map(|entry| entry.into_path())
            .filter(|path| {
                let stem =
                    normalized_tokens(&path.file_stem().unwrap_or_default().to_string_lossy());
                query_tokens
                    .iter()
                    .any(|token| stem.iter().any(|candidate| candidate == token))
            })
            .collect();
        fallback.sort();
        if let Some(path) = fallback.into_iter().next() {
            return Some(path);
        }

        let mut all_icons: Vec<PathBuf> = metadata
            .iter()
            .map(|icon| icons_root.join(&icon.path))
            .filter(|path| path.is_file())
            .collect();
        if all_icons.is_empty() {
            all_icons = WalkDir::new(&icons_root)
                .max_depth(8)
                .follow_links(false)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|entry| entry.file_type().is_file() && is_icon_file(entry.path()))
                .map(|entry| entry.into_path())
                .collect();
        }
        all_icons.sort();
        if !all_icons.is_empty() {
            return Some(all_icons[position % all_icons.len()].clone());
        }
    }
    None
}

fn deactivate_other_packs(selected_id: &str) -> Result<(), String> {
    let active = active_root()?;
    let inactive = inactive_root()?;
    fs::create_dir_all(&active).map_err(|error| error.to_string())?;
    fs::create_dir_all(&inactive).map_err(|error| error.to_string())?;
    let Ok(entries) = fs::read_dir(&active) else {
        return Ok(());
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_dir() || entry.file_name().to_string_lossy() == selected_id {
            continue;
        }
        let destination = inactive.join(entry.file_name());
        if destination.exists() {
            fs::remove_dir_all(&destination).map_err(|error| error.to_string())?;
        }
        fs::rename(&path, &destination)
            .map_err(|error| format!("deactivate icon pack {}: {error}", path.display()))?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn icon_pack_list() -> Result<Vec<IconPackDescriptor>, String> {
    let active = active_root()?;
    let inactive = inactive_root()?;
    fs::create_dir_all(&active).map_err(|error| error.to_string())?;
    fs::create_dir_all(&inactive).map_err(|error| error.to_string())?;
    let mut packs = Vec::new();
    list_root(&active, true, &mut packs);
    list_root(&inactive, false, &mut packs);
    packs.sort_by_key(|pack| pack.name.to_lowercase());
    Ok(packs)
}

#[tauri::command]
pub(crate) fn icon_pack_install(path: String) -> Result<IconPackDescriptor, String> {
    let source = PathBuf::from(path);
    if !source.exists() {
        return Err(format!(
            "icon-pack source does not exist: {}",
            source.display()
        ));
    }
    if source.is_file()
        && !source
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("streamDeckIconPack"))
    {
        return Err(
            "icon-pack installer requires a .streamDeckIconPack package or extracted directory"
                .into(),
        );
    }

    let active = active_root()?;
    let inactive = inactive_root()?;
    fs::create_dir_all(&active).map_err(|error| error.to_string())?;
    fs::create_dir_all(&inactive).map_err(|error| error.to_string())?;
    let staging = home_dir()?.join(format!(
        ".local/share/opendeck/.icon-pack-stage-{}",
        Uuid::new_v4()
    ));
    let _ = fs::remove_dir_all(&staging);
    if source.is_dir() {
        copy_tree(&source, &staging)?;
    } else {
        extract_package(&source, &staging)?;
    }
    let hash = if source.is_file() {
        sha256_file(&source)?
    } else {
        format!(
            "{:x}",
            Sha256::digest(source.display().to_string().as_bytes())
        )
    };
    let meta = parse_manifest(&staging, &hash)?;
    write_meta(&staging, &meta)?;
    let active_path = active.join(&meta.id);
    let inactive_path = inactive.join(&meta.id);
    deactivate_other_packs(&meta.id)?;
    let _ = fs::remove_dir_all(&active_path);
    let _ = fs::remove_dir_all(&inactive_path);
    fs::rename(&staging, &active_path)
        .map_err(|error| format!("activate icon pack {}: {error}", meta.name))?;
    descriptor(active_path, true)
}

#[tauri::command]
pub(crate) fn icon_pack_set_active(pack_id: String, active: bool) -> Result<(), String> {
    validate_pack_id(&pack_id)?;
    if active {
        deactivate_other_packs(&pack_id)?;
    }
    let active_path = active_root()?.join(&pack_id);
    let inactive_path = inactive_root()?.join(&pack_id);
    fs::create_dir_all(active_root()?).map_err(|error| error.to_string())?;
    fs::create_dir_all(inactive_root()?).map_err(|error| error.to_string())?;
    let (source, destination) = if active {
        (&inactive_path, &active_path)
    } else {
        (&active_path, &inactive_path)
    };
    if destination.exists() {
        fs::remove_dir_all(destination).map_err(|error| error.to_string())?;
    }
    if source.exists() {
        fs::rename(source, destination).map_err(|error| error.to_string())?;
    } else if !destination.exists() {
        return Err(format!("icon pack {pack_id} is not installed"));
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn icon_pack_remove(pack_id: String) -> Result<(), String> {
    validate_pack_id(&pack_id)?;
    let active_path = active_root()?.join(&pack_id);
    let inactive_path = inactive_root()?.join(&pack_id);
    if active_path.exists() {
        fs::remove_dir_all(active_path).map_err(|error| error.to_string())?;
    }
    if inactive_path.exists() {
        fs::remove_dir_all(inactive_path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_ids_cannot_escape_pack_roots() {
        assert_eq!(safe_id("com.example.My Pack"), "com.example.my-pack");
        assert!(validate_pack_id("../escape").is_err());
        assert!(validate_pack_id("pack/escape").is_err());
        assert!(validate_pack_id("com.example.pack").is_ok());
    }
}
