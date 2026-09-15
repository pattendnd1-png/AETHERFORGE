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
    NameLengthLimit { entry: usize, length: u32, max: u32 },
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
    #[error(
        "entry {entry} data offset {offset} does not increase after previous offset {previous}"
    )]
    DataOffsetsNotIncreasing {
        entry: usize,
        previous: u32,
        offset: u32,
    },
    #[error(
        "entry {entry} needs {expected} raw bytes but only {available} stored bytes are available"
    )]
    StoredSpanTooSmall {
        entry: usize,
        expected: usize,
        available: usize,
    },
    #[error(
        "entry {entry} compressed header advertises size {compressed_size}, smaller than 12 bytes"
    )]
    CompressedSizeTooSmall { entry: usize, compressed_size: u32 },
    #[error("entry {entry} compressed size {compressed_size} exceeds stored span {available}")]
    CompressedSizeOutOfRange {
        entry: usize,
        compressed_size: u32,
        available: usize,
    },
    #[error(
        "entry {entry} directory size {directory_size} disagrees with compression header size {header_size}"
    )]
    DecompressedSizeMismatch {
        entry: usize,
        directory_size: u32,
        header_size: u32,
    },
    #[error("entry {entry} output size {size} exceeds configured limit {limit}")]
    OutputLimitExceeded { entry: usize, size: u64, limit: u64 },
    #[error("compressed payload ended at byte {offset} before output was complete")]
    CompressedPayloadEof { offset: usize },
    #[error("invalid back-reference offset {offset} after producing {produced} bytes")]
    InvalidBackReference { offset: u16, produced: usize },
    #[error(
        "back-reference would grow output from {produced} to {attempted}, past expected {expected}"
    )]
    OutputOverflow {
        produced: usize,
        attempted: usize,
        expected: usize,
    },
    #[error("entry index {index} is outside archive entry count {entry_count}")]
    EntryIndexOutOfRange { index: usize, entry_count: usize },
}
