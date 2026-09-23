use forgeclean::package::VersionComparator;
use forgeclean::purge::{purge_batch, verify_batch};
use forgeclean::scan::{ScanOptions, scan_cache};
use std::cmp::Ordering;
use std::fs;
use std::path::PathBuf;
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
    let path =
        std::env::temp_dir().join(format!("forgeclean-{tag}-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}

fn scanned_batch(root: &std::path::Path) -> forgeclean::manifest::CleanupBatch {
    for version in ["1", "2"] {
        fs::write(
            root.join(format!("pkg-{version}-1-x86_64.pkg.tar.zst")),
            version,
        )
        .unwrap();
    }
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
fn purge_directly_removes_only_approved_file() {
    let root = temp_dir("purge");
    let batch = scanned_batch(&root);
    let target = batch.entries[0].path.clone();
    let kept = root.join("pkg-2-1-x86_64.pkg.tar.zst");
    assert!(verify_batch(&batch).is_ok());
    let report = purge_batch(&batch);
    assert!(report.is_ok(), "{:?}", report.skipped);
    assert_eq!(report.deleted_files, 1);
    assert!(!target.exists());
    assert!(kept.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn purge_refuses_file_changed_after_scan() {
    let root = temp_dir("changed");
    let batch = scanned_batch(&root);
    let target = batch.entries[0].path.clone();
    fs::write(&target, b"replacement-with-different-size").unwrap();
    let report = purge_batch(&batch);
    assert_eq!(report.deleted_files, 0);
    assert!(!report.skipped.is_empty());
    assert!(target.exists());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn purge_refuses_symlink_replacement() {
    use std::os::unix::fs::symlink;
    let root = temp_dir("symlink");
    let batch = scanned_batch(&root);
    let target = batch.entries[0].path.clone();
    let other = root.join("pkg-9-1-x86_64.pkg.tar.zst");
    fs::write(&other, b"other").unwrap();
    fs::remove_file(&target).unwrap();
    symlink(&other, &target).unwrap();
    let report = purge_batch(&batch);
    assert_eq!(report.deleted_files, 0);
    assert!(!report.skipped.is_empty());
    assert!(other.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn verification_blocks_path_escape_even_for_package_filename() {
    use forgeclean::manifest::FileIdentity;
    let root = temp_dir("escape-root");
    let outside_root = temp_dir("escape-outside");
    let mut batch = scanned_batch(&root);
    let outside = outside_root.join("pkg-0-1-x86_64.pkg.tar.zst");
    fs::write(&outside, b"outside").unwrap();
    let md = fs::symlink_metadata(&outside).unwrap();
    batch.entries[0].path = outside.clone();
    batch.entries[0].bytes = md.len();
    batch.entries[0].identity = FileIdentity::from_metadata(&md);
    let report = verify_batch(&batch);
    assert!(!report.is_ok());
    assert!(outside.exists());
    fs::remove_dir_all(root).unwrap();
    fs::remove_dir_all(outside_root).unwrap();
}

#[test]
fn purge_removes_selected_archive_and_signature_but_keeps_retained_pair() {
    let root = temp_dir("purge-signature");
    for version in ["1", "2"] {
        let pkg = root.join(format!("pkg-{version}-1-x86_64.pkg.tar.zst"));
        fs::write(&pkg, version).unwrap();
        fs::write(format!("{}.sig", pkg.display()), format!("sig-{version}")).unwrap();
    }
    let batch = scan_cache(
        &ScanOptions {
            cache_dir: root.clone(),
            keep_versions: 1,
            partial_age_days: 7,
        },
        &NumericCmp,
    )
    .unwrap();
    assert_eq!(batch.entries.len(), 2);
    let report = purge_batch(&batch);
    assert!(report.is_ok(), "{:?}", report.skipped);
    assert_eq!(report.deleted_files, 2);
    assert!(!root.join("pkg-1-1-x86_64.pkg.tar.zst").exists());
    assert!(!root.join("pkg-1-1-x86_64.pkg.tar.zst.sig").exists());
    assert!(root.join("pkg-2-1-x86_64.pkg.tar.zst").exists());
    assert!(root.join("pkg-2-1-x86_64.pkg.tar.zst.sig").exists());
    fs::remove_dir_all(root).unwrap();
}
