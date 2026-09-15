use crate::coldstore::{
    ColdPackObjectRef, inspect_coldpack_manifest, lock_store_exclusive, lock_store_shared,
    object_path_for_hash, verify_object_reference,
};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

pub const QUARANTINE_SECONDS: u64 = 7 * 24 * 60 * 60;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ColdPackStoreReport {
    pub manifests: usize,
    pub logical_bytes: u64,
    pub unique_references: usize,
    pub active_objects: usize,
    pub active_bytes: u64,
    pub live_objects: usize,
    pub live_bytes: u64,
    pub orphan_objects: usize,
    pub orphan_bytes: u64,
    pub quarantine_objects: usize,
    pub quarantine_bytes: u64,
    pub expired_quarantine_objects: usize,
    pub expired_quarantine_bytes: u64,
    pub orphan_hashes: Vec<String>,
    pub purge_hashes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColdPackGcResult {
    pub report: ColdPackStoreReport,
    pub quarantined_objects: usize,
    pub quarantined_bytes: u64,
    pub purged_objects: usize,
    pub purged_bytes: u64,
}

#[derive(Debug, Clone)]
struct ObjectEntry {
    hash: String,
    path: PathBuf,
    bytes: u64,
}

#[derive(Debug, Clone)]
struct QuarantineEntry {
    hash: String,
    path: PathBuf,
    bytes: u64,
    timestamp: u64,
}

#[derive(Debug)]
struct Inventory {
    report: ColdPackStoreReport,
    active: Vec<ObjectEntry>,
    quarantine: Vec<QuarantineEntry>,
    live: HashMap<String, u64>,
}

pub fn coldpack_status(
    root: &Path,
    store: &Path,
    now_secs: u64,
) -> Result<ColdPackStoreReport, String> {
    let _lock = lock_store_shared(store)?;
    Ok(build_inventory(root, store, now_secs, false)?.report)
}

pub fn coldpack_audit(
    root: &Path,
    store: &Path,
    now_secs: u64,
) -> Result<ColdPackStoreReport, String> {
    let _lock = lock_store_shared(store)?;
    Ok(build_inventory(root, store, now_secs, true)?.report)
}

pub fn coldpack_gc_preview(
    root: &Path,
    store: &Path,
    now_secs: u64,
) -> Result<ColdPackStoreReport, String> {
    let _lock = lock_store_shared(store)?;
    Ok(build_inventory(root, store, now_secs, true)?.report)
}

pub fn coldpack_gc_apply(
    root: &Path,
    store: &Path,
    now_secs: u64,
) -> Result<ColdPackGcResult, String> {
    let _lock = lock_store_exclusive(store)?;
    let inventory = build_inventory(root, store, now_secs, true)?;
    let mut quarantined_objects = 0usize;
    let mut quarantined_bytes = 0u64;
    let mut purged_objects = 0usize;
    let mut purged_bytes = 0u64;

    let quarantine_root = store.join("quarantine").join(now_secs.to_string());
    let mut orphan_entries = Vec::new();
    for entry in &inventory.active {
        if !inventory.live.contains_key(&entry.hash) {
            let destination = quarantine_root
                .join(&entry.hash[..2])
                .join(format!("{}.zst", entry.hash));
            if fs::symlink_metadata(&destination).is_ok() {
                return Err(format!(
                    "AETHER_GUARD=BLOCKED ColdPack quarantine destination exists: {}",
                    destination.display()
                ));
            }
            orphan_entries.push((entry.clone(), destination));
        }
    }

    for (entry, destination) in orphan_entries {
        let parent = destination
            .parent()
            .ok_or_else(|| "ColdPack quarantine destination has no parent".to_owned())?;
        create_safe_dir(parent)?;
        fs::rename(&entry.path, &destination).map_err(|e| {
            format!(
                "cannot quarantine ColdPack object {} -> {}: {e}",
                entry.path.display(),
                destination.display()
            )
        })?;
        if let Some(source_parent) = entry.path.parent() {
            sync_directory(source_parent)?;
        }
        sync_directory(parent)?;
        quarantined_objects += 1;
        quarantined_bytes = quarantined_bytes
            .checked_add(entry.bytes)
            .ok_or_else(|| "ColdPack quarantined-byte overflow".to_owned())?;
    }

    for entry in &inventory.quarantine {
        let expired = now_secs.saturating_sub(entry.timestamp) >= QUARANTINE_SECONDS;
        if expired && !inventory.live.contains_key(&entry.hash) {
            fs::remove_file(&entry.path).map_err(|e| {
                format!(
                    "cannot purge expired ColdPack quarantine object {}: {e}",
                    entry.path.display()
                )
            })?;
            if let Some(parent) = entry.path.parent() {
                sync_directory(parent)?;
            }
            purged_objects += 1;
            purged_bytes = purged_bytes
                .checked_add(entry.bytes)
                .ok_or_else(|| "ColdPack purged-byte overflow".to_owned())?;
        }
    }
    remove_empty_directories(&store.join("objects"))?;
    remove_empty_directories(&store.join("quarantine"))?;

    Ok(ColdPackGcResult {
        report: inventory.report,
        quarantined_objects,
        quarantined_bytes,
        purged_objects,
        purged_bytes,
    })
}

fn build_inventory(
    root: &Path,
    store: &Path,
    now_secs: u64,
    verify_objects: bool,
) -> Result<Inventory, String> {
    if !root.is_dir() {
        return Err(format!("ForgeClean root is missing: {}", root.display()));
    }
    let manifests = collect_manifests(root, store)?;
    let mut live = HashMap::<String, u64>::new();
    let mut logical_bytes = 0u64;
    for manifest in &manifests {
        let info =
            inspect_coldpack_manifest(manifest, if verify_objects { Some(store) } else { None })?;
        logical_bytes = logical_bytes
            .checked_add(info.logical_bytes)
            .ok_or_else(|| "ColdPack logical-byte accounting overflow".to_owned())?;
        for reference in info.chunks {
            match live.get(&reference.sha256) {
                Some(length) if *length != reference.length => {
                    return Err(format!(
                        "AETHER_GUARD=BLOCKED conflicting ColdPack chunk lengths for {}",
                        reference.sha256
                    ));
                }
                Some(_) => {}
                None => {
                    live.insert(reference.sha256, reference.length);
                }
            }
        }
    }

    let active = inventory_active_objects(store)?;
    let mut active_by_hash = HashMap::new();
    let mut active_bytes = 0u64;
    for entry in &active {
        active_bytes = active_bytes
            .checked_add(entry.bytes)
            .ok_or_else(|| "ColdPack active-byte accounting overflow".to_owned())?;
        if active_by_hash.insert(entry.hash.clone(), entry).is_some() {
            return Err(format!(
                "AETHER_GUARD=BLOCKED duplicate active ColdPack object: {}",
                entry.hash
            ));
        }
    }

    let mut live_bytes = 0u64;
    for (hash, length) in &live {
        let entry = active_by_hash.get(hash).ok_or_else(|| {
            format!("AETHER_GUARD=BLOCKED referenced ColdPack object is missing: {hash}")
        })?;
        if verify_objects {
            let reference = ColdPackObjectRef {
                sha256: hash.clone(),
                length: *length,
            };
            let stored_bytes = verify_object_reference(store, &reference)?;
            if stored_bytes != entry.bytes {
                return Err(format!(
                    "AETHER_GUARD=BLOCKED ColdPack object size changed: {hash}"
                ));
            }
        }
        live_bytes = live_bytes
            .checked_add(entry.bytes)
            .ok_or_else(|| "ColdPack live-byte accounting overflow".to_owned())?;
    }

    let mut orphan_hashes = Vec::new();
    let mut orphan_bytes = 0u64;
    for entry in &active {
        if !live.contains_key(&entry.hash) {
            orphan_hashes.push(entry.hash.clone());
            orphan_bytes = orphan_bytes
                .checked_add(entry.bytes)
                .ok_or_else(|| "ColdPack orphan-byte accounting overflow".to_owned())?;
        }
    }
    orphan_hashes.sort();

    let quarantine = inventory_quarantine(store)?;
    let mut quarantine_bytes = 0u64;
    let mut expired_quarantine_objects = 0usize;
    let mut expired_quarantine_bytes = 0u64;
    let mut purge_hashes = Vec::new();
    for entry in &quarantine {
        quarantine_bytes = quarantine_bytes
            .checked_add(entry.bytes)
            .ok_or_else(|| "ColdPack quarantine-byte accounting overflow".to_owned())?;
        let expired = now_secs.saturating_sub(entry.timestamp) >= QUARANTINE_SECONDS;
        if expired && !live.contains_key(&entry.hash) {
            expired_quarantine_objects += 1;
            expired_quarantine_bytes = expired_quarantine_bytes
                .checked_add(entry.bytes)
                .ok_or_else(|| "ColdPack expired-byte accounting overflow".to_owned())?;
            purge_hashes.push(entry.hash.clone());
        }
    }
    purge_hashes.sort();

    let report = ColdPackStoreReport {
        manifests: manifests.len(),
        logical_bytes,
        unique_references: live.len(),
        active_objects: active.len(),
        active_bytes,
        live_objects: live.len(),
        live_bytes,
        orphan_objects: orphan_hashes.len(),
        orphan_bytes,
        quarantine_objects: quarantine.len(),
        quarantine_bytes,
        expired_quarantine_objects,
        expired_quarantine_bytes,
        orphan_hashes,
        purge_hashes,
    };
    Ok(Inventory {
        report,
        active,
        quarantine,
        live,
    })
}

fn collect_manifests(root: &Path, store: &Path) -> Result<Vec<PathBuf>, String> {
    fn walk(path: &Path, store: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
        if path == store {
            return Ok(());
        }
        for entry in fs::read_dir(path).map_err(|e| {
            format!(
                "cannot scan ColdPack manifests under {}: {e}",
                path.display()
            )
        })? {
            let entry = entry.map_err(|e| e.to_string())?;
            let child = entry.path();
            if child == store {
                continue;
            }
            let file_type = entry.file_type().map_err(|e| e.to_string())?;
            if file_type.is_symlink() {
                if child.extension() == Some(OsStr::new("fcoldpack")) {
                    return Err(format!(
                        "AETHER_GUARD=BLOCKED ColdPack manifest is symlink: {}",
                        child.display()
                    ));
                }
                continue;
            }
            if file_type.is_dir() {
                walk(&child, store, out)?;
            } else if file_type.is_file() && child.extension() == Some(OsStr::new("fcoldpack")) {
                out.push(child);
            }
        }
        Ok(())
    }

    let mut manifests = Vec::new();
    walk(root, store, &mut manifests)?;
    manifests.sort();
    Ok(manifests)
}

fn inventory_active_objects(store: &Path) -> Result<Vec<ObjectEntry>, String> {
    let root = store.join("objects");
    if !root.exists() {
        return Ok(Vec::new());
    }
    let root_meta = fs::symlink_metadata(&root).map_err(|e| e.to_string())?;
    if root_meta.file_type().is_symlink() || !root_meta.is_dir() {
        return Err(format!(
            "AETHER_GUARD=BLOCKED ColdPack objects root is invalid: {}",
            root.display()
        ));
    }
    let mut objects = Vec::new();
    for prefix_entry in fs::read_dir(&root).map_err(|e| e.to_string())? {
        let prefix_entry = prefix_entry.map_err(|e| e.to_string())?;
        let prefix_path = prefix_entry.path();
        let prefix_type = prefix_entry.file_type().map_err(|e| e.to_string())?;
        let prefix = prefix_entry.file_name().to_string_lossy().into_owned();
        if prefix_type.is_symlink() || !prefix_type.is_dir() || !is_hex_prefix(&prefix) {
            return Err(format!(
                "AETHER_GUARD=BLOCKED malformed ColdPack object prefix: {}",
                prefix_path.display()
            ));
        }
        for object_entry in fs::read_dir(&prefix_path).map_err(|e| e.to_string())? {
            let object_entry = object_entry.map_err(|e| e.to_string())?;
            let path = object_entry.path();
            let file_type = object_entry.file_type().map_err(|e| e.to_string())?;
            if file_type.is_symlink() || !file_type.is_file() {
                return Err(format!(
                    "AETHER_GUARD=BLOCKED malformed ColdPack object entry: {}",
                    path.display()
                ));
            }
            let hash = hash_from_object_name(&object_entry.file_name())?;
            if &hash[..2] != prefix.as_str() {
                return Err(format!(
                    "AETHER_GUARD=BLOCKED ColdPack object prefix mismatch: {}",
                    path.display()
                ));
            }
            let expected = object_path_for_hash(store, &hash)?;
            if expected != path {
                return Err(format!(
                    "AETHER_GUARD=BLOCKED ColdPack object path mismatch: {}",
                    path.display()
                ));
            }
            let bytes = fs::metadata(&path).map_err(|e| e.to_string())?.len();
            objects.push(ObjectEntry { hash, path, bytes });
        }
    }
    objects.sort_by(|a, b| a.hash.cmp(&b.hash));
    Ok(objects)
}

fn inventory_quarantine(store: &Path) -> Result<Vec<QuarantineEntry>, String> {
    let root = store.join("quarantine");
    if !root.exists() {
        return Ok(Vec::new());
    }
    let root_meta = fs::symlink_metadata(&root).map_err(|e| e.to_string())?;
    if root_meta.file_type().is_symlink() || !root_meta.is_dir() {
        return Err(format!(
            "AETHER_GUARD=BLOCKED ColdPack quarantine root is invalid: {}",
            root.display()
        ));
    }
    let mut objects = Vec::new();
    for stamp_entry in fs::read_dir(&root).map_err(|e| e.to_string())? {
        let stamp_entry = stamp_entry.map_err(|e| e.to_string())?;
        let stamp_path = stamp_entry.path();
        let stamp_type = stamp_entry.file_type().map_err(|e| e.to_string())?;
        if stamp_type.is_symlink() || !stamp_type.is_dir() {
            return Err(format!(
                "AETHER_GUARD=BLOCKED malformed ColdPack quarantine timestamp: {}",
                stamp_path.display()
            ));
        }
        let timestamp: u64 = stamp_entry
            .file_name()
            .to_string_lossy()
            .parse()
            .map_err(|_| {
                format!(
                    "AETHER_GUARD=BLOCKED invalid ColdPack quarantine timestamp: {}",
                    stamp_path.display()
                )
            })?;
        for prefix_entry in fs::read_dir(&stamp_path).map_err(|e| e.to_string())? {
            let prefix_entry = prefix_entry.map_err(|e| e.to_string())?;
            let prefix_path = prefix_entry.path();
            let prefix_type = prefix_entry.file_type().map_err(|e| e.to_string())?;
            let prefix = prefix_entry.file_name().to_string_lossy().into_owned();
            if prefix_type.is_symlink() || !prefix_type.is_dir() || !is_hex_prefix(&prefix) {
                return Err(format!(
                    "AETHER_GUARD=BLOCKED malformed ColdPack quarantine prefix: {}",
                    prefix_path.display()
                ));
            }
            for object_entry in fs::read_dir(&prefix_path).map_err(|e| e.to_string())? {
                let object_entry = object_entry.map_err(|e| e.to_string())?;
                let path = object_entry.path();
                let file_type = object_entry.file_type().map_err(|e| e.to_string())?;
                if file_type.is_symlink() || !file_type.is_file() {
                    return Err(format!(
                        "AETHER_GUARD=BLOCKED malformed ColdPack quarantine object: {}",
                        path.display()
                    ));
                }
                let hash = hash_from_object_name(&object_entry.file_name())?;
                if &hash[..2] != prefix.as_str() {
                    return Err(format!(
                        "AETHER_GUARD=BLOCKED ColdPack quarantine prefix mismatch: {}",
                        path.display()
                    ));
                }
                let bytes = fs::metadata(&path).map_err(|e| e.to_string())?.len();
                objects.push(QuarantineEntry {
                    hash,
                    path,
                    bytes,
                    timestamp,
                });
            }
        }
    }
    objects.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(objects)
}

fn hash_from_object_name(name: &OsStr) -> Result<String, String> {
    let name = name.to_string_lossy();
    let hash = name.strip_suffix(".zst").ok_or_else(|| {
        format!("AETHER_GUARD=BLOCKED ColdPack object does not end in .zst: {name}")
    })?;
    if hash.len() != 64
        || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        || hash.bytes().any(|byte| byte.is_ascii_uppercase())
    {
        return Err(format!(
            "AETHER_GUARD=BLOCKED invalid ColdPack object name: {name}"
        ));
    }
    Ok(hash.to_owned())
}

fn is_hex_prefix(value: &str) -> bool {
    value.len() == 2
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn create_safe_dir(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(format!(
                    "AETHER_GUARD=BLOCKED ColdPack GC path component is symlink: {}",
                    ancestor.display()
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    fs::create_dir_all(path).map_err(|e| {
        format!(
            "cannot create ColdPack GC directory {}: {e}",
            path.display()
        )
    })
}

fn remove_empty_directories(root: &Path) -> Result<bool, String> {
    if !root.exists() {
        return Ok(true);
    }
    let meta = fs::symlink_metadata(root).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(format!(
            "AETHER_GUARD=BLOCKED ColdPack cleanup directory invalid: {}",
            root.display()
        ));
    }
    let mut empty = true;
    let entries: Vec<PathBuf> = fs::read_dir(root)
        .map_err(|e| e.to_string())?
        .map(|entry| entry.map(|entry| entry.path()).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    for path in entries {
        let child = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if child.file_type().is_symlink() {
            return Err(format!(
                "AETHER_GUARD=BLOCKED ColdPack cleanup path is symlink: {}",
                path.display()
            ));
        }
        if child.is_dir() {
            if remove_empty_directories(&path)? {
                fs::remove_dir(&path).map_err(|e| e.to_string())?;
            } else {
                empty = false;
            }
        } else {
            empty = false;
        }
    }
    Ok(empty)
}

fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| format!("cannot fsync directory {}: {e}", path.display()))
}
