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

fn manifest_path(root: &Path) -> Result<PathBuf, String> {
    let direct = root.join("manifest.json");
    if direct.is_file() {
        return Ok(direct);
    }
    WalkDir::new(root)
        .max_depth(4)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .find(|entry| {
            entry.file_type().is_file()
                && entry
                    .file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("manifest.json")
        })
        .map(|entry| entry.into_path())
        .ok_or_else(|| "icon-pack manifest.json was not found".to_string())
}

fn parse_manifest(root: &Path, fallback_hash: &str) -> Result<StoredPackMeta, String> {
    let manifest = manifest_path(root)?;
    let value: Value =
        serde_json::from_slice(&fs::read(&manifest).map_err(|error| error.to_string())?)
            .map_err(|error| format!("parse {}: {error}", manifest.display()))?;
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
    let _ = fs::remove_dir_all(&active_path);
    let _ = fs::remove_dir_all(&inactive_path);
    fs::rename(&staging, &active_path)
        .map_err(|error| format!("activate icon pack {}: {error}", meta.name))?;
    descriptor(active_path, true)
}

#[tauri::command]
pub(crate) fn icon_pack_set_active(pack_id: String, active: bool) -> Result<(), String> {
    validate_pack_id(&pack_id)?;
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
