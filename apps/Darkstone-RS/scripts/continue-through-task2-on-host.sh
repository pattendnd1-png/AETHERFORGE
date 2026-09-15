#!/usr/bin/env bash
set -euo pipefail

repo="${1:-$(pwd)}"
cd "$repo"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    printf 'DARKSTONE_TASK2_BLOCKED=missing-%s\n' "$1" >&2
    exit 70
  }
}

need cargo
need rustc
need python3

in_git=false
if command -v git >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  in_git=true
  branch="$(git branch --show-current)"
  if [[ "$branch" != "feature/initial-rebuild" ]]; then
    printf 'DARKSTONE_TASK2_BLOCKED=wrong-branch:%s\n' "$branch" >&2
    exit 71
  fi
else
  printf 'DARKSTONE_TASK2_GIT=standalone-no-commit\n'
fi

printf 'RUSTC=%s\n' "$(rustc --version)"
printf 'CARGO=%s\n' "$(cargo --version)"

# Task 1 must be GREEN before Task 2 is staged. Fresh RED bundles use the
# existing continuation runner; already-advanced trees are re-verified.
if grep -q '^pub struct MeshVertex' crates/darkstone-assets/src/lib.rs 2>/dev/null; then
  cargo fmt --all --check
  cargo test -p darkstone-assets
  cargo clippy -p darkstone-assets --all-targets -- -D warnings
  printf 'DARKSTONE_TASK1_GREEN=PASS-existing\n'
else
  ./scripts/continue-task1-on-host.sh "$repo"
fi

# Stage Task 2's test harness only after Task 1 has passed.
python3 - <<'PY'
from pathlib import Path
p = Path('Cargo.toml')
s = p.read_text()
needle = 'members = [\n  "crates/darkstone-assets",\n]'
replacement = 'members = [\n  "crates/darkstone-assets",\n  "crates/darkstone-mtf",\n]'
if needle in s:
    s = s.replace(needle, replacement)
elif '"crates/darkstone-mtf"' not in s:
    raise SystemExit('DARKSTONE_TASK2_BLOCKED=workspace-members-shape')
p.write_text(s)
PY

mkdir -p crates/darkstone-mtf/src crates/darkstone-mtf/tests
cat > crates/darkstone-mtf/Cargo.toml <<'TOML'
[package]
name = "darkstone-mtf"
version.workspace = true
edition.workspace = true
license.workspace = true
rust-version.workspace = true

[dependencies]
thiserror.workspace = true
TOML

cat > crates/darkstone-mtf/src/lib.rs <<'RUST'
//! Hardened Darkstone MTF archive reader.
//!
//! Production parser contracts are intentionally absent in the Task-2 RED state.
RUST

cat > crates/darkstone-mtf/tests/archive.rs <<'RUST'
use std::sync::Arc;

use darkstone_mtf::{Limits, MtfArchive, MtfError};

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn entry_header_len(name: &[u8]) -> usize {
    4 + name.len() + 4 + 4
}

fn single_entry(name: &[u8], uncompressed_size: u32, stored: &[u8]) -> Arc<[u8]> {
    let data_offset = 4 + entry_header_len(name);
    let mut out = Vec::with_capacity(data_offset + stored.len());
    push_u32(&mut out, 1);
    push_u32(&mut out, name.len() as u32);
    out.extend_from_slice(name);
    push_u32(&mut out, data_offset as u32);
    push_u32(&mut out, uncompressed_size);
    out.extend_from_slice(stored);
    out.into()
}

fn compressed_block(expected: u32, payload: &[u8]) -> Vec<u8> {
    let compressed_size = 12 + payload.len();
    let mut out = Vec::with_capacity(compressed_size);
    out.extend_from_slice(&[0xAE, 0xBE]);
    out.extend_from_slice(&0u16.to_le_bytes());
    push_u32(&mut out, compressed_size as u32);
    push_u32(&mut out, expected);
    out.extend_from_slice(payload);
    out
}

#[test]
fn parses_one_uncompressed_entry() {
    let bytes = single_entry(b"TEST\\HELLO.TXT\0", 5, b"hello");
    let archive = MtfArchive::parse(bytes, Limits::default()).unwrap();

    assert_eq!(archive.entries().len(), 1);
    assert_eq!(archive.entries()[0].name, "TEST\\HELLO.TXT");
    assert_eq!(archive.entries()[0].raw_name, b"TEST\\HELLO.TXT");
    assert_eq!(archive.read_entry(0).unwrap(), b"hello");
}

