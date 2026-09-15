pub mod archive;
pub mod blte;
pub mod build;
pub mod index;
pub mod key;
pub mod storage;

pub use archive::{ARCHIVE_ENVELOPE_BYTES, ArchiveReader};
pub use blte::{BlteDecodeOptions, decode_blte, decode_blte_with_options};
pub use build::{
    BuildConfig, BuildInfo, build_config_path, extract_build_key, load_build_config,
    parse_build_info,
};
pub use index::{ArchiveLocation, LocalIndex};
pub use key::{EncodingKey, EncodingKeyPrefix};
pub use storage::CascStorage;

use sanctuary_core::{SanctuaryError, xdg_cache_dir};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

pub const INVENTORY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageKind {
    Index,
    Archive,
    Config,
    Other,
}

impl StorageKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Index => "Index",
            Self::Archive => "Archive",
            Self::Config => "Config",
            Self::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageEntry {
    pub relative_path: String,
    pub byte_len: u64,
    pub kind: StorageKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentInventory {
    pub schema_version: u32,
    pub fingerprint: String,
    pub build: BuildInfo,
    pub archive_files: usize,
    pub index_files: usize,
    pub config_files: usize,
    pub other_files: usize,
    pub total_bytes: u64,
    pub entries: Vec<StorageEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentStoreSummary {
    pub build_key: Option<String>,
    pub data_files: usize,
    pub index_files: usize,
    pub total_bytes: u64,
}

#[derive(Debug, Clone)]
struct FingerprintRecord {
    relative_path: String,
    byte_len: u64,
    modified_nanos: u128,
}

pub fn default_inventory_cache_path() -> PathBuf {
    xdg_cache_dir().join("content/inventory-v1.json")
}

pub fn classify_storage_path(relative_path: &str) -> StorageKind {
    let normalized = relative_path.replace('\\', "/");
    let path = Path::new(&normalized);
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("idx"))
    {
        return StorageKind::Index;
    }

    let basename = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if basename
        .strip_prefix("data.")
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()))
    {
        return StorageKind::Archive;
    }

    let lowered = format!("/{}/", normalized.to_ascii_lowercase());
    if lowered.contains("/config/") || basename.eq_ignore_ascii_case("config") {
        return StorageKind::Config;
    }
    StorageKind::Other
}

pub fn build_inventory(install_path: &Path) -> Result<ContentInventory, SanctuaryError> {
    let build_info_path = install_path.join(".build.info");
    let build_text = read_text(&build_info_path)?;
    let build = parse_build_info(&build_text)?;
    let (mut entries, records) = scan_data_tree(install_path)?;
    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));

    let mut archive_files = 0usize;
    let mut index_files = 0usize;
    let mut config_files = 0usize;
    let mut other_files = 0usize;
    let mut total_bytes = 0u64;
    for entry in &entries {
        total_bytes = total_bytes.saturating_add(entry.byte_len);
        match entry.kind {
            StorageKind::Archive => archive_files += 1,
            StorageKind::Index => index_files += 1,
            StorageKind::Config => config_files += 1,
            StorageKind::Other => other_files += 1,
        }
    }

    let fingerprint = fingerprint_from_records(&build, &records);
    Ok(ContentInventory {
        schema_version: INVENTORY_SCHEMA_VERSION,
        fingerprint,
        build,
        archive_files,
        index_files,
        config_files,
        other_files,
        total_bytes,
        entries,
    })
}

pub fn probe_content_store(install_path: &Path) -> Result<ContentStoreSummary, SanctuaryError> {
    let inventory = build_inventory(install_path)?;
    if inventory.entries.is_empty() {
        return Err(SanctuaryError::ContentStoreUnreadable(format!(
            "{} contains no data files",
            install_path.join("Data").display()
        )));
    }
    Ok(ContentStoreSummary {
        build_key: inventory.build.build_key,
        data_files: inventory
            .entries
            .len()
            .saturating_sub(inventory.index_files),
        index_files: inventory.index_files,
        total_bytes: inventory.total_bytes,
    })
}

