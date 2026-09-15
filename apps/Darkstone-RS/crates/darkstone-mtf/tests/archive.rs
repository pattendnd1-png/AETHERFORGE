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
