use forgeclean::offload::offload_batch;
use forgeclean::package::VersionComparator;
use forgeclean::scan::{ScanOptions, scan_cache};
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

struct NumericCmp;
impl VersionComparator for NumericCmp {
    fn compare(&self, a: &str, b: &str) -> Result<Ordering, String> {
        let leading = |s: &str| s.split('-').next().unwrap().parse::<u32>().unwrap();
        Ok(leading(a).cmp(&leading(b)))
    }
}

fn temp_dir(tag: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "forgeclean-offload-{tag}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn batch(root: &Path) -> forgeclean::manifest::CleanupBatch {
    fs::write(root.join("pkg-1-1-x86_64.pkg.tar.zst"), b"old-package").unwrap();
    fs::write(root.join("pkg-2-1-x86_64.pkg.tar.zst"), b"new-package").unwrap();
    scan_cache(
        &ScanOptions {
            cache_dir: root.to_path_buf(),
            keep_versions: 1,
            partial_age_days: 7,
        },
        &NumericCmp,
    )
    .unwrap()
}

#[test]
fn verified_offload_copies_then_removes_source() {
    let source = temp_dir("success-src");
    let destination = temp_dir("success-dst");
    let batch = batch(&source);
    let report = offload_batch(&batch, &destination);
    assert!(report.is_ok(), "{:?}", report.failures);
    assert_eq!(report.offloaded_files, 1);
    assert!(!source.join("pkg-1-1-x86_64.pkg.tar.zst").exists());
    assert_eq!(
        fs::read(destination.join("pkg-1-1-x86_64.pkg.tar.zst")).unwrap(),
        b"old-package"
    );
    assert!(source.join("pkg-2-1-x86_64.pkg.tar.zst").exists());
    fs::remove_dir_all(source).unwrap();
    fs::remove_dir_all(destination).unwrap();
}

#[test]
fn conflicting_destination_preserves_source() {
    let source = temp_dir("conflict-src");
    let destination = temp_dir("conflict-dst");
    let batch = batch(&source);
    fs::write(destination.join("pkg-1-1-x86_64.pkg.tar.zst"), b"different").unwrap();
    let report = offload_batch(&batch, &destination);
    assert!(!report.is_ok());
    assert!(source.join("pkg-1-1-x86_64.pkg.tar.zst").exists());
    assert_eq!(
        fs::read(destination.join("pkg-1-1-x86_64.pkg.tar.zst")).unwrap(),
        b"different"
    );
    fs::remove_dir_all(source).unwrap();
    fs::remove_dir_all(destination).unwrap();
}

#[test]
fn changed_source_is_never_deleted() {
    let source = temp_dir("changed-src");
    let destination = temp_dir("changed-dst");
    let batch = batch(&source);
    fs::write(
        source.join("pkg-1-1-x86_64.pkg.tar.zst"),
        b"changed-after-scan",
    )
    .unwrap();
    let report = offload_batch(&batch, &destination);
    assert!(!report.is_ok());
    assert!(source.join("pkg-1-1-x86_64.pkg.tar.zst").exists());
    fs::remove_dir_all(source).unwrap();
    fs::remove_dir_all(destination).unwrap();
}

#[cfg(unix)]
#[test]
fn destination_component_symlink_is_rejected() {
    use forgeclean::offload::prepare_destination_under_mount;
    use std::os::unix::fs::symlink;
    let mount = temp_dir("mount-guard");
    let outside = temp_dir("mount-outside");
    symlink(&outside, mount.join("AetherForge")).unwrap();
    let result = prepare_destination_under_mount(&mount);
    assert!(result.is_err());
    fs::remove_dir_all(mount).unwrap();
    fs::remove_dir_all(outside).unwrap();
}

#[test]
fn external_partition_keeps_partials_in_cleanup_only_batch() {
    use forgeclean::manifest::{CleanupBatch, CleanupCategory, CleanupEntry, FileIdentity};
    let root = temp_dir("partition-root");
    let partial = root.join("pkg-3-1-x86_64.pkg.tar.zst.part");
    fs::write(&partial, b"partial").unwrap();
    let md = fs::symlink_metadata(&partial).unwrap();
    let batch = CleanupBatch {
        root: root.clone(),
        keep_versions: 2,
        partial_age_days: 7,
        entries: vec![CleanupEntry {
            path: partial,
            bytes: md.len(),
            category: CleanupCategory::PartialDownload,
            reason: "stale partial".to_owned(),
            identity: FileIdentity::from_metadata(&md),
        }],
    };
    let (offload, cleanup) = forgeclean::offload::partition_external_batch(&batch);
    assert!(offload.entries.is_empty());
    assert_eq!(cleanup.entries.len(), 1);
    fs::remove_dir_all(root).unwrap();
}