#[test]
fn decompresses_literal_chunk() {
    let stored = compressed_block(8, b"\xFF12345678");
    let bytes = single_entry(b"A.BIN\0", 8, &stored);
    let archive = MtfArchive::parse(bytes, Limits::default()).unwrap();

    assert_eq!(archive.read_entry(0).unwrap(), b"12345678");
}

#[test]
fn decompresses_overlapping_back_reference() {
    let word = ((3u16) << 10) | 2; // count = 3 + 3 = 6, offset = 2
    let mut payload = vec![0b0000_0011, b'A', b'B'];
    payload.extend_from_slice(&word.to_le_bytes());
    let stored = compressed_block(8, &payload);
    let bytes = single_entry(b"OVERLAP.BIN\0", 8, &stored);
    let archive = MtfArchive::parse(bytes, Limits::default()).unwrap();

    assert_eq!(archive.read_entry(0).unwrap(), b"ABABABAB");
}

#[test]
fn rejects_filename_over_limit() {
    let bytes = single_entry(b"TOO-LONG\0", 1, b"x");
    let limits = Limits {
        max_name_bytes: 4,
        ..Limits::default()
    };
    let err = MtfArchive::parse(bytes, limits).unwrap_err();

    assert!(matches!(err, MtfError::NameLengthLimit { .. }));
}

#[test]
fn rejects_decreasing_data_offsets() {
    let a = b"A\0";
    let b = b"B\0";
    let directory_end = 4 + entry_header_len(a) + entry_header_len(b);
    let mut bytes = Vec::new();
    push_u32(&mut bytes, 2);
    push_u32(&mut bytes, a.len() as u32);
    bytes.extend_from_slice(a);
    push_u32(&mut bytes, (directory_end + 4) as u32);
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, b.len() as u32);
    bytes.extend_from_slice(b);
    push_u32(&mut bytes, directory_end as u32);
    push_u32(&mut bytes, 1);
    bytes.resize(directory_end + 8, 0);

    let err = MtfArchive::parse(bytes.into(), Limits::default()).unwrap_err();
    assert!(matches!(err, MtfError::DataOffsetsNotIncreasing { .. }));
}

#[test]
fn rejects_data_offset_outside_archive() {
    let name = b"A\0";
    let mut bytes = Vec::new();
    push_u32(&mut bytes, 1);
    push_u32(&mut bytes, name.len() as u32);
    bytes.extend_from_slice(name);
    push_u32(&mut bytes, 4096);
    push_u32(&mut bytes, 1);

    let err = MtfArchive::parse(bytes.into(), Limits::default()).unwrap_err();
    assert!(matches!(err, MtfError::DataOffsetOutOfRange { .. }));
}

#[test]
fn rejects_compression_header_size_mismatch() {
    let mut stored = compressed_block(9, b"\xFF12345678");
    // Directory advertises 8 bytes while the compression header advertises 9.
    let bytes = single_entry(b"BAD.BIN\0", 8, &stored);
    stored.fill(0); // prove the archive owns its input and does not borrow this buffer
    let archive = MtfArchive::parse(bytes, Limits::default()).unwrap();
    let err = archive.read_entry(0).unwrap_err();

    assert!(matches!(err, MtfError::DecompressedSizeMismatch { .. }));
}

#[test]
fn rejects_zero_back_reference_offset() {
    let payload = [0u8, 0, 0];
    let stored = compressed_block(3, &payload);
    let bytes = single_entry(b"ZERO.BIN\0", 3, &stored);
    let archive = MtfArchive::parse(bytes, Limits::default()).unwrap();
    let err = archive.read_entry(0).unwrap_err();

    assert!(matches!(
        err,
        MtfError::InvalidBackReference {
            offset: 0,
            produced: 0
        }
    ));
}

