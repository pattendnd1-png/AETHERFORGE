use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use aether_system::{extract_zip_archive, list_zip_archive, validate_archive_entry};

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn write_stored_zip(path: &Path, name: &str, content: &[u8]) {
    let name_bytes = name.as_bytes();
    let crc = crc32(content);
    let mut bytes = Vec::new();

    // Local file header.
    write_u32(&mut bytes, 0x0403_4b50);
    write_u16(&mut bytes, 20);
    write_u16(&mut bytes, 0);
    write_u16(&mut bytes, 0);
    write_u16(&mut bytes, 0);
    write_u16(&mut bytes, 0);
    write_u32(&mut bytes, crc);
    write_u32(&mut bytes, content.len() as u32);
    write_u32(&mut bytes, content.len() as u32);
    write_u16(&mut bytes, name_bytes.len() as u16);
    write_u16(&mut bytes, 0);
    bytes.extend_from_slice(name_bytes);
    bytes.extend_from_slice(content);

    let central_offset = bytes.len() as u32;

    // Central directory header.
    write_u32(&mut bytes, 0x0201_4b50);
    write_u16(&mut bytes, 20);
    write_u16(&mut bytes, 20);
    write_u16(&mut bytes, 0);
    write_u16(&mut bytes, 0);
    write_u16(&mut bytes, 0);
    write_u16(&mut bytes, 0);
    write_u32(&mut bytes, crc);
    write_u32(&mut bytes, content.len() as u32);
    write_u32(&mut bytes, content.len() as u32);
    write_u16(&mut bytes, name_bytes.len() as u16);
    write_u16(&mut bytes, 0);
    write_u16(&mut bytes, 0);
    write_u16(&mut bytes, 0);
    write_u16(&mut bytes, 0);
    write_u32(&mut bytes, 0);
    write_u32(&mut bytes, 0);
    bytes.extend_from_slice(name_bytes);

    let central_size = bytes.len() as u32 - central_offset;

    // End of central directory.
    write_u32(&mut bytes, 0x0605_4b50);
    write_u16(&mut bytes, 0);
    write_u16(&mut bytes, 0);
    write_u16(&mut bytes, 1);
    write_u16(&mut bytes, 1);
    write_u32(&mut bytes, central_size);
    write_u32(&mut bytes, central_offset);
    write_u16(&mut bytes, 0);

    let mut file = fs::File::create(path).unwrap();
    file.write_all(&bytes).unwrap();
}

fn temp(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "aetherai-native-archive-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn unsafe_archive_entries_are_rejected_before_extraction() {
    assert!(validate_archive_entry("../escape.txt").is_err());
    assert!(validate_archive_entry("/absolute.txt").is_err());
    assert!(validate_archive_entry("safe/../../escape.txt").is_err());
    assert!(validate_archive_entry("safe/file.txt").is_ok());
}

#[test]
fn portable_archive_names_reject_windows_and_ambiguous_paths() {
    for unsafe_name in [
        r"..\escape.txt",
        r"C:\escape.txt",
        "C:/escape.txt",
        r"\\server\share\escape.txt",
        r"safe\mixed.txt",
        "safe//double.txt",
        "safe/./dot.txt",
        "safe/../escape.txt",
        "CON.txt",
        "safe/AUX.log",
        "safe/trailing.",
        "safe/trailing ",
    ] {
        assert!(
            validate_archive_entry(unsafe_name).is_err(),
            "unsafe archive entry unexpectedly accepted: {unsafe_name:?}"
        );
    }

    assert!(validate_archive_entry("safe/portable-file.txt").is_ok());
}

#[test]
fn extraction_refuses_nonempty_destination_without_modifying_it() {
    let root = temp("nonempty-destination");
    let archive = root.join("export.zip");
    let out = root.join("out");
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("keep.txt"), "keep").unwrap();
    write_stored_zip(&archive, "chatgpt/conversations.json", b"[]");

    assert!(extract_zip_archive(&archive, &out).is_err());
    assert_eq!(fs::read_to_string(out.join("keep.txt")).unwrap(), "keep");
    assert!(!out.join("chatgpt/conversations.json").exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn native_zip_listing_and_extraction_round_trip() {
    let root = temp("roundtrip");
    let archive = root.join("export.zip");
    let out = root.join("out");
    write_stored_zip(
        &archive,
        "chatgpt/conversations.json",
        br#"[{"title":"Imported"}]"#,
    );

    let entries = list_zip_archive(&archive).unwrap();
    assert_eq!(entries, vec!["chatgpt/conversations.json"]);

    extract_zip_archive(&archive, &out).unwrap();
    assert_eq!(
        fs::read_to_string(out.join("chatgpt/conversations.json")).unwrap(),
        r#"[{"title":"Imported"}]"#
    );
    let _ = fs::remove_dir_all(root);
}
