use crate::key::{EncodingKey, EncodingKeyPrefix, LOCAL_INDEX_KEY_BYTES};
use sanctuary_core::SanctuaryError;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

const INDEX_ENTRY_BYTES: usize = 18;
const INDEX_BUCKET_COUNT: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveLocation {
    pub archive_index: u16,
    pub offset: u64,
    pub encoded_size: u32,
}

#[derive(Debug, Clone)]
pub struct LocalIndex {
    entries: BTreeMap<EncodingKeyPrefix, ArchiveLocation>,
    selected_files: Vec<PathBuf>,
}

impl LocalIndex {
    pub fn open(install_path: &Path) -> Result<Self, SanctuaryError> {
        Self::open_data_root(&install_path.join("Data/data"))
    }

    pub fn open_data_root(data_root: &Path) -> Result<Self, SanctuaryError> {
        let selected_files = select_bucket_files(data_root)?;
        if selected_files.is_empty() {
            return Err(SanctuaryError::ContentStoreUnreadable(format!(
                "{} contains no local CASC index files",
                data_root.display()
            )));
        }

        let mut entries = BTreeMap::new();
        for path in &selected_files {
            let bytes = fs::read(path).map_err(|source| SanctuaryError::Io {
                path: path.clone(),
                source,
            })?;
            for (key, location) in parse_guarded_index(path, &bytes)? {
                entries.entry(key).or_insert(location);
            }
        }

        if entries.is_empty() {
            return Err(SanctuaryError::ContentStoreUnreadable(
                "local CASC index contains no usable entries".into(),
            ));
        }

        Ok(Self {
            entries,
            selected_files,
        })
    }

    pub fn lookup(&self, key: &EncodingKey) -> Option<ArchiveLocation> {
        self.lookup_prefix(key.prefix())
    }

    pub fn lookup_prefix(&self, prefix: EncodingKeyPrefix) -> Option<ArchiveLocation> {
        self.entries.get(&prefix).copied()
    }

    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    pub fn selected_files(&self) -> &[PathBuf] {
        &self.selected_files
    }
}

fn select_bucket_files(data_root: &Path) -> Result<Vec<PathBuf>, SanctuaryError> {
    let mut selected: [Option<(String, PathBuf)>; INDEX_BUCKET_COUNT] = Default::default();
    let directory = fs::read_dir(data_root).map_err(|source| SanctuaryError::Io {
        path: data_root.to_path_buf(),
        source,
    })?;

    for item in directory {
        let item = item.map_err(|source| SanctuaryError::Io {
            path: data_root.to_path_buf(),
            source,
        })?;
        let file_type = item.file_type().map_err(|source| SanctuaryError::Io {
            path: item.path(),
            source,
        })?;
        if !file_type.is_file() {
            continue;
        }
        let Some(name) = item.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };
        let Some(bucket) = bucket_from_name(&name) else {
            continue;
        };
        let slot = &mut selected[bucket];
        if slot.as_ref().is_none_or(|(current, _)| name > *current) {
            *slot = Some((name, item.path()));
        }
    }

    Ok(selected
        .into_iter()
        .flatten()
        .map(|(_, path)| path)
        .collect())
}

fn bucket_from_name(name: &str) -> Option<usize> {
    if !name.to_ascii_lowercase().ends_with(".idx") || name.len() < 6 {
        return None;
    }
    let prefix = name.get(..2)?;
    let bucket = usize::from(u8::from_str_radix(prefix, 16).ok()?);
    (bucket < INDEX_BUCKET_COUNT).then_some(bucket)
}

fn parse_guarded_index(
    path: &Path,
    bytes: &[u8],
) -> Result<Vec<(EncodingKeyPrefix, ArchiveLocation)>, SanctuaryError> {
    let header_size = read_i32_le(bytes, 0, path, "header hash size")?;
    if header_size < 0 {
        return Err(index_error(path, "negative header size"));
    }
    let header_size = usize::try_from(header_size).map_err(|_| index_error(path, "header size"))?;
    let header_end = 8usize
        .checked_add(header_size)
        .ok_or_else(|| index_error(path, "header size overflow"))?;
    if header_end > bytes.len() {
        return Err(index_error(path, "truncated guarded header"));
    }
    let entries_header =
        align_up_16(header_end).ok_or_else(|| index_error(path, "header alignment overflow"))?;
    let entries_size = read_i32_le(bytes, entries_header, path, "entries size")?;
    if entries_size < 0 {
        return Err(index_error(path, "negative entries size"));
    }
    let entries_size =
        usize::try_from(entries_size).map_err(|_| index_error(path, "entries size"))?;
    if entries_size % INDEX_ENTRY_BYTES != 0 {
        return Err(index_error(
            path,
            "entry block is not aligned to 18-byte records",
        ));
    }
    let entries_start = entries_header
        .checked_add(8)
        .ok_or_else(|| index_error(path, "entries offset overflow"))?;
    let entries_end = entries_start
        .checked_add(entries_size)
        .ok_or_else(|| index_error(path, "entries length overflow"))?;
    if entries_end > bytes.len() {
        return Err(index_error(path, "truncated entry block"));
    }

    let mut output = Vec::with_capacity(entries_size / INDEX_ENTRY_BYTES);
    for raw in bytes[entries_start..entries_end]
        .as_chunks::<INDEX_ENTRY_BYTES>()
        .0
    {
        if raw.iter().all(|byte| *byte == 0) {
            continue;
        }
        let mut prefix = [0u8; LOCAL_INDEX_KEY_BYTES];
        prefix.copy_from_slice(&raw[..LOCAL_INDEX_KEY_BYTES]);
        let index_high = u16::from(raw[9]);
        let index_low = u32::from_be_bytes([raw[10], raw[11], raw[12], raw[13]]);
        let encoded_size = i32::from_le_bytes([raw[14], raw[15], raw[16], raw[17]]);
        if encoded_size <= 0 {
            return Err(index_error(path, "entry has non-positive encoded size"));
        }
        let archive_index =
            (index_high << 2) | u16::try_from((index_low >> 30) & 0x03).unwrap_or(0);
        let offset = u64::from(index_low & 0x3fff_ffff);
        output.push((
            EncodingKeyPrefix::from_bytes(prefix),
            ArchiveLocation {
                archive_index,
                offset,
                encoded_size: u32::try_from(encoded_size)
                    .map_err(|_| index_error(path, "encoded size conversion"))?,
            },
        ));
    }
    Ok(output)
}

