use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{Cursor, Read, Write};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const LEGACY_COLD_SUFFIX: &str = ".fcold.tar.zst";
pub const COLDPACK_SUFFIX: &str = ".fcoldpack";
const ZSTD_LEVEL: i32 = 19;
const HASH_BUFFER_BYTES: usize = 64 * 1024;
const CDC_MIN_BYTES: usize = 64 * 1024;
const CDC_AVG_MASK: u64 = (256 * 1024) as u64 - 1;
const CDC_MAX_BYTES: usize = 1024 * 1024;
const COLDPACK_FORMAT: &str = "FORGECLEAN_COLDPACK";
const COLDPACK_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColdArchiveRecord {
    pub source: PathBuf,
    pub archive_path: PathBuf,
    pub checksum_path: PathBuf,
    pub archive_sha256: String,
    pub source_sha256: String,
    pub logical_bytes: u64,
    pub stored_bytes: u64,
    pub chunks_written: usize,
    pub chunks_reused: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ColdPackManifest {
    format: String,
    version: u32,
    source_name_hex: String,
    source_sha256: String,
    logical_bytes: u64,
    entries: Vec<ColdPackEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ColdPackEntry {
    path_hex: String,
    kind: ColdPackEntryKind,
    mode: u32,
    size: u64,
    chunks: Vec<ColdPackChunkRef>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum ColdPackEntryKind {
    Directory,
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ColdPackChunkRef {
    sha256: String,
    length: u64,
}

#[derive(Debug, Default)]
struct PackStats {
    logical_bytes: u64,
    stored_bytes: u64,
    chunks_written: usize,
    chunks_reused: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ColdPackObjectRef {
    pub sha256: String,
    pub length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ColdPackManifestInfo {
    pub logical_bytes: u64,
    pub chunks: Vec<ColdPackObjectRef>,
}

pub(crate) fn lock_store_exclusive(store_dir: &Path) -> Result<File, String> {
    let file = open_store_lock(store_dir)?;
    file.lock()
        .map_err(|e| format!("cannot acquire exclusive ColdPack store lock: {e}"))?;
    Ok(file)
}

pub(crate) fn lock_store_shared(store_dir: &Path) -> Result<File, String> {
    let file = open_store_lock(store_dir)?;
    file.lock_shared()
        .map_err(|e| format!("cannot acquire shared ColdPack store lock: {e}"))?;
    Ok(file)
}

fn open_store_lock(store_dir: &Path) -> Result<File, String> {
    ensure_directory_no_symlink(store_dir)?;
    let path = store_dir.join("maintenance.lock");
    match fs::symlink_metadata(&path) {
        Ok(meta) if meta.file_type().is_symlink() || !meta.is_file() => {
            return Err(format!(
                "AETHER_GUARD=BLOCKED ColdPack lock is not a regular file: {}",
                path.display()
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|e| format!("cannot open ColdPack store lock {}: {e}", path.display()))?;
    let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(format!(
            "AETHER_GUARD=BLOCKED ColdPack lock changed type: {}",
            path.display()
        ));
    }
    Ok(file)
}

pub(crate) fn inspect_coldpack_manifest(
    archive_path: &Path,
    verify_store: Option<&Path>,
) -> Result<ColdPackManifestInfo, String> {
    verify_checksum_sidecar(archive_path)?;
    let bytes = fs::read(archive_path).map_err(|e| {
        format!(
            "cannot read ColdPack manifest {}: {e}",
            archive_path.display()
        )
    })?;
    let manifest: ColdPackManifest = serde_json::from_slice(&bytes).map_err(|e| {
        format!(
            "cannot parse ColdPack manifest {}: {e}",
            archive_path.display()
        )
    })?;
    validate_manifest(&manifest)?;
    if let Some(store_dir) = verify_store
        && hash_manifest_content(&manifest, store_dir)? != manifest.source_sha256
    {
        return Err(format!(
            "AETHER_GUARD=BLOCKED ColdPack manifest content hash mismatch: {}",
            archive_path.display()
        ));
    }
    let chunks = manifest
        .entries
        .iter()
        .flat_map(|entry| entry.chunks.iter())
        .map(|chunk| ColdPackObjectRef {
            sha256: chunk.sha256.to_ascii_lowercase(),
            length: chunk.length,
        })
        .collect();
    Ok(ColdPackManifestInfo {
        logical_bytes: manifest.logical_bytes,
        chunks,
    })
}

pub(crate) fn object_path_for_hash(store_dir: &Path, hash: &str) -> Result<PathBuf, String> {
    chunk_object_path(store_dir, hash)
}

pub(crate) fn verify_object_reference(
    store_dir: &Path,
    reference: &ColdPackObjectRef,
) -> Result<u64, String> {
    let path = chunk_object_path(store_dir, &reference.sha256)?;
    verify_chunk_object(&path, &reference.sha256, reference.length)?;
    fs::metadata(&path)
        .map(|meta| meta.len())
        .map_err(|e| format!("cannot inspect ColdPack object {}: {e}", path.display()))
}

pub fn archive_to_cold(
    source: &Path,
    cold_dir: &Path,
    label: &str,
) -> Result<ColdArchiveRecord, String> {
    let store = cold_dir.join(".coldpack-store");
    archive_to_coldpack(source, cold_dir, &store, label)
}

pub fn archive_to_coldpack(
    source: &Path,
    cold_dir: &Path,
    store_dir: &Path,
    label: &str,
) -> Result<ColdArchiveRecord, String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|e| format!("cannot inspect cold source {}: {e}", source.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "AETHER_GUARD=BLOCKED cold source is symlink: {}",
            source.display()
        ));
    }
    reject_symlinks_recursive(source)?;
    ensure_directory_no_symlink(cold_dir)?;
    ensure_directory_no_symlink(store_dir)?;
    let _store_lock = lock_store_exclusive(store_dir)?;

    let label = sanitize_label(label);
    if label.is_empty() {
        return Err("cold archive label is empty".to_owned());
    }
    let archive_path = cold_dir.join(format!("{label}{COLDPACK_SUFFIX}"));
    let checksum_path = checksum_path_for(&archive_path);
    if archive_path.exists() || checksum_path.exists() {
        return Err(format!(
            "cold archive already exists: {}",
            archive_path.display()
        ));
    }

    let source_hash_before = hash_tree(source)?;
    let root_name = source
        .file_name()
        .ok_or_else(|| "cold source has no filename".to_owned())?;
    let mut entries = Vec::new();
    let mut stats = PackStats::default();
    collect_coldpack_entries(
        source,
        Path::new(root_name),
        store_dir,
        &mut entries,
        &mut stats,
    )?;

    let source_hash_after = hash_tree(source)?;
    if source_hash_before != source_hash_after {
        return Err(format!(
            "AETHER_GUARD=BLOCKED source changed during ColdPack creation; source preserved: {}",
            source.display()
        ));
    }

    let manifest = ColdPackManifest {
        format: COLDPACK_FORMAT.to_owned(),
        version: COLDPACK_VERSION,
        source_name_hex: os_str_to_hex(root_name),
        source_sha256: source_hash_before.clone(),
        logical_bytes: stats.logical_bytes,
        entries,
    };
    let manifest_hash = hash_manifest_content(&manifest, store_dir)?;
    if manifest_hash != source_hash_before {
        return Err(
            "AETHER_GUARD=BLOCKED ColdPack chunk snapshot does not match source; source preserved"
                .to_owned(),
        );
    }
    let manifest_bytes = serde_json::to_vec(&manifest)
        .map_err(|e| format!("cannot serialize ColdPack manifest: {e}"))?;
    let archive_hash = sha256_bytes(&manifest_bytes);
    atomic_write_file(&archive_path, &manifest_bytes)?;
    if hash_file(&archive_path)? != archive_hash {
        let _ = fs::remove_file(&archive_path);
        return Err(
            "AETHER_GUARD=BLOCKED committed ColdPack manifest hash mismatch; source preserved"
                .to_owned(),
        );
    }
    write_checksum(&checksum_path, &archive_hash, &archive_path)?;
    sync_directory(cold_dir)?;

    if hash_tree(source)? != source_hash_before {
        let _ = fs::remove_file(&checksum_path);
        let _ = fs::remove_file(&archive_path);
        let _ = sync_directory(cold_dir);
        return Err(format!(
            "AETHER_GUARD=BLOCKED source changed before ColdPack commit; source preserved: {}",
            source.display()
        ));
    }

    if metadata.is_dir() {
        fs::remove_dir_all(source).map_err(|e| {
            format!(
                "verified ColdPack retained but source directory delete failed {}: {e}",
                source.display()
            )
        })?;
    } else {
        fs::remove_file(source).map_err(|e| {
            format!(
                "verified ColdPack retained but source delete failed {}: {e}",
                source.display()
            )
        })?;
    }

    Ok(ColdArchiveRecord {
        source: source.to_path_buf(),
        archive_path,
        checksum_path,
        archive_sha256: archive_hash,
        source_sha256: source_hash_before,
        logical_bytes: stats.logical_bytes,
        stored_bytes: stats.stored_bytes,
        chunks_written: stats.chunks_written,
        chunks_reused: stats.chunks_reused,
    })
}

pub fn restore_cold_archive(
    archive_path: &Path,
    destination_root: &Path,
) -> Result<Vec<PathBuf>, String> {
    if is_coldpack_path(archive_path) {
        let store = archive_path
            .parent()
            .ok_or_else(|| "ColdPack manifest has no parent".to_owned())?
            .join(".coldpack-store");
        restore_coldpack_archive(archive_path, &store, destination_root)
    } else {
        restore_legacy_cold_archive(archive_path, destination_root)
    }
}

pub fn restore_cold_archive_with_store(
    archive_path: &Path,
    store_dir: &Path,
    destination_root: &Path,
) -> Result<Vec<PathBuf>, String> {
    if is_coldpack_path(archive_path) {
        restore_coldpack_archive(archive_path, store_dir, destination_root)
    } else {
        restore_legacy_cold_archive(archive_path, destination_root)
    }
}

pub fn restore_coldpack_archive(
    archive_path: &Path,
    store_dir: &Path,
    destination_root: &Path,
) -> Result<Vec<PathBuf>, String> {
    let _store_lock = lock_store_shared(store_dir)?;
    verify_checksum_sidecar(archive_path)?;
    let bytes = fs::read(archive_path).map_err(|e| {
        format!(
            "cannot read ColdPack manifest {}: {e}",
            archive_path.display()
        )
    })?;
    let manifest: ColdPackManifest = serde_json::from_slice(&bytes).map_err(|e| {
        format!(
            "cannot parse ColdPack manifest {}: {e}",
            archive_path.display()
        )
    })?;
    validate_manifest(&manifest)?;
    ensure_directory_no_symlink(store_dir)?;
    if hash_manifest_content(&manifest, store_dir)? != manifest.source_sha256 {
        return Err("AETHER_GUARD=BLOCKED ColdPack manifest content hash mismatch".to_owned());
    }

    fs::create_dir_all(destination_root).map_err(|e| {
        format!(
            "cannot create restore root {}: {e}",
            destination_root.display()
        )
    })?;
    let restore_root = fs::canonicalize(destination_root).map_err(|e| {
        format!(
            "cannot canonicalize restore root {}: {e}",
            destination_root.display()
        )
    })?;
    let temp = restore_root.join(format!(
        ".forgeclean-coldpack-restore-{}-{}",
        std::process::id(),
        unique_stamp()?
    ));
    fs::create_dir(&temp)
        .map_err(|e| format!("cannot create restore staging {}: {e}", temp.display()))?;

    let restore_result = restore_manifest_into(&manifest, store_dir, &temp);
    if let Err(error) = restore_result {
        let _ = fs::remove_dir_all(&temp);
        return Err(error);
    }

    let mut staged = Vec::new();
    for entry in fs::read_dir(&temp).map_err(|e| e.to_string())? {
        staged.push(entry.map_err(|e| e.to_string())?.path());
    }
    staged.sort();
    if staged.is_empty() {
        let _ = fs::remove_dir_all(&temp);
        return Err("ColdPack contains no restorable entries".to_owned());
    }
    for item in &staged {
        let name = item
            .file_name()
            .ok_or_else(|| "restore entry has no filename".to_owned())?;
        let final_path = restore_root.join(name);
        if final_path.exists() || fs::symlink_metadata(&final_path).is_ok() {
            let _ = fs::remove_dir_all(&temp);
            return Err(format!(
                "restore destination already exists: {}",
                final_path.display()
            ));
        }
    }

    let mut restored = Vec::new();
    for item in staged {
        let name = item
            .file_name()
            .ok_or_else(|| "restore entry has no filename".to_owned())?
            .to_owned();
        let final_path = restore_root.join(name);
        fs::rename(&item, &final_path)
            .map_err(|e| format!("cannot commit restored entry {}: {e}", final_path.display()))?;
        restored.push(final_path);
    }
    fs::remove_dir(&temp)
        .map_err(|e| format!("cannot remove restore staging {}: {e}", temp.display()))?;
    sync_directory(&restore_root)?;
    Ok(restored)
}

pub fn coldpack_object_count(store_dir: &Path) -> Result<usize, String> {
    let objects = store_dir.join("objects");
    if !objects.exists() {
        return Ok(0);
    }
    count_files_recursive(&objects)
}

pub fn archive_to_legacy_cold(
    source: &Path,
    cold_dir: &Path,
    label: &str,
) -> Result<ColdArchiveRecord, String> {
    let metadata = fs::symlink_metadata(source)
        .map_err(|e| format!("cannot inspect cold source {}: {e}", source.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "AETHER_GUARD=BLOCKED cold source is symlink: {}",
            source.display()
        ));
    }
    reject_symlinks_recursive(source)?;
    ensure_directory_no_symlink(cold_dir)?;
    let label = sanitize_label(label);
    if label.is_empty() {
        return Err("cold archive label is empty".to_owned());
    }
    let archive_path = cold_dir.join(format!("{label}{LEGACY_COLD_SUFFIX}"));
    let checksum_path = checksum_path_for(&archive_path);
    if archive_path.exists() || checksum_path.exists() {
        return Err(format!(
            "cold archive already exists: {}",
            archive_path.display()
        ));
    }

    let source_hash_before = hash_tree(source)?;
    let temp_path = cold_dir.join(format!(
        ".{label}.{}.{}.tmp",
        std::process::id(),
        unique_stamp()?
    ));
    let temp_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
        .map_err(|e| format!("cannot create cold temp {}: {e}", temp_path.display()))?;
    if let Err(error) = write_legacy_archive(source, temp_file) {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }
    if hash_tree(source)? != source_hash_before {
        let _ = fs::remove_file(&temp_path);
        return Err(format!(
            "AETHER_GUARD=BLOCKED source changed during legacy archive; source preserved: {}",
            source.display()
        ));
    }
    let archive_hash = hash_file(&temp_path)?;
    fs::rename(&temp_path, &archive_path)
        .map_err(|e| format!("cannot commit cold archive {}: {e}", archive_path.display()))?;
    File::open(&archive_path)
        .and_then(|file| file.sync_all())
        .map_err(|e| format!("cannot fsync cold archive {}: {e}", archive_path.display()))?;
    write_checksum(&checksum_path, &archive_hash, &archive_path)?;
    sync_directory(cold_dir)?;
    if hash_tree(source)? != source_hash_before {
        return Err(format!(
            "AETHER_GUARD=BLOCKED source changed before legacy archive commit; source preserved: {}",
            source.display()
        ));
    }
    let logical_bytes = logical_size(source)?;
    if metadata.is_dir() {
        fs::remove_dir_all(source).map_err(|e| e.to_string())?;
    } else {
        fs::remove_file(source).map_err(|e| e.to_string())?;
    }
    Ok(ColdArchiveRecord {
        source: source.to_path_buf(),
        archive_path,
        checksum_path,
        archive_sha256: archive_hash,
        source_sha256: source_hash_before,
        logical_bytes,
        stored_bytes: 0,
        chunks_written: 0,
        chunks_reused: 0,
    })
}

fn collect_coldpack_entries(
    path: &Path,
    relative: &Path,
    store_dir: &Path,
    entries: &mut Vec<ColdPackEntry>,
    stats: &mut PackStats,
) -> Result<(), String> {
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() {
        return Err(format!(
            "AETHER_GUARD=BLOCKED symlink inside cold source: {}",
            path.display()
        ));
    }
    if meta.is_dir() {
        entries.push(ColdPackEntry {
            path_hex: path_to_hex(relative),
            kind: ColdPackEntryKind::Directory,
            mode: meta.mode(),
            size: 0,
            chunks: Vec::new(),
        });
        let mut children = Vec::new();
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            children.push(entry.map_err(|e| e.to_string())?.path());
        }
        children.sort();
        for child in children {
            let name = child
                .file_name()
                .ok_or_else(|| "child has no filename".to_owned())?;
            collect_coldpack_entries(&child, &relative.join(name), store_dir, entries, stats)?;
        }
    } else if meta.is_file() {
        let mut chunks = Vec::new();
        chunk_file(path, |data| {
            let hash = sha256_bytes(data);
            let (written, stored_bytes) = ensure_chunk_object(store_dir, &hash, data)?;
            if written {
                stats.chunks_written += 1;
                stats.stored_bytes += stored_bytes;
            } else {
                stats.chunks_reused += 1;
            }
            chunks.push(ColdPackChunkRef {
                sha256: hash,
                length: data.len() as u64,
            });
            Ok(())
        })?;
        stats.logical_bytes += meta.len();
        entries.push(ColdPackEntry {
            path_hex: path_to_hex(relative),
            kind: ColdPackEntryKind::File,
            mode: meta.mode(),
            size: meta.len(),
            chunks,
        });
    } else {
        return Err(format!(
            "unsupported cold source entry type: {}",
            path.display()
        ));
    }
    Ok(())
}

fn chunk_file<F>(path: &Path, mut on_chunk: F) -> Result<(), String>
where
    F: FnMut(&[u8]) -> Result<(), String>,
{
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut read_buffer = vec![0u8; HASH_BUFFER_BYTES];
    let mut chunk = Vec::with_capacity(CDC_MAX_BYTES);
    let mut gear = 0u64;
    loop {
        let n = file.read(&mut read_buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        for byte in &read_buffer[..n] {
            chunk.push(*byte);
            gear = gear.rotate_left(1).wrapping_add(gear_value(*byte));
            let boundary = chunk.len() >= CDC_MIN_BYTES && (gear & CDC_AVG_MASK) == 0;
            if boundary || chunk.len() >= CDC_MAX_BYTES {
                on_chunk(&chunk)?;
                chunk.clear();
                gear = 0;
            }
        }
    }
    if !chunk.is_empty() {
        on_chunk(&chunk)?;
    }
    Ok(())
}

fn gear_value(byte: u8) -> u64 {
    let mut value = u64::from(byte).wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn ensure_chunk_object(store_dir: &Path, hash: &str, data: &[u8]) -> Result<(bool, u64), String> {
    let object_path = chunk_object_path(store_dir, hash)?;
    let parent = object_path
        .parent()
        .ok_or_else(|| "chunk object has no parent".to_owned())?;
    ensure_directory_no_symlink(parent)?;
    if object_path.exists() {
        verify_chunk_object(&object_path, hash, data.len() as u64)?;
        return Ok((false, 0));
    }

    let compressed = zstd::stream::encode_all(Cursor::new(data), ZSTD_LEVEL)
        .map_err(|e| format!("cannot compress ColdPack chunk {hash}: {e}"))?;
    let temp = parent.join(format!(
        ".{hash}.{}.{}.tmp",
        std::process::id(),
        unique_stamp()?
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| format!("cannot create ColdPack temp object {}: {e}", temp.display()))?;
    file.write_all(&compressed).map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;

    match fs::hard_link(&temp, &object_path) {
        Ok(()) => {
            fs::remove_file(&temp).map_err(|e| e.to_string())?;
            sync_directory(parent)?;
            verify_chunk_object(&object_path, hash, data.len() as u64)?;
            Ok((true, compressed.len() as u64))
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let _ = fs::remove_file(&temp);
            verify_chunk_object(&object_path, hash, data.len() as u64)?;
            Ok((false, 0))
        }
        Err(error) => {
            let _ = fs::remove_file(&temp);
            Err(format!(
                "cannot commit ColdPack chunk {}: {error}",
                object_path.display()
            ))
        }
    }
}

fn chunk_object_path(store_dir: &Path, hash: &str) -> Result<PathBuf, String> {
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("ColdPack chunk id is not SHA-256".to_owned());
    }
    let lower = hash.to_ascii_lowercase();
    Ok(store_dir
        .join("objects")
        .join(&lower[..2])
        .join(format!("{lower}.zst")))
}

fn verify_chunk_object(
    path: &Path,
    expected_hash: &str,
    expected_len: u64,
) -> Result<Vec<u8>, String> {
    let meta = fs::symlink_metadata(path)
        .map_err(|e| format!("cannot inspect ColdPack object {}: {e}", path.display()))?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(format!(
            "AETHER_GUARD=BLOCKED ColdPack object is not a regular file: {}",
            path.display()
        ));
    }
    let file = File::open(path)
        .map_err(|e| format!("cannot open ColdPack object {}: {e}", path.display()))?;
    let data = zstd::stream::decode_all(file)
        .map_err(|e| format!("cannot decompress ColdPack object {}: {e}", path.display()))?;
    if data.len() as u64 != expected_len {
        return Err(format!(
            "AETHER_GUARD=BLOCKED ColdPack object length mismatch: {}",
            path.display()
        ));
    }
    if sha256_bytes(&data) != expected_hash.to_ascii_lowercase() {
        return Err(format!(
            "AETHER_GUARD=BLOCKED ColdPack object hash mismatch: {}",
            path.display()
        ));
    }
    Ok(data)
}

fn hash_manifest_content(manifest: &ColdPackManifest, store_dir: &Path) -> Result<String, String> {
    let mut hasher = Sha256::new();
    for entry in &manifest.entries {
        let relative = hex_to_path(&entry.path_hex)?;
        validate_archive_path(&relative)?;
        hasher.update(relative.as_os_str().as_bytes());
        match entry.kind {
            ColdPackEntryKind::Directory => hasher.update(b"D"),
            ColdPackEntryKind::File => {
                hasher.update(b"F");
                let mut total = 0u64;
                for chunk in &entry.chunks {
                    let path = chunk_object_path(store_dir, &chunk.sha256)?;
                    let data = verify_chunk_object(&path, &chunk.sha256, chunk.length)?;
                    hasher.update(&data);
                    total = total
                        .checked_add(chunk.length)
                        .ok_or_else(|| "ColdPack file length overflow".to_owned())?;
                }
                if total != entry.size {
                    return Err(
                        "AETHER_GUARD=BLOCKED ColdPack manifest file-size mismatch".to_owned()
                    );
                }
            }
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn restore_manifest_into(
    manifest: &ColdPackManifest,
    store_dir: &Path,
    staging: &Path,
) -> Result<(), String> {
    let mut directories = Vec::new();
    let mut files = Vec::new();
    for entry in &manifest.entries {
        let relative = hex_to_path(&entry.path_hex)?;
        validate_archive_path(&relative)?;
        match entry.kind {
            ColdPackEntryKind::Directory => directories.push((relative, entry)),
            ColdPackEntryKind::File => files.push((relative, entry)),
        }
    }
    directories.sort_by_key(|(path, _)| path.components().count());
    for (relative, _) in &directories {
        fs::create_dir_all(staging.join(relative)).map_err(|e| e.to_string())?;
    }
    files.sort_by(|(a, _), (b, _)| a.cmp(b));
    for (relative, entry) in &files {
        let destination = staging.join(relative);
        let parent = destination
            .parent()
            .ok_or_else(|| "ColdPack file has no parent".to_owned())?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let mut out = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)
            .map_err(|e| format!("cannot create restored file {}: {e}", destination.display()))?;
        let mut written = 0u64;
        for chunk in &entry.chunks {
            let object = chunk_object_path(store_dir, &chunk.sha256)?;
            let data = verify_chunk_object(&object, &chunk.sha256, chunk.length)?;
            out.write_all(&data).map_err(|e| e.to_string())?;
            written += data.len() as u64;
        }
        out.flush().map_err(|e| e.to_string())?;
        out.sync_all().map_err(|e| e.to_string())?;
        if written != entry.size {
            return Err(format!(
                "AETHER_GUARD=BLOCKED restored file size mismatch: {}",
                destination.display()
            ));
        }
    }
    let restored_source = staging.join(hex_to_path(&manifest.source_name_hex)?);
    if hash_tree(&restored_source)? != manifest.source_sha256 {
        return Err("AETHER_GUARD=BLOCKED restored ColdPack source hash mismatch".to_owned());
    }
    for (relative, entry) in &files {
        fs::set_permissions(
            staging.join(relative),
            fs::Permissions::from_mode(entry.mode & 0o7777),
        )
        .map_err(|e| e.to_string())?;
    }
    directories.sort_by_key(|(path, _)| std::cmp::Reverse(path.components().count()));
    for (relative, entry) in directories {
        fs::set_permissions(
            staging.join(relative),
            fs::Permissions::from_mode(entry.mode & 0o7777),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn validate_manifest(manifest: &ColdPackManifest) -> Result<(), String> {
    if manifest.format != COLDPACK_FORMAT || manifest.version != COLDPACK_VERSION {
        return Err(format!(
            "unsupported ColdPack format/version: {}/{}",
            manifest.format, manifest.version
        ));
    }
    if manifest.entries.is_empty() {
        return Err("ColdPack manifest contains no entries".to_owned());
    }
    let source_name = hex_to_path(&manifest.source_name_hex)?;
    validate_archive_path(&source_name)?;
    if source_name.components().count() != 1 {
        return Err("AETHER_GUARD=BLOCKED ColdPack source name is not one component".to_owned());
    }
    let mut logical = 0u64;
    for entry in &manifest.entries {
        let path = hex_to_path(&entry.path_hex)?;
        validate_archive_path(&path)?;
        match entry.kind {
            ColdPackEntryKind::Directory => {
                if entry.size != 0 || !entry.chunks.is_empty() {
                    return Err(
                        "AETHER_GUARD=BLOCKED ColdPack directory carries file data".to_owned()
                    );
                }
            }
            ColdPackEntryKind::File => {
                let mut chunk_bytes = 0u64;
                for chunk in &entry.chunks {
                    if chunk.sha256.len() != 64
                        || !chunk.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
                    {
                        return Err(
                            "AETHER_GUARD=BLOCKED ColdPack chunk id is not SHA-256".to_owned()
                        );
                    }
                    chunk_bytes = chunk_bytes
                        .checked_add(chunk.length)
                        .ok_or_else(|| "ColdPack chunk length overflow".to_owned())?;
                }
                if chunk_bytes != entry.size {
                    return Err(
                        "AETHER_GUARD=BLOCKED ColdPack manifest file-size mismatch".to_owned()
                    );
                }
                logical = logical
                    .checked_add(entry.size)
                    .ok_or_else(|| "ColdPack logical-byte overflow".to_owned())?;
            }
        }
    }
    if logical != manifest.logical_bytes {
        return Err("AETHER_GUARD=BLOCKED ColdPack logical-byte mismatch".to_owned());
    }
    Ok(())
}

fn is_coldpack_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == OsStr::new("fcoldpack"))
}

fn ensure_directory_no_symlink(path: &Path) -> Result<(), String> {
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(format!(
                    "AETHER_GUARD=BLOCKED ColdPack directory component is symlink: {}",
                    ancestor.display()
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    fs::create_dir_all(path).map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    Ok(())
}

fn atomic_write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "atomic write path has no parent".to_owned())?;
    ensure_directory_no_symlink(parent)?;
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| "atomic write filename is not UTF-8".to_owned())?;
    let temp = parent.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        unique_stamp()?
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| format!("cannot create temporary file {}: {e}", temp.display()))?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    if path.exists() || fs::symlink_metadata(path).is_ok() {
        let _ = fs::remove_file(&temp);
        return Err(format!("destination already exists: {}", path.display()));
    }
    fs::rename(&temp, path).map_err(|e| {
        let _ = fs::remove_file(&temp);
        format!("cannot commit {}: {e}", path.display())
    })?;
    sync_directory(parent)
}

fn count_files_recursive(path: &Path) -> Result<usize, String> {
    let mut count = 0usize;
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let meta = entry.file_type().map_err(|e| e.to_string())?;
        if meta.is_dir() {
            count += count_files_recursive(&entry.path())?;
        } else if meta.is_file() {
            count += 1;
        }
    }
    Ok(count)
}

fn write_legacy_archive(source: &Path, file: File) -> Result<(), String> {
    let mut encoder = zstd::Encoder::new(file, ZSTD_LEVEL)
        .map_err(|e| format!("cannot create zstd encoder: {e}"))?;
    encoder
        .include_checksum(true)
        .map_err(|e| format!("cannot enable zstd checksum: {e}"))?;
    let mut builder = tar::Builder::new(encoder);
    builder.follow_symlinks(false);
    let name = source
        .file_name()
        .ok_or_else(|| "cold source has no filename".to_owned())?;
    if fs::symlink_metadata(source)
        .map_err(|e| e.to_string())?
        .is_dir()
    {
        builder
            .append_dir_all(Path::new(name), source)
            .map_err(|e| format!("cannot append directory to cold archive: {e}"))?;
    } else {
        builder
            .append_path_with_name(source, Path::new(name))
            .map_err(|e| format!("cannot append file to cold archive: {e}"))?;
    }
    let encoder = builder
        .into_inner()
        .map_err(|e| format!("cannot finish tar stream: {e}"))?;
    let file = encoder
        .finish()
        .map_err(|e| format!("cannot finish zstd stream: {e}"))?;
    file.sync_all()
        .map_err(|e| format!("cannot fsync cold temp archive: {e}"))
}

fn restore_legacy_cold_archive(
    archive_path: &Path,
    destination_root: &Path,
) -> Result<Vec<PathBuf>, String> {
    verify_checksum_sidecar(archive_path)?;
    fs::create_dir_all(destination_root).map_err(|e| {
        format!(
            "cannot create restore root {}: {e}",
            destination_root.display()
        )
    })?;
    let restore_root = fs::canonicalize(destination_root).map_err(|e| {
        format!(
            "cannot canonicalize restore root {}: {e}",
            destination_root.display()
        )
    })?;
    let temp = restore_root.join(format!(
        ".forgeclean-restore-{}-{}",
        std::process::id(),
        unique_stamp()?
    ));
    fs::create_dir(&temp)
        .map_err(|e| format!("cannot create restore staging {}: {e}", temp.display()))?;
    if let Err(error) = unpack_legacy_archive_safely(archive_path, &temp) {
        let _ = fs::remove_dir_all(&temp);
        return Err(error);
    }
    commit_staging(&temp, &restore_root)
}

fn unpack_legacy_archive_safely(archive_path: &Path, temp: &Path) -> Result<(), String> {
    let file = File::open(archive_path)
        .map_err(|e| format!("cannot open cold archive {}: {e}", archive_path.display()))?;
    let decoder = zstd::Decoder::new(file).map_err(|e| format!("cannot open zstd stream: {e}"))?;
    let mut archive = tar::Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|e| format!("cannot read tar entries: {e}"))?;
    for item in entries {
        let mut entry = item.map_err(|e| format!("cannot read tar entry: {e}"))?;
        let path = entry
            .path()
            .map_err(|e| format!("invalid tar path: {e}"))?
            .into_owned();
        validate_archive_path(&path)?;
        if entry.header().entry_type().is_symlink() || entry.header().entry_type().is_hard_link() {
            return Err(format!(
                "AETHER_GUARD=BLOCKED cold archive link entry: {}",
                path.display()
            ));
        }
        let unpacked = entry
            .unpack_in(temp)
            .map_err(|e| format!("cannot unpack {}: {e}", path.display()))?;
        if !unpacked {
            return Err(format!(
                "AETHER_GUARD=BLOCKED archive entry escaped restore root: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn commit_staging(temp: &Path, restore_root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut staged = Vec::new();
    for entry in fs::read_dir(temp).map_err(|e| e.to_string())? {
        staged.push(entry.map_err(|e| e.to_string())?.path());
    }
    staged.sort();
    if staged.is_empty() {
        let _ = fs::remove_dir_all(temp);
        return Err("cold archive contains no restorable entries".to_owned());
    }
    for item in &staged {
        let name = item
            .file_name()
            .ok_or_else(|| "restore entry has no filename".to_owned())?;
        let final_path = restore_root.join(name);
        if final_path.exists() || fs::symlink_metadata(&final_path).is_ok() {
            let _ = fs::remove_dir_all(temp);
            return Err(format!(
                "restore destination already exists: {}",
                final_path.display()
            ));
        }
    }
    let mut restored = Vec::new();
    for item in staged {
        let name = item
            .file_name()
            .ok_or_else(|| "restore entry has no filename".to_owned())?
            .to_owned();
        let final_path = restore_root.join(name);
        fs::rename(&item, &final_path)
            .map_err(|e| format!("cannot commit restored entry {}: {e}", final_path.display()))?;
        restored.push(final_path);
    }
    fs::remove_dir(temp)
        .map_err(|e| format!("cannot remove restore staging {}: {e}", temp.display()))?;
    sync_directory(restore_root)?;
    Ok(restored)
}

fn validate_archive_path(path: &Path) -> Result<(), String> {
    if path.is_absolute() || path.as_os_str().is_empty() {
        return Err(format!(
            "AETHER_GUARD=BLOCKED absolute/empty archive path: {}",
            path.display()
        ));
    }
    for component in path.components() {
        if matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            return Err(format!(
                "AETHER_GUARD=BLOCKED unsafe archive path: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn reject_symlinks_recursive(path: &Path) -> Result<(), String> {
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() {
        return Err(format!(
            "AETHER_GUARD=BLOCKED symlink inside cold source: {}",
            path.display()
        ));
    }
    if meta.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            reject_symlinks_recursive(&entry.map_err(|e| e.to_string())?.path())?;
        }
    }
    Ok(())
}

fn logical_size(path: &Path) -> Result<u64, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if meta.is_file() {
        return Ok(meta.len());
    }
    if meta.is_dir() {
        let mut total = 0u64;
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            total = total
                .checked_add(logical_size(&entry.map_err(|e| e.to_string())?.path())?)
                .ok_or_else(|| "logical size overflow".to_owned())?;
        }
        return Ok(total);
    }
    Ok(0)
}

fn hash_tree(path: &Path) -> Result<String, String> {
    let root_name = path
        .file_name()
        .ok_or_else(|| "source has no filename".to_owned())?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; HASH_BUFFER_BYTES];
    hash_tree_inner(path, Path::new(root_name), &mut hasher, &mut buffer)?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn hash_tree_inner(
    path: &Path,
    relative: &Path,
    hasher: &mut Sha256,
    buffer: &mut [u8],
) -> Result<(), String> {
    let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() {
        return Err(format!(
            "AETHER_GUARD=BLOCKED symlink inside cold source: {}",
            path.display()
        ));
    }
    hasher.update(relative.as_os_str().as_bytes());
    if meta.is_dir() {
        hasher.update(b"D");
        let mut children = Vec::new();
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            children.push(entry.map_err(|e| e.to_string())?.path());
        }
        children.sort();
        for child in children {
            let child_name = child
                .file_name()
                .ok_or_else(|| "child has no filename".to_owned())?;
            hash_tree_inner(&child, &relative.join(child_name), hasher, buffer)?;
        }
    } else if meta.is_file() {
        hasher.update(b"F");
        let mut file = File::open(path).map_err(|e| e.to_string())?;
        loop {
            let n = file.read(buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
    } else {
        return Err(format!(
            "unsupported cold source entry type: {}",
            path.display()
        ));
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|e| format!("cannot open {} for hashing: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; HASH_BUFFER_BYTES];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn checksum_path_for(archive: &Path) -> PathBuf {
    let mut value = archive.as_os_str().to_os_string();
    value.push(".sha256");
    PathBuf::from(value)
}

fn verify_checksum_sidecar(archive_path: &Path) -> Result<(), String> {
    let checksum_path = checksum_path_for(archive_path);
    let expected = read_checksum(&checksum_path)?;
    let actual = hash_file(archive_path)?;
    if expected != actual {
        return Err(format!(
            "AETHER_GUARD=BLOCKED cold archive checksum mismatch: {}",
            archive_path.display()
        ));
    }
    Ok(())
}

fn write_checksum(path: &Path, hash: &str, archive: &Path) -> Result<(), String> {
    let name = archive
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| "archive filename is not UTF-8".to_owned())?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("cannot create cold checksum {}: {e}", path.display()))?;
    writeln!(file, "{hash}  {name}").map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}

fn read_checksum(path: &Path) -> Result<String, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("cannot read cold checksum {}: {e}", path.display()))?;
    let hash = text
        .split_whitespace()
        .next()
        .ok_or_else(|| "cold checksum file is empty".to_owned())?;
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("cold checksum is not SHA-256".to_owned());
    }
    Ok(hash.to_ascii_lowercase())
}

fn path_to_hex(path: &Path) -> String {
    hex_encode(path.as_os_str().as_bytes())
}

fn os_str_to_hex(value: &OsStr) -> String {
    hex_encode(value.as_bytes())
}

fn hex_to_path(value: &str) -> Result<PathBuf, String> {
    let bytes = hex_decode(value)?;
    Ok(PathBuf::from(OsString::from_vec(bytes)))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn hex_decode(value: &str) -> Result<Vec<u8>, String> {
    if (value.len() & 1) != 0 {
        return Err("ColdPack hex path has odd length".to_owned());
    }
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.as_chunks::<2>().0 {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        out.push((high << 4) | low);
    }
    Ok(out)
}

fn hex_nibble(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err("ColdPack path contains invalid hex".to_owned()),
    }
}

fn sanitize_label(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('.')
        .to_owned()
}

fn unique_stamp() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .map_err(|e| e.to_string())
}

fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|e| format!("cannot fsync directory {}: {e}", path.display()))
}

#[cfg(test)]
mod store_lock_tests {
    use super::*;
    use std::fs::TryLockError;

    #[test]
    fn coldpack_store_exclusive_lock_blocks_second_writer() {
        let root = std::env::temp_dir().join(format!(
            "forgeclean-lock-{}-{}",
            std::process::id(),
            unique_stamp().unwrap()
        ));
        let first = lock_store_exclusive(&root).unwrap();
        let second = open_store_lock(&root).unwrap();
        assert!(matches!(second.try_lock(), Err(TryLockError::WouldBlock)));
        drop(first);
        second.try_lock().unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
