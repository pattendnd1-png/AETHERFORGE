use crate::{index::ArchiveLocation, key::EncodingKeyPrefix};
use sanctuary_core::SanctuaryError;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};

pub const ARCHIVE_ENVELOPE_BYTES: usize = 30;
pub const MAX_ENCODED_BLOB_BYTES: usize = 512 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ArchiveReader {
    data_root: PathBuf,
}

impl ArchiveReader {
    pub fn new(install_path: &Path) -> Self {
        Self {
            data_root: install_path.join("Data/data"),
        }
    }

    pub fn from_data_root(data_root: PathBuf) -> Self {
        Self { data_root }
    }

    pub fn read_blte(
        &self,
        expected_prefix: EncodingKeyPrefix,
        location: ArchiveLocation,
    ) -> Result<Vec<u8>, SanctuaryError> {
        let encoded_size = usize::try_from(location.encoded_size).map_err(|_| {
            SanctuaryError::ContentStoreUnreadable(
                "archive encoded size does not fit memory".into(),
            )
        })?;
        if encoded_size < ARCHIVE_ENVELOPE_BYTES {
            return Err(SanctuaryError::ContentStoreUnreadable(format!(
                "archive record is only {encoded_size} bytes; CASC envelope requires {ARCHIVE_ENVELOPE_BYTES}"
            )));
        }
        if encoded_size > MAX_ENCODED_BLOB_BYTES {
            return Err(SanctuaryError::ContentStoreUnreadable(format!(
                "archive record exceeds the {} byte read limit",
                MAX_ENCODED_BLOB_BYTES
            )));
        }

        let path = self
            .data_root
            .join(format!("data.{:03}", location.archive_index));
        let mut file = File::open(&path).map_err(|source| SanctuaryError::Io {
            path: path.clone(),
            source,
        })?;
        let file_len = file
            .metadata()
            .map_err(|source| SanctuaryError::Io {
                path: path.clone(),
                source,
            })?
            .len();
        let end = location
            .offset
            .checked_add(u64::from(location.encoded_size))
            .ok_or_else(|| {
                SanctuaryError::ContentStoreUnreadable(format!(
                    "archive read overflows for {}",
                    path.display()
                ))
            })?;
        if end > file_len {
            return Err(SanctuaryError::ContentStoreUnreadable(format!(
                "archive read {}..{} exceeds {} byte file {}",
                location.offset,
                end,
                file_len,
                path.display()
            )));
        }

        file.seek(SeekFrom::Start(location.offset))
            .map_err(|source| SanctuaryError::Io {
                path: path.clone(),
                source,
            })?;
        let mut record = vec![0u8; encoded_size];
        file.read_exact(&mut record)
            .map_err(|source| SanctuaryError::Io {
                path: path.clone(),
                source,
            })?;

        validate_envelope(&path, &record, expected_prefix, location.encoded_size)?;
        Ok(record[ARCHIVE_ENVELOPE_BYTES..].to_vec())
    }
}

fn validate_envelope(
    path: &Path,
    record: &[u8],
    expected_prefix: EncodingKeyPrefix,
    expected_size: u32,
) -> Result<(), SanctuaryError> {
    let stored_key = record
        .get(..16)
        .ok_or_else(|| envelope_error(path, "truncated encoding key"))?;
    if !envelope_key_matches(stored_key, expected_prefix) {
        return Err(envelope_error(
            path,
            "encoding-key prefix does not match local index",
        ));
    }

    let raw_size = record
        .get(16..20)
        .ok_or_else(|| envelope_error(path, "truncated encoded-size field"))?;
    let stored_size = i32::from_le_bytes([raw_size[0], raw_size[1], raw_size[2], raw_size[3]]);
    if stored_size <= 0 || u32::try_from(stored_size).ok() != Some(expected_size) {
        return Err(envelope_error(
            path,
            "encoded-size field does not match local index",
        ));
    }
    Ok(())
}

