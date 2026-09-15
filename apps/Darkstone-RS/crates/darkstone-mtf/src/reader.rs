use std::sync::Arc;

use crate::{MtfError, decompress::decompress_payload};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_entries: u32,
    pub max_name_bytes: u32,
    pub max_entry_output: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_entries: 100_000,
            max_name_bytes: 1024,
            max_entry_output: 512 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MtfEntry {
    pub name: String,
    pub raw_name: Vec<u8>,
    pub data_offset: u32,
    pub uncompressed_size: u32,
    pub stored_size: u64,
}

#[derive(Debug, Clone)]
pub struct MtfArchive {
    bytes: Arc<[u8]>,
    entries: Vec<MtfEntry>,
    limits: Limits,
}

impl MtfArchive {
    pub fn parse(bytes: Arc<[u8]>, limits: Limits) -> Result<Self, MtfError> {
        let mut at = 0usize;
        let count = take_u32(&bytes, &mut at)?;
        if count > limits.max_entries {
            return Err(MtfError::EntryCountLimit {
                count,
                max: limits.max_entries,
            });
        }

        let capacity = usize::try_from(count).map_err(|_| MtfError::ArithmeticOverflow)?;
        let mut entries = Vec::with_capacity(capacity);
        for entry_index in 0..capacity {
            let name_len = take_u32(&bytes, &mut at)?;
            if name_len == 0 {
                return Err(MtfError::EmptyName { entry: entry_index });
            }
            if name_len > limits.max_name_bytes {
                return Err(MtfError::NameLengthLimit {
                    entry: entry_index,
                    length: name_len,
                    max: limits.max_name_bytes,
                });
            }

            let name_len_usize =
                usize::try_from(name_len).map_err(|_| MtfError::ArithmeticOverflow)?;
            let end = at
                .checked_add(name_len_usize)
                .ok_or(MtfError::ArithmeticOverflow)?;
            let stored_name = bytes
                .get(at..end)
                .ok_or(MtfError::UnexpectedEof { offset: at })?;
            at = end;

            let raw_name_slice = stored_name.strip_suffix(&[0]).unwrap_or(stored_name);
            if raw_name_slice.is_empty() {
                return Err(MtfError::EmptyName { entry: entry_index });
            }
            let raw_name = raw_name_slice.to_vec();
            let name = String::from_utf8_lossy(raw_name_slice).into_owned();
            let data_offset = take_u32(&bytes, &mut at)?;
            let uncompressed_size = take_u32(&bytes, &mut at)?;

            entries.push(MtfEntry {
                name,
                raw_name,
                data_offset,
                uncompressed_size,
                stored_size: 0,
            });
        }

        let directory_end = at;
        let mut previous_offset = None;
        for (entry_index, entry) in entries.iter().enumerate() {
            let offset =
                usize::try_from(entry.data_offset).map_err(|_| MtfError::ArithmeticOverflow)?;
            if offset < directory_end {
                return Err(MtfError::DataOffsetBeforeDirectory {
                    entry: entry_index,
                    offset: entry.data_offset,
                    directory_end,
                });
            }
            if offset > bytes.len() {
                return Err(MtfError::DataOffsetOutOfRange {
                    entry: entry_index,
                    offset: entry.data_offset,
                    archive_len: bytes.len(),
                });
            }
            if let Some(previous) = previous_offset {
                if entry.data_offset <= previous {
                    return Err(MtfError::DataOffsetsNotIncreasing {
                        entry: entry_index,
                        previous,
                        offset: entry.data_offset,
                    });
                }
            }
            previous_offset = Some(entry.data_offset);
        }

        for entry_index in 0..entries.len() {
            let start = usize::try_from(entries[entry_index].data_offset)
                .map_err(|_| MtfError::ArithmeticOverflow)?;
            let end = if let Some(next) = entries.get(entry_index + 1) {
                usize::try_from(next.data_offset).map_err(|_| MtfError::ArithmeticOverflow)?
            } else {
                bytes.len()
            };
            let span = end.checked_sub(start).ok_or(MtfError::ArithmeticOverflow)?;
            entries[entry_index].stored_size =
                u64::try_from(span).map_err(|_| MtfError::ArithmeticOverflow)?;
        }

        Ok(Self {
            bytes,
            entries,
            limits,
        })
    }

    #[must_use]
    pub fn entries(&self) -> &[MtfEntry] {
        &self.entries
    }

    pub fn read_entry(&self, index: usize) -> Result<Vec<u8>, MtfError> {
        let entry = self
            .entries
            .get(index)
            .ok_or(MtfError::EntryIndexOutOfRange {
                index,
                entry_count: self.entries.len(),
            })?;

        let output_size = u64::from(entry.uncompressed_size);
        if output_size > self.limits.max_entry_output {
            return Err(MtfError::OutputLimitExceeded {
                entry: index,
                size: output_size,
                limit: self.limits.max_entry_output,
            });
        }
        let expected =
            usize::try_from(entry.uncompressed_size).map_err(|_| MtfError::ArithmeticOverflow)?;
        let start = usize::try_from(entry.data_offset).map_err(|_| MtfError::ArithmeticOverflow)?;
        let stored_size =
            usize::try_from(entry.stored_size).map_err(|_| MtfError::ArithmeticOverflow)?;
        let end = start
            .checked_add(stored_size)
            .ok_or(MtfError::ArithmeticOverflow)?;
        let stored = self
            .bytes
            .get(start..end)
            .ok_or(MtfError::UnexpectedEof { offset: start })?;

        if is_compressed(stored) {
            if stored.len() < 12 {
                return Err(MtfError::UnexpectedEof { offset: start });
            }
            let compressed_size = u32::from_le_bytes(stored[4..8].try_into().unwrap());
            let header_size = u32::from_le_bytes(stored[8..12].try_into().unwrap());
            if header_size != entry.uncompressed_size {
                return Err(MtfError::DecompressedSizeMismatch {
                    entry: index,
                    directory_size: entry.uncompressed_size,
                    header_size,
                });
            }
            if compressed_size < 12 {
                return Err(MtfError::CompressedSizeTooSmall {
                    entry: index,
                    compressed_size,
                });
            }
            let compressed_size_usize =
                usize::try_from(compressed_size).map_err(|_| MtfError::ArithmeticOverflow)?;
            if compressed_size_usize > stored.len() {
                return Err(MtfError::CompressedSizeOutOfRange {
                    entry: index,
                    compressed_size,
                    available: stored.len(),
                });
            }
            return decompress_payload(&stored[12..compressed_size_usize], expected);
        }

        if expected > stored.len() {
            return Err(MtfError::StoredSpanTooSmall {
                entry: index,
                expected,
                available: stored.len(),
            });
        }
        Ok(stored[..expected].to_vec())
    }
}

fn is_compressed(stored: &[u8]) -> bool {
    matches!(stored.first().copied(), Some(0xAE | 0xAF)) && stored.get(1) == Some(&0xBE)
}

fn take_u32(bytes: &[u8], at: &mut usize) -> Result<u32, MtfError> {
    let end = at.checked_add(4).ok_or(MtfError::ArithmeticOverflow)?;
    let raw = bytes
        .get(*at..end)
        .ok_or(MtfError::UnexpectedEof { offset: *at })?;
    *at = end;
    Ok(u32::from_le_bytes(raw.try_into().unwrap()))
}