fn read_i32_le(
    bytes: &[u8],
    offset: usize,
    path: &Path,
    label: &str,
) -> Result<i32, SanctuaryError> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| index_error(path, "integer offset overflow"))?;
    let raw = bytes
        .get(offset..end)
        .ok_or_else(|| index_error(path, &format!("missing {label}")))?;
    Ok(i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

fn align_up_16(value: usize) -> Option<usize> {
    value.checked_add(15).map(|aligned| aligned & !15)
}

fn index_error(path: &Path, detail: &str) -> SanctuaryError {
    SanctuaryError::ContentStoreUnreadable(format!(
        "invalid local index {}: {detail}",
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(prefix: [u8; 9], archive: u16, offset: u32, size: i32) -> [u8; 18] {
        assert!(archive < 1024);
        assert!(offset < 0x4000_0000);
        let mut raw = [0u8; 18];
        raw[..9].copy_from_slice(&prefix);
        raw[9] = (archive >> 2) as u8;
        let low = (offset & 0x3fff_ffff) | (u32::from(archive & 0x03) << 30);
        raw[10..14].copy_from_slice(&low.to_be_bytes());
        raw[14..18].copy_from_slice(&size.to_le_bytes());
        raw
    }

    fn guarded(entries: &[[u8; 18]]) -> Vec<u8> {
        let header = [7u8; 16];
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(header.len() as i32).to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&header);
        while bytes.len() % 16 != 0 {
            bytes.push(0);
        }
        bytes.extend_from_slice(&((entries.len() * 18) as i32).to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        for entry in entries {
            bytes.extend_from_slice(entry);
        }
        bytes
    }

    #[test]
    fn parses_archive_index_offset_and_size() {
        let bytes = guarded(&[entry([0x11; 9], 7, 0x12345, 64)]);
        let parsed = parse_guarded_index(Path::new("0a.idx"), &bytes).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].1.archive_index, 7);
        assert_eq!(parsed[0].1.offset, 0x12345);
        assert_eq!(parsed[0].1.encoded_size, 64);
    }

    #[test]
    fn newest_file_per_bucket_is_selected_and_duplicate_first_wins() {
        let td = tempfile::tempdir().unwrap();
        fs::write(
            td.path().join("0a00000001.idx"),
            guarded(&[entry([1; 9], 1, 10, 40)]),
        )
        .unwrap();
        fs::write(
            td.path().join("0a00000002.idx"),
            guarded(&[
                entry([1; 9], 2, 20, 50),
                entry([1; 9], 3, 30, 60),
                entry([2; 9], 4, 40, 70),
            ]),
        )
        .unwrap();
        let index = LocalIndex::open_data_root(td.path()).unwrap();
        assert_eq!(index.selected_files().len(), 1);
        assert!(index.selected_files()[0].ends_with("0a00000002.idx"));
        assert_eq!(
            index.lookup_prefix(EncodingKeyPrefix::from_bytes([1; 9])),
            Some(ArchiveLocation {
                archive_index: 2,
                offset: 20,
                encoded_size: 50,
            })
        );
        assert_eq!(index.entry_count(), 2);
    }

    #[test]
    fn rejects_truncated_or_misaligned_index() {
        assert!(parse_guarded_index(Path::new("bad.idx"), &[1, 2, 3]).is_err());
        let mut bytes = guarded(&[entry([1; 9], 1, 1, 40)]);
        let header_size = 16usize;
        let entries_header = align_up_16(8 + header_size).unwrap();
        bytes[entries_header..entries_header + 4].copy_from_slice(&17i32.to_le_bytes());
        assert!(parse_guarded_index(Path::new("bad.idx"), &bytes).is_err());
    }
}
