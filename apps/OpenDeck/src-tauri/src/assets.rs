use crate::editor::AssetRecord;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct AssetIndex {
    assets: Vec<AssetRecord>,
}

fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| "HOME is not set".into())
}

fn asset_root() -> Result<PathBuf, String> {
    Ok(home_dir()?.join(".config/opendeck-v2/assets"))
}

fn imported_root() -> Result<PathBuf, String> {
    Ok(asset_root()?.join("imported"))
}

fn index_path() -> Result<PathBuf, String> {
    Ok(asset_root()?.join("index.json"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn classify_image(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some(("image/png", "png"))
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(("image/jpeg", "jpg"))
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(("image/gif", "gif"))
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        Some(("image/webp", "webp"))
    } else {
        None
    }
}

fn safe_input_path(path: &Path) -> Result<(), String> {
    if path.components().any(|part| matches!(part, Component::ParentDir)) {
        return Err("asset path may not contain '..'".into());
    }
    if !path.is_file() {
        return Err("asset path is not a file".into());
    }
    Ok(())
}

fn read_index() -> Result<AssetIndex, String> {
    let path = index_path()?;
    if !path.exists() {
        return Ok(AssetIndex::default());
    }
    let text = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| "invalid index path".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temp = parent.join(format!(".assets-{}.tmp", Uuid::new_v4()));
    let mut file = File::create(&temp).map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    fs::rename(temp, path).map_err(|error| error.to_string())?;
    Ok(())
}

fn save_index(index: &AssetIndex) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(index).map_err(|error| error.to_string())?;
    write_atomic(&index_path()?, &bytes)
}

fn import_asset_at(path: &Path) -> Result<AssetRecord, String> {
    safe_input_path(path)?;
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let (mime, extension) = classify_image(&bytes).ok_or_else(|| "unsupported image format".to_string())?;
    let sha256 = sha256_hex(&bytes);
    let mut index = read_index()?;
    if let Some(existing) = index.assets.iter().find(|asset| asset.sha256 == sha256 && asset.source == "imported") {
        return Ok(existing.clone());
    }
    let root = imported_root()?;
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let destination = root.join(format!("{sha256}.{extension}"));
    if !destination.exists() {
        write_atomic(&destination, &bytes)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&destination, fs::Permissions::from_mode(0o600)).map_err(|error| error.to_string())?;
        }
    }
    let record = AssetRecord {
        id: format!("asset-{sha256}"),
        name: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
        path: destination.display().to_string(),
        source: "imported".into(),
        mime: mime.into(),
        sha256,
    };
    index.assets.push(record.clone());
    save_index(&index)?;
    Ok(record)
}

fn icon_pack_assets() -> Result<Vec<AssetRecord>, String> {
    let root = home_dir()?.join(".local/share/opendeck/icon-packs");
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut assets = Vec::new();
    for entry in WalkDir::new(&root).max_depth(6).follow_links(false).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Ok(bytes) = fs::read(path) else { continue };
        let Some((mime, _)) = classify_image(&bytes) else { continue };
        let sha256 = sha256_hex(&bytes);
        assets.push(AssetRecord {
            id: format!("icon-pack-{sha256}"),
            name: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
            path: path.display().to_string(),
            source: "icon-pack".into(),
            mime: mime.into(),
            sha256,
        });
    }
    assets.sort_by(|a, b| a.path.cmp(&b.path));
    assets.dedup_by(|a, b| a.sha256 == b.sha256 && a.path == b.path);
    Ok(assets)
}

#[tauri::command]
pub(crate) fn editor_import_asset(path: String) -> Result<AssetRecord, String> {
    import_asset_at(Path::new(&path))
}

#[tauri::command]
pub(crate) fn editor_list_assets() -> Result<Vec<AssetRecord>, String> {
    let mut assets = read_index()?.assets;
    assets.extend(icon_pack_assets()?);
    assets.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(assets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_supported_magic() {
        assert_eq!(classify_image(&[0x89, b'P', b'N', b'G', 0, 0]).unwrap().0, "image/png");
        assert_eq!(classify_image(&[0xff, 0xd8, 0xff, 0]).unwrap().0, "image/jpeg");
        assert_eq!(classify_image(b"GIF89a...").unwrap().0, "image/gif");
        assert_eq!(classify_image(b"RIFFxxxxWEBP").unwrap().0, "image/webp");
        assert!(classify_image(b"plain text").is_none());
    }

    #[test]
    fn hash_is_stable() {
        assert_eq!(sha256_hex(b"OpenDeck"), sha256_hex(b"OpenDeck"));
        assert_ne!(sha256_hex(b"OpenDeck"), sha256_hex(b"OpenDeck2"));
    }

    #[test]
    fn traversal_is_rejected() {
        assert!(safe_input_path(Path::new("../icon.png")).is_err());
    }
}