pub fn read_cached_inventory(
    cache_path: &Path,
) -> Result<Option<ContentInventory>, SanctuaryError> {
    let bytes = match fs::read(cache_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(SanctuaryError::Io {
                path: cache_path.to_path_buf(),
                source,
            });
        }
    };
    let inventory = match serde_json::from_slice::<ContentInventory>(&bytes) {
        Ok(inventory) => inventory,
        Err(_) => return Ok(None),
    };
    if inventory.schema_version != INVENTORY_SCHEMA_VERSION {
        return Ok(None);
    }
    Ok(Some(inventory))
}

pub fn write_cached_inventory(
    cache_path: &Path,
    inventory: &ContentInventory,
) -> Result<(), SanctuaryError> {
    let parent = cache_path.parent().ok_or_else(|| {
        SanctuaryError::Cache(format!("{} has no parent directory", cache_path.display()))
    })?;
    fs::create_dir_all(parent).map_err(|source| SanctuaryError::Io {
        path: parent.to_path_buf(),
        source,
    })?;

    let file_name = cache_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("inventory-v1.json");
    let temporary = parent.join(format!(".{file_name}.tmp-{}", std::process::id()));
    let file = File::create(&temporary).map_err(|source| SanctuaryError::Io {
        path: temporary.clone(),
        source,
    })?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, inventory)
        .map_err(|error| SanctuaryError::Cache(error.to_string()))?;
    writer
        .write_all(b"\n")
        .map_err(|source| SanctuaryError::Io {
            path: temporary.clone(),
            source,
        })?;
    writer.flush().map_err(|source| SanctuaryError::Io {
        path: temporary.clone(),
        source,
    })?;
    drop(writer);
    fs::rename(&temporary, cache_path).map_err(|source| SanctuaryError::Io {
        path: cache_path.to_path_buf(),
        source,
    })?;
    Ok(())
}

pub fn inventory_is_current(
    inventory: &ContentInventory,
    install_path: &Path,
) -> Result<bool, SanctuaryError> {
    if inventory.schema_version != INVENTORY_SCHEMA_VERSION {
        return Ok(false);
    }
    let build_text = read_text(&install_path.join(".build.info"))?;
    let build = parse_build_info(&build_text)?;
    let (_, records) = scan_data_tree(install_path)?;
    Ok(inventory.fingerprint == fingerprint_from_records(&build, &records))
}

fn read_text(path: &Path) -> Result<String, SanctuaryError> {
    fs::read_to_string(path).map_err(|error| {
        SanctuaryError::ContentStoreUnreadable(format!("cannot read {}: {error}", path.display()))
    })
}

fn scan_data_tree(
    install_path: &Path,
) -> Result<(Vec<StorageEntry>, Vec<FingerprintRecord>), SanctuaryError> {
    let data_root = install_path.join("Data");
    if !data_root.is_dir() {
        return Err(SanctuaryError::ContentStoreUnreadable(format!(
            "{} is not a readable content directory",
            data_root.display()
        )));
    }

    let mut entries = Vec::new();
    let mut records = Vec::new();
    push_fingerprint_record(
        install_path,
        &install_path.join(".build.info"),
        &mut records,
    )?;
    walk_directory(install_path, &data_root, &mut entries, &mut records)?;
    records.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok((entries, records))
}

