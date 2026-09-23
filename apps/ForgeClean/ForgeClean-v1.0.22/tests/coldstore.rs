use forgeclean::coldstore::{archive_to_cold, restore_cold_archive};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_root(tag: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("forgeclean-{tag}-{}-{stamp}", std::process::id()))
}

#[test]
fn archive_deletes_source_only_after_verified_archive_and_restores_exact_bytes() {
    let root = temp_root("coldstore-roundtrip");
    let source = root.join("Demo-v1.0.0");
    let cold = root.join("Cold");
    let restored = root.join("Restored");
    fs::create_dir_all(source.join("src")).unwrap();
    fs::write(
        source.join("src/main.rs"),
        b"fn main() { println!(\"hello\"); }\n",
    )
    .unwrap();
    let record = archive_to_cold(&source, &cold, "Demo-v1.0.0").unwrap();
    assert!(!source.exists());
    assert!(record.archive_path.is_file());
    assert!(record.checksum_path.is_file());
    let restored_roots = restore_cold_archive(&record.archive_path, &restored).unwrap();
    assert_eq!(restored_roots, vec![restored.join("Demo-v1.0.0")]);
    assert_eq!(
        fs::read(restored.join("Demo-v1.0.0/src/main.rs")).unwrap(),
        b"fn main() { println!(\"hello\"); }\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn restore_refuses_to_overwrite_existing_destination() {
    let root = temp_root("coldstore-overwrite");
    let source = root.join("payload.txt");
    let cold = root.join("Cold");
    let restored = root.join("Restored");
    fs::create_dir_all(&restored).unwrap();
    fs::write(&source, b"archive copy").unwrap();
    let record = archive_to_cold(&source, &cold, "payload.txt").unwrap();
    fs::write(restored.join("payload.txt"), b"existing").unwrap();
    let err = restore_cold_archive(&record.archive_path, &restored).unwrap_err();
    assert!(err.contains("restore destination already exists"));
    assert_eq!(fs::read(restored.join("payload.txt")).unwrap(), b"existing");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn deeply_nested_tree_archives_and_restores_without_stack_exhaustion() {
    let root = temp_root("coldstore-deep-tree");
    let source = root.join("DeepProject-v1.0.0");
    let cold = root.join("Cold");
    let restored = root.join("Restored");
    let mut leaf = source.clone();
    for depth in 0..96 {
        leaf = leaf.join(format!("d{depth:02}"));
    }
    fs::create_dir_all(&leaf).unwrap();
    fs::write(leaf.join("payload.bin"), vec![0x5a; 256 * 1024]).unwrap();

    let record = archive_to_cold(&source, &cold, "DeepProject-v1.0.0").unwrap();
    assert!(!source.exists());
    let restored_roots = restore_cold_archive(&record.archive_path, &restored).unwrap();
    assert_eq!(restored_roots, vec![restored.join("DeepProject-v1.0.0")]);

    let mut restored_leaf = restored.join("DeepProject-v1.0.0");
    for depth in 0..96 {
        restored_leaf = restored_leaf.join(format!("d{depth:02}"));
    }
    assert_eq!(
        fs::read(restored_leaf.join("payload.bin")).unwrap(),
        vec![0x5a; 256 * 1024]
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn coldpack_shared_store_reuses_identical_chunks_across_archives() {
    use forgeclean::coldstore::{archive_to_coldpack, coldpack_object_count};
    let root = temp_root("coldpack-dedup");
    let cold_a = root.join("Projects/A/Cold");
    let cold_b = root.join("Projects/B/Cold");
    let store = root.join("ColdStorage/.coldpack-store");
    let source_a = root.join("A-v1");
    let source_b = root.join("B-v1");
    fs::create_dir_all(&source_a).unwrap();
    fs::create_dir_all(&source_b).unwrap();
    let repeated = vec![0x41; 1024 * 1024];
    fs::write(source_a.join("shared.bin"), &repeated).unwrap();
    fs::write(source_b.join("shared.bin"), &repeated).unwrap();

    let first = archive_to_coldpack(&source_a, &cold_a, &store, "A-v1").unwrap();
    let after_first = coldpack_object_count(&store).unwrap();
    let second = archive_to_coldpack(&source_b, &cold_b, &store, "B-v1").unwrap();
    let after_second = coldpack_object_count(&store).unwrap();

    assert!(first.chunks_written > 0);
    assert_eq!(second.chunks_written, 0);
    assert!(second.chunks_reused > 0);
    assert_eq!(after_first, after_second);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn coldpack_restore_reconstructs_exact_bytes_from_shared_store() {
    use forgeclean::coldstore::{archive_to_coldpack, restore_coldpack_archive};
    let root = temp_root("coldpack-roundtrip");
    let source = root.join("Demo-v2");
    let cold = root.join("Projects/Demo/Cold");
    let store = root.join("ColdStorage/.coldpack-store");
    let restored = root.join("Restored");
    fs::create_dir_all(source.join("src")).unwrap();
    let payload: Vec<u8> = (0..2_000_000u32).map(|n| (n % 251) as u8).collect();
    fs::write(source.join("src/payload.bin"), &payload).unwrap();

    let record = archive_to_coldpack(&source, &cold, &store, "Demo-v2").unwrap();
    assert!(
        record
            .archive_path
            .extension()
            .is_some_and(|ext| ext == std::ffi::OsStr::new("fcoldpack"))
    );
    let restored_roots = restore_coldpack_archive(&record.archive_path, &store, &restored).unwrap();
    assert_eq!(restored_roots, vec![restored.join("Demo-v2")]);
    assert_eq!(
        fs::read(restored.join("Demo-v2/src/payload.bin")).unwrap(),
        payload
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn coldpack_corrupt_reused_object_preserves_new_source() {
    use forgeclean::coldstore::archive_to_coldpack;
    fn first_file(path: &std::path::Path) -> Option<PathBuf> {
        for entry in fs::read_dir(path).ok()? {
            let path = entry.ok()?.path();
            if path.is_dir() {
                if let Some(found) = first_file(&path) {
                    return Some(found);
                }
            } else if path.is_file() {
                return Some(path);
            }
        }
        None
    }

    let root = temp_root("coldpack-corruption");
    let store = root.join("ColdStorage/.coldpack-store");
    let first = root.join("First");
    let second = root.join("Second");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&second).unwrap();
    let data = vec![0x7b; 512 * 1024];
    fs::write(first.join("same.bin"), &data).unwrap();
    fs::write(second.join("same.bin"), &data).unwrap();
    archive_to_coldpack(&first, &root.join("ColdA"), &store, "First").unwrap();
    let object = first_file(&store.join("objects")).expect("chunk object");
    fs::write(&object, b"corrupt").unwrap();

    let error = archive_to_coldpack(&second, &root.join("ColdB"), &store, "Second").unwrap_err();
    assert!(error.contains("ColdPack object"));
    assert!(second.exists());
    assert_eq!(fs::read(second.join("same.bin")).unwrap(), data);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_tar_zst_archive_remains_restorable() {
    use forgeclean::coldstore::{archive_to_legacy_cold, restore_cold_archive};
    let root = temp_root("coldpack-legacy");
    let source = root.join("Legacy-v1");
    let cold = root.join("Cold");
    let restored = root.join("Restored");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("legacy.txt"), b"legacy bytes\n").unwrap();
    let record = archive_to_legacy_cold(&source, &cold, "Legacy-v1").unwrap();
    assert!(
        record
            .archive_path
            .to_string_lossy()
            .ends_with(".fcold.tar.zst")
    );
    let restored_roots = restore_cold_archive(&record.archive_path, &restored).unwrap();
    assert_eq!(restored_roots, vec![restored.join("Legacy-v1")]);
    assert_eq!(
        fs::read(restored.join("Legacy-v1/legacy.txt")).unwrap(),
        b"legacy bytes\n"
    );
    fs::remove_dir_all(root).unwrap();
}