#[test]
fn rejects_back_reference_before_output_start() {
    let word = 1u16; // count=3, offset=1, but nothing has been produced yet
    let mut payload = vec![0u8];
    payload.extend_from_slice(&word.to_le_bytes());
    let stored = compressed_block(3, &payload);
    let bytes = single_entry(b"BEFORE.BIN\0", 3, &stored);
    let archive = MtfArchive::parse(bytes, Limits::default()).unwrap();
    let err = archive.read_entry(0).unwrap_err();

    assert!(matches!(
        err,
        MtfError::InvalidBackReference {
            offset: 1,
            produced: 0
        }
    ));
}

#[test]
fn rejects_decompression_past_advertised_size() {
    let word = ((3u16) << 10) | 2; // six-byte copy after two literals
    let mut payload = vec![0b0000_0011, b'A', b'B'];
    payload.extend_from_slice(&word.to_le_bytes());
    let stored = compressed_block(5, &payload);
    let bytes = single_entry(b"OVERFLOW.BIN\0", 5, &stored);
    let archive = MtfArchive::parse(bytes, Limits::default()).unwrap();
    let err = archive.read_entry(0).unwrap_err();

    assert!(matches!(err, MtfError::OutputOverflow { .. }));
}

#[test]
fn enforces_decompressed_output_limit_before_allocation() {
    let stored = compressed_block(32, b"\xFF12345678");
    let bytes = single_entry(b"BIG.BIN\0", 32, &stored);
    let limits = Limits {
        max_entry_output: 16,
        ..Limits::default()
    };
    let archive = MtfArchive::parse(bytes, limits).unwrap();
    let err = archive.read_entry(0).unwrap_err();

    assert!(matches!(err, MtfError::OutputLimitExceeded { .. }));
}
RUST

cargo fmt --all

set +e
red_output="$(cargo test -p darkstone-mtf 2>&1)"
red_rc=$?
set -e
printf '%s\n' "$red_output"
if (( red_rc == 0 )); then
  echo 'DARKSTONE_TASK2_RED=FAIL-test-passed-before-implementation' >&2
  exit 72
fi
if ! grep -Eq 'unresolved import|unresolved imports|cannot find' <<<"$red_output"; then
  echo 'DARKSTONE_TASK2_RED=FAIL-unexpected-failure' >&2
  exit 73
fi
printf 'DARKSTONE_TASK2_RED=PASS\n'

# GREEN: only now install the parser/decompressor implementation.
cat > crates/darkstone-mtf/src/lib.rs <<'RUST'
//! Bounds-checked reader for Darkstone MTF archives.

mod decompress;
mod reader;

pub use reader::{Limits, MtfArchive, MtfEntry};

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MtfError {
    #[error("arithmetic overflow while parsing MTF data")]
    ArithmeticOverflow,
    #[error("unexpected end of MTF data at byte offset {offset}")]
    UnexpectedEof { offset: usize },
    #[error("archive declares {count} entries, limit is {max}")]
    EntryCountLimit { count: u32, max: u32 },
    #[error("entry {entry} declares filename length {length}, limit is {max}")]
    NameLengthLimit {
        entry: usize,
        length: u32,
        max: u32,
    },
    #[error("entry {entry} has an empty filename")]
    EmptyName { entry: usize },
    #[error("entry {entry} data offset {offset} is before directory end {directory_end}")]
    DataOffsetBeforeDirectory {
        entry: usize,
        offset: u32,
        directory_end: usize,
    },
    #[error("entry {entry} data offset {offset} is outside archive length {archive_len}")]
    DataOffsetOutOfRange {
        entry: usize,
        offset: u32,
        archive_len: usize,
    },
    #[error("entry {entry} data offset {offset} does not increase after previous offset {previous}")]
    DataOffsetsNotIncreasing {
        entry: usize,
        previous: u32,
        offset: u32,
    },
    #[error("entry {entry} needs {expected} raw bytes but only {available} stored bytes are available")]
    StoredSpanTooSmall {
        entry: usize,
        expected: usize,
        available: usize,
    },
    #[error("entry {entry} compressed header advertises size {compressed_size}, smaller than 12 bytes")]
    CompressedSizeTooSmall {
        entry: usize,
        compressed_size: u32,
    },
    #[error("entry {entry} compressed size {compressed_size} exceeds stored span {available}")]
    CompressedSizeOutOfRange {
        entry: usize,
        compressed_size: u32,
        available: usize,
    },
    #[error("entry {entry} directory size {directory_size} disagrees with compression header size {header_size}")]
    DecompressedSizeMismatch {
        entry: usize,
        directory_size: u32,
        header_size: u32,
    },
    #[error("entry {entry} output size {size} exceeds configured limit {limit}")]
    OutputLimitExceeded {
        entry: usize,
        size: u64,
        limit: u64,
    },
    #[error("compressed payload ended at byte {offset} before output was complete")]
    CompressedPayloadEof { offset: usize },
    #[error("invalid back-reference offset {offset} after producing {produced} bytes")]
    InvalidBackReference { offset: u16, produced: usize },
    #[error("back-reference would grow output from {produced} to {attempted}, past expected {expected}")]
    OutputOverflow {
        produced: usize,
        attempted: usize,
        expected: usize,
    },
    #[error("entry index {index} is outside archive entry count {entry_count}")]
    EntryIndexOutOfRange { index: usize, entry_count: usize },
}
RUST