fn envelope_key_matches(stored_key: &[u8], expected_prefix: EncodingKeyPrefix) -> bool {
    let expected = expected_prefix.as_bytes();
    if stored_key.get(..expected.len()) == Some(&expected[..]) {
        return true;
    }
    let mut reversed = [0u8; 16];
    if stored_key.len() != reversed.len() {
        return false;
    }
    reversed.copy_from_slice(stored_key);
    reversed.reverse();
    reversed.get(..expected.len()) == Some(&expected[..])
}

fn envelope_error(path: &Path, detail: &str) -> SanctuaryError {
    SanctuaryError::ContentStoreUnreadable(format!(
        "invalid archive envelope in {}: {detail}",
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn record(prefix: [u8; 9], payload: &[u8], reverse_key: bool) -> Vec<u8> {
        let size = ARCHIVE_ENVELOPE_BYTES + payload.len();
        let mut key = [0u8; 16];
        key[..9].copy_from_slice(&prefix);
        if reverse_key {
            key.reverse();
        }
        let mut bytes = Vec::with_capacity(size);
        bytes.extend_from_slice(&key);
        bytes.extend_from_slice(&(size as i32).to_le_bytes());
        bytes.extend_from_slice(&[0u8; 10]);
        bytes.extend_from_slice(payload);
        bytes
    }

    #[test]
    fn reads_bounded_archive_record_and_strips_envelope() {
        let td = tempfile::tempdir().unwrap();
        let path = td.path().join("data.003");
        let payload = b"BLTEpayload";
        let bytes = record([0x42; 9], payload, false);
        let mut file = vec![0u8; 19];
        file.extend_from_slice(&bytes);
        fs::write(&path, file).unwrap();
        let reader = ArchiveReader::from_data_root(td.path().to_path_buf());
        let decoded = reader
            .read_blte(
                EncodingKeyPrefix::from_bytes([0x42; 9]),
                ArchiveLocation {
                    archive_index: 3,
                    offset: 19,
                    encoded_size: bytes.len() as u32,
                },
            )
            .unwrap();
        assert_eq!(decoded, payload);
    }

    #[test]
    fn accepts_reversed_envelope_key_and_rejects_mismatch() {
        let td = tempfile::tempdir().unwrap();
        let good = record([0x11; 9], b"BLTE", true);
        fs::write(td.path().join("data.000"), &good).unwrap();
        let reader = ArchiveReader::from_data_root(td.path().to_path_buf());
        let location = ArchiveLocation {
            archive_index: 0,
            offset: 0,
            encoded_size: good.len() as u32,
        };
        assert!(
            reader
                .read_blte(EncodingKeyPrefix::from_bytes([0x11; 9]), location)
                .is_ok()
        );
        assert!(
            reader
                .read_blte(EncodingKeyPrefix::from_bytes([0x22; 9]), location)
                .is_err()
        );
    }

    #[test]
    fn rejects_out_of_bounds_and_size_mismatch() {
        let td = tempfile::tempdir().unwrap();
        let mut bytes = record([1; 9], b"BLTE", false);
        bytes[16..20].copy_from_slice(&999i32.to_le_bytes());
        fs::write(td.path().join("data.000"), &bytes).unwrap();
        let reader = ArchiveReader::from_data_root(td.path().to_path_buf());
        assert!(
            reader
                .read_blte(
                    EncodingKeyPrefix::from_bytes([1; 9]),
                    ArchiveLocation {
                        archive_index: 0,
                        offset: 0,
                        encoded_size: bytes.len() as u32,
                    },
                )
                .is_err()
        );
        assert!(
            reader
                .read_blte(
                    EncodingKeyPrefix::from_bytes([1; 9]),
                    ArchiveLocation {
                        archive_index: 0,
                        offset: 9999,
                        encoded_size: bytes.len() as u32,
                    },
                )
                .is_err()
        );
    }
}