fn walk_directory(
    install_path: &Path,
    directory: &Path,
    entries: &mut Vec<StorageEntry>,
    records: &mut Vec<FingerprintRecord>,
) -> Result<(), SanctuaryError> {
    let dir_entries = fs::read_dir(directory).map_err(|error| {
        SanctuaryError::ContentStoreUnreadable(format!(
            "cannot enumerate {}: {error}",
            directory.display()
        ))
    })?;
    for dir_entry in dir_entries {
        let dir_entry = dir_entry.map_err(|error| {
            SanctuaryError::ContentStoreUnreadable(format!(
                "cannot enumerate {}: {error}",
                directory.display()
            ))
        })?;
        let path = dir_entry.path();
        let file_type = dir_entry.file_type().map_err(|error| {
            SanctuaryError::ContentStoreUnreadable(format!(
                "cannot inspect {}: {error}",
                path.display()
            ))
        })?;
        if file_type.is_dir() {
            walk_directory(install_path, &path, entries, records)?;
        } else if file_type.is_file() {
            let metadata = dir_entry.metadata().map_err(|error| {
                SanctuaryError::ContentStoreUnreadable(format!(
                    "cannot inspect {}: {error}",
                    path.display()
                ))
            })?;
            let relative_path = relative_string(install_path, &path)?;
            entries.push(StorageEntry {
                kind: classify_storage_path(&relative_path),
                relative_path: relative_path.clone(),
                byte_len: metadata.len(),
            });
            records.push(FingerprintRecord {
                relative_path,
                byte_len: metadata.len(),
                modified_nanos: modified_nanos(&metadata, &path)?,
            });
        }
    }
    Ok(())
}

fn push_fingerprint_record(
    install_path: &Path,
    path: &Path,
    records: &mut Vec<FingerprintRecord>,
) -> Result<(), SanctuaryError> {
    let metadata = fs::metadata(path).map_err(|error| {
        SanctuaryError::ContentStoreUnreadable(format!(
            "cannot inspect {}: {error}",
            path.display()
        ))
    })?;
    records.push(FingerprintRecord {
        relative_path: relative_string(install_path, path)?,
        byte_len: metadata.len(),
        modified_nanos: modified_nanos(&metadata, path)?,
    });
    Ok(())
}

fn modified_nanos(metadata: &fs::Metadata, path: &Path) -> Result<u128, SanctuaryError> {
    let modified = metadata.modified().map_err(|error| {
        SanctuaryError::ContentStoreUnreadable(format!(
            "cannot read modification time for {}: {error}",
            path.display()
        ))
    })?;
    Ok(modified
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos())
}

