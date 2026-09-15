use forgeclean::coldpack_gc::{
    QUARANTINE_SECONDS, coldpack_audit, coldpack_gc_apply, coldpack_gc_preview, coldpack_status,
};
use forgeclean::coldstore::{archive_to_coldpack, restore_coldpack_archive};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_root(tag: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("forgeclean-{tag}-{}-{stamp}", std::process::id()))
}

fn create_archive(root: &Path, name: &str, byte: u8) -> (PathBuf, PathBuf) {
    let source = root.join(format!("{name}-source"));
    let cold = root.join("Projects").join(name).join("Cold");
    let store = root.join("ColdStorage/.coldpack-store");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("payload.bin"), vec![byte; 900_000]).unwrap();
    let record = archive_to_coldpack(&source, &cold, &store, name).unwrap();
    (record.archive_path, store)
}

#[test]
fn status_reports_live_and_orphan_objects_without_mutation() {
    let root = temp_root("gc-status");
    let (live_manifest, store) = create_archive(&root, "Live", 0x11);
    let (orphan_manifest, _) = create_archive(&root, "Orphan", 0x22);
    fs::remove_file(&orphan_manifest).unwrap();
    fs::remove_file(format!("{}.sha256", orphan_manifest.display())).unwrap();
    let before = snapshot_files(&store.join("objects"));

    let report = coldpack_status(&root, &store, 2_000_000).unwrap();
    assert_eq!(report.manifests, 1);
    assert!(report.live_objects > 0);
    assert!(report.orphan_objects > 0);
    assert_eq!(snapshot_files(&store.join("objects")), before);
    assert!(live_manifest.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn audit_blocks_on_corrupt_referenced_object() {
    let root = temp_root("gc-corrupt-live");
    let (_manifest, store) = create_archive(&root, "Live", 0x33);
    let object = snapshot_files(&store.join("objects"))
        .into_iter()
        .next()
        .unwrap();
    fs::write(&object, b"corrupt").unwrap();
    let error = coldpack_audit(&root, &store, 2_000_000).unwrap_err();
    assert!(error.contains("ColdPack object"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn preview_is_non_mutating_and_apply_quarantines_only_orphans() {
    let root = temp_root("gc-apply");
    let (live_manifest, store) = create_archive(&root, "Live", 0x44);
    let (orphan_manifest, _) = create_archive(&root, "Orphan", 0x55);
    fs::remove_file(&orphan_manifest).unwrap();
    fs::remove_file(format!("{}.sha256", orphan_manifest.display())).unwrap();
    let before = snapshot_files(&store.join("objects"));

    let preview = coldpack_gc_preview(&root, &store, 3_000_000).unwrap();
    assert!(preview.orphan_objects > 0);
    assert_eq!(snapshot_files(&store.join("objects")), before);

    let applied = coldpack_gc_apply(&root, &store, 3_000_000).unwrap();
    assert_eq!(applied.quarantined_objects, preview.orphan_objects);
    assert!(applied.quarantined_bytes > 0);
    assert!(!snapshot_files(&store.join("quarantine/3000000")).is_empty());

    let restored = root.join("Restored");
    restore_coldpack_archive(&live_manifest, &store, &restored).unwrap();
    assert_eq!(
        fs::read(restored.join("Live-source/payload.bin")).unwrap(),
        vec![0x44; 900_000]
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn apply_purges_only_expired_still_unreferenced_quarantine() {
    let root = temp_root("gc-purge");
    let (manifest, store) = create_archive(&root, "Orphan", 0x66);
    fs::remove_file(&manifest).unwrap();
    fs::remove_file(format!("{}.sha256", manifest.display())).unwrap();
    let t0 = 4_000_000;
    let first = coldpack_gc_apply(&root, &store, t0).unwrap();
    assert!(first.quarantined_objects > 0);
    let before_expiry = coldpack_gc_apply(&root, &store, t0 + QUARANTINE_SECONDS - 1).unwrap();
    assert_eq!(before_expiry.purged_objects, 0);
    let expired = coldpack_gc_apply(&root, &store, t0 + QUARANTINE_SECONDS).unwrap();
    assert!(expired.purged_objects > 0);
    assert_eq!(snapshot_files(&store.join("quarantine")).len(), 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn gc_fails_closed_on_corrupt_manifest_without_moving_objects() {
    let root = temp_root("gc-corrupt-manifest");
    let (manifest, store) = create_archive(&root, "Live", 0x77);
    let before = snapshot_files(&store.join("objects"));
    fs::write(&manifest, b"{not-json}").unwrap();
    let error = coldpack_gc_apply(&root, &store, 5_000_000).unwrap_err();
    assert!(error.contains("checksum mismatch") || error.contains("parse ColdPack manifest"));
    assert_eq!(snapshot_files(&store.join("objects")), before);
    assert_eq!(snapshot_files(&store.join("quarantine")).len(), 0);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn status_rejects_conflicting_lengths_for_same_chunk_hash() {
    let root = temp_root("gc-conflicting-lengths");
    let (_first, store) = create_archive(&root, "First", 0x19);
    let (second, _) = create_archive(&root, "Second", 0x19);
    let mut json: serde_json::Value = serde_json::from_slice(&fs::read(&second).unwrap()).unwrap();
    {
        let entries = json["entries"].as_array_mut().unwrap();
        let file = entries
            .iter_mut()
            .find(|entry| entry["kind"].as_str() == Some("file"))
            .unwrap();
        let chunks = file["chunks"].as_array_mut().unwrap();
        let old_length = chunks[0]["length"].as_u64().unwrap();
        chunks[0]["length"] = serde_json::Value::from(old_length + 1);
        let old_size = file["size"].as_u64().unwrap();
        file["size"] = serde_json::Value::from(old_size + 1);
    }
    let old_logical = json["logical_bytes"].as_u64().unwrap();
    json["logical_bytes"] = serde_json::Value::from(old_logical + 1);
    let bytes = serde_json::to_vec(&json).unwrap();
    fs::write(&second, &bytes).unwrap();
    rewrite_checksum(&second, &bytes);

    let error = coldpack_status(&root, &store, 5_000_000).unwrap_err();
    assert!(error.contains("conflicting ColdPack chunk lengths"));
    fs::remove_dir_all(root).unwrap();
}

fn rewrite_checksum(manifest: &Path, bytes: &[u8]) {
    let hash = format!("{:x}", Sha256::digest(bytes));
    let mut sidecar = manifest.as_os_str().to_os_string();
    sidecar.push(".sha256");
    let name = manifest.file_name().unwrap().to_string_lossy();
    fs::write(PathBuf::from(sidecar), format!("{hash}  {name}\n")).unwrap();
}

fn snapshot_files(root: &Path) -> Vec<PathBuf> {
    fn walk(path: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.is_file() {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out.sort();
    out
}