cat > crates/darkstone-mtf/src/reader.rs <<'RUST'
use std::sync::Arc;

use crate::{decompress::decompress_payload, MtfError};

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
            let offset = usize::try_from(entry.data_offset).map_err(|_| MtfError::ArithmeticOverflow)?;
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
        let start =
            usize::try_from(entry.data_offset).map_err(|_| MtfError::ArithmeticOverflow)?;
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
RUST

cat > crates/darkstone-mtf/src/decompress.rs <<'RUST'
use crate::MtfError;

pub(crate) fn decompress_payload(payload: &[u8], expected: usize) -> Result<Vec<u8>, MtfError> {
    let mut out = Vec::with_capacity(expected);
    let mut at = 0usize;

    while out.len() < expected {
        let flags = take_u8(payload, &mut at)?;
        for bit in 0..8 {
            if out.len() == expected {
                break;
            }

            if flags & (1u8 << bit) != 0 {
                let byte = take_u8(payload, &mut at)?;
                out.push(byte);
                continue;
            }

            let word = take_u16(payload, &mut at)?;
            let count = usize::from(word >> 10) + 3;
            let offset = word & 0x03ff;
            if offset == 0 || usize::from(offset) > out.len() {
                return Err(MtfError::InvalidBackReference {
                    offset,
                    produced: out.len(),
                });
            }

            let attempted = out
                .len()
                .checked_add(count)
                .ok_or(MtfError::ArithmeticOverflow)?;
            if attempted > expected {
                return Err(MtfError::OutputOverflow {
                    produced: out.len(),
                    attempted,
                    expected,
                });
            }

            for _ in 0..count {
                let source = out
                    .len()
                    .checked_sub(usize::from(offset))
                    .ok_or(MtfError::InvalidBackReference {
                        offset,
                        produced: out.len(),
                    })?;
                let byte = out[source];
                out.push(byte);
            }
        }
    }

    Ok(out)
}

fn take_u8(bytes: &[u8], at: &mut usize) -> Result<u8, MtfError> {
    let value = bytes
        .get(*at)
        .copied()
        .ok_or(MtfError::CompressedPayloadEof { offset: *at })?;
    *at = (*at).checked_add(1).ok_or(MtfError::ArithmeticOverflow)?;
    Ok(value)
}

fn take_u16(bytes: &[u8], at: &mut usize) -> Result<u16, MtfError> {
    let end = at.checked_add(2).ok_or(MtfError::ArithmeticOverflow)?;
    let raw = bytes
        .get(*at..end)
        .ok_or(MtfError::CompressedPayloadEof { offset: *at })?;
    *at = end;
    Ok(u16::from_le_bytes(raw.try_into().unwrap()))
}
RUST

cargo fmt --all
cargo fmt --all --check
cargo test -p darkstone-mtf
cargo clippy -p darkstone-mtf --all-targets -- -D warnings
printf 'DARKSTONE_TASK2_GREEN=PASS\n'

if [[ "$in_git" == true ]]; then
  git add Cargo.toml crates/darkstone-mtf
  if ! git diff --cached --quiet; then
    git commit -m 'feat: add hardened Darkstone MTF reader'
  fi
  printf 'DARKSTONE_TASK2_COMMIT=%s\n' "$(git rev-parse --short HEAD)"
else
  printf 'DARKSTONE_TASK2_COMMIT=SKIPPED-standalone\n'
fi