fn relative_string(install_path: &Path, path: &Path) -> Result<String, SanctuaryError> {
    let relative = path.strip_prefix(install_path).map_err(|error| {
        SanctuaryError::ContentStoreUnreadable(format!(
            "cannot make {} relative to {}: {error}",
            path.display(),
            install_path.display()
        ))
    })?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn fingerprint_from_records(build: &BuildInfo, records: &[FingerprintRecord]) -> String {
    let mut hash = Fnv1a64::new();
    hash.update(build.build_key.as_deref().unwrap_or_default().as_bytes());
    for record in records {
        hash.update(b"\0");
        hash.update(record.relative_path.as_bytes());
        hash.update(b"\0");
        hash.update(record.byte_len.to_string().as_bytes());
        hash.update(b"\0");
        hash.update(record.modified_nanos.to_string().as_bytes());
    }
    format!("fnv1a64:{:016x}", hash.finish())
}

struct Fnv1a64(u64);

impl Fnv1a64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    fn new() -> Self {
        Self(Self::OFFSET)
    }

    fn update(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    fn finish(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_content_store() -> tempfile::TempDir {
        let td = tempfile::tempdir().unwrap();
        fs::write(
            td.path().join(".build.info"),
            "Branch!STRING:0|Build Key!HEX:16|CDN Key!HEX:16|Version!STRING:0|Product!STRING:0|Mystery!STRING:0\nD3|0011223344556677|8899aabbccddeeff|2.7.8.12345|d3|kept\n",
        )
        .unwrap();
        let data = td.path().join("Data/data");
        fs::create_dir_all(td.path().join("Data/config/cache")).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("0000000002.idx"), b"idx-two").unwrap();
        fs::write(data.join("data.001"), b"archive-one").unwrap();
        fs::write(data.join("0000000001.idx"), b"idx-one").unwrap();
        fs::write(td.path().join("Data/config/cache/config"), b"config").unwrap();
        fs::write(td.path().join("Data/readme.bin"), b"other").unwrap();
        td
    }

    #[test]
    fn parses_typed_headers_and_preserves_unknown_columns() {
        let td = fixture_content_store();
        let text = fs::read_to_string(td.path().join(".build.info")).unwrap();
        let info = parse_build_info(&text).unwrap();
        assert_eq!(info.branch.as_deref(), Some("D3"));
        assert_eq!(info.build_key.as_deref(), Some("0011223344556677"));
        assert_eq!(info.cdn_key.as_deref(), Some("8899aabbccddeeff"));
        assert_eq!(info.version.as_deref(), Some("2.7.8.12345"));
        assert_eq!(info.product.as_deref(), Some("d3"));
        assert_eq!(
            info.unknown.get("Mystery").map(String::as_str),
            Some("kept")
        );
    }

    #[test]
    fn accepts_buildkey_without_space() {
        let info = parse_build_info("BuildKey!HEX:16|Branch!STRING:0\nabc123|D3\n").unwrap();
        assert_eq!(info.build_key.as_deref(), Some("abc123"));
    }

    #[test]
    fn rejects_missing_data_row() {
        let error = parse_build_info("Build Key!HEX:16|Branch!STRING:0\n").unwrap_err();
        assert!(error.to_string().contains("no data row"));
    }

    #[test]
    fn classifies_storage_paths() {
        assert_eq!(
            classify_storage_path("Data/data/123.idx"),
            StorageKind::Index
        );
        assert_eq!(
            classify_storage_path("Data/data/data.001"),
            StorageKind::Archive
        );
        assert_eq!(
            classify_storage_path("Data/config/cache/blob"),
            StorageKind::Config
        );
        assert_eq!(classify_storage_path("Data/config"), StorageKind::Config);
        assert_eq!(classify_storage_path("Data/other.bin"), StorageKind::Other);
    }

    #[test]
    fn inventory_is_sorted_and_totals_are_stable() {
        let td = fixture_content_store();
        let inventory = build_inventory(td.path()).unwrap();
        assert_eq!(inventory.index_files, 2);
        assert_eq!(inventory.archive_files, 1);
        assert_eq!(inventory.config_files, 1);
        assert_eq!(inventory.other_files, 1);
        assert!(inventory.total_bytes > 0);
        assert!(
            inventory
                .entries
                .windows(2)
                .all(|pair| pair[0].relative_path <= pair[1].relative_path)
        );
    }

    #[test]
    fn cache_round_trips_and_wrong_schema_is_absent() {
        let td = fixture_content_store();
        let inventory = build_inventory(td.path()).unwrap();
        let cache_dir = tempfile::tempdir().unwrap();
        let cache = cache_dir.path().join("inventory-v1.json");
        write_cached_inventory(&cache, &inventory).unwrap();
        assert_eq!(
            read_cached_inventory(&cache).unwrap(),
            Some(inventory.clone())
        );

        let mut wrong = inventory;
        wrong.schema_version = 99;
        fs::write(&cache, serde_json::to_vec(&wrong).unwrap()).unwrap();
        assert!(read_cached_inventory(&cache).unwrap().is_none());
    }

    #[test]
    fn corrupt_cache_is_non_fatal() {
        let td = tempfile::tempdir().unwrap();
        let cache = td.path().join("inventory-v1.json");
        fs::write(&cache, b"not-json").unwrap();
        assert!(read_cached_inventory(&cache).unwrap().is_none());
    }

    #[test]
    fn changed_data_size_invalidates_fingerprint() {
        let td = fixture_content_store();
        let inventory = build_inventory(td.path()).unwrap();
        assert!(inventory_is_current(&inventory, td.path()).unwrap());
        fs::write(
            td.path().join("Data/data/data.001"),
            b"archive-one-is-now-longer",
        )
        .unwrap();
        assert!(!inventory_is_current(&inventory, td.path()).unwrap());
    }

    #[test]
    fn compatibility_summary_keeps_index_and_data_counts() {
        let td = fixture_content_store();
        let summary = probe_content_store(td.path()).unwrap();
        assert_eq!(summary.index_files, 2);
        assert_eq!(summary.data_files, 3);
        assert_eq!(summary.build_key.as_deref(), Some("0011223344556677"));
    }
}
