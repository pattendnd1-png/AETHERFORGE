use forgeclean::manifest::{CleanupBatch, CleanupCategory};
use forgeclean::package::VersionComparator;
use forgeclean::scan::{ScanOptions, scan_cache};
use std::cmp::Ordering;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

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

#[test]
fn scan_selects_only_superseded_package_versions() {
    let root = temp_dir("scan");
    for version in ["1", "2", "3"] {
        fs::write(
            root.join(format!("mesa-{version}-1-x86_64.pkg.tar.zst")),
            version,
        )
        .unwrap();
    }
    fs::write(root.join("unrelated.txt"), b"keep").unwrap();
    let batch = scan_cache(
        &ScanOptions {
            cache_dir: root.clone(),
            keep_versions: 2,
            partial_age_days: 7,
        },
        &NumericCmp,
    )
    .unwrap();
    assert_eq!(batch.entries.len(), 1);
    assert_eq!(
        batch.entries[0].category,
        CleanupCategory::SupersededPackage
    );
    assert!(
        batch.entries[0]
            .path
            .ends_with("mesa-1-1-x86_64.pkg.tar.zst")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn manifest_round_trip_preserves_identity() {
    let root = temp_dir("manifest");
    for version in ["1", "2"] {
        fs::write(
            root.join(format!("pkg-{version}-1-x86_64.pkg.tar.zst")),
            version,
        )
        .unwrap();
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
    let manifest = root.join("batch.txt");
    batch.write_to(&manifest).unwrap();
    let reread = CleanupBatch::read_from(&manifest).unwrap();
    assert_eq!(batch, reread);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn fresh_partial_is_not_selected() {
    let root = temp_dir("partial");
    fs::write(root.join("pkg-1-1-x86_64.pkg.tar.zst.part"), b"active").unwrap();
    let batch = scan_cache(
        &ScanOptions {
            cache_dir: root.clone(),
            keep_versions: 2,
            partial_age_days: 7,
        },
        &NumericCmp,
    )
    .unwrap();
    assert!(batch.entries.is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn zero_day_partial_age_selects_partial() {
    let root = temp_dir("partial-zero");
    let file = root.join("pkg-1-1-x86_64.pkg.tar.zst.part");
    fs::write(&file, b"stale").unwrap();
    std::thread::sleep(Duration::from_millis(2));
    let batch = scan_cache(
        &ScanOptions {
            cache_dir: root.clone(),
            keep_versions: 2,
            partial_age_days: 0,
        },
        &NumericCmp,
    )
    .unwrap();
    assert_eq!(batch.entries.len(), 1);
    assert_eq!(batch.entries[0].category, CleanupCategory::PartialDownload);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn installed_version_pin_survives_retention_selection() {
    use forgeclean::scan::protect_installed_versions;
    use std::collections::BTreeMap;

    let root = temp_dir("installed-pin");
    for version in ["1", "2", "3"] {
        fs::write(
            root.join(format!("mesa-{version}-1-x86_64.pkg.tar.zst")),
            version,
        )
        .unwrap();
    }
    let mut batch = scan_cache(
        &ScanOptions {
            cache_dir: root.clone(),
            keep_versions: 1,
            partial_age_days: 7,
        },
        &NumericCmp,
    )
    .unwrap();
    assert_eq!(batch.entries.len(), 2);
    let installed = BTreeMap::from([("mesa".to_owned(), "1-1".to_owned())]);
    let pins = protect_installed_versions(&mut batch, &installed);
    assert_eq!(pins, 1);
    assert_eq!(batch.entries.len(), 1);
    assert!(
        batch.entries[0]
            .path
            .ends_with("mesa-2-1-x86_64.pkg.tar.zst")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn superseded_package_signature_is_batched_with_archive() {
    let root = temp_dir("signature-sidecar");
    for version in ["1", "2"] {
        let pkg = root.join(format!("mesa-{version}-1-x86_64.pkg.tar.zst"));
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
    assert!(batch.entries.iter().any(|e| {
        e.category == CleanupCategory::SupersededPackage
            && e.path.ends_with("mesa-1-1-x86_64.pkg.tar.zst")
    }));
    assert!(batch.entries.iter().any(|e| {
        e.category == CleanupCategory::PackageSignature
            && e.path.ends_with("mesa-1-1-x86_64.pkg.tar.zst.sig")
    }));
    assert!(
        !batch
            .entries
            .iter()
            .any(|e| e.path.ends_with("mesa-2-1-x86_64.pkg.tar.zst.sig"))
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn signature_symlink_is_never_batched() {
    use std::os::unix::fs::symlink;
    let root = temp_dir("signature-symlink");
    let old = root.join("mesa-1-1-x86_64.pkg.tar.zst");
    let new = root.join("mesa-2-1-x86_64.pkg.tar.zst");
    fs::write(&old, b"old").unwrap();
    fs::write(&new, b"new").unwrap();
    let target = root.join("unrelated.sig");
    fs::write(&target, b"keep").unwrap();
    symlink(&target, format!("{}.sig", old.display())).unwrap();
    let batch = scan_cache(
        &ScanOptions {
            cache_dir: root.clone(),
            keep_versions: 1,
            partial_age_days: 7,
        },
        &NumericCmp,
    )
    .unwrap();
    assert_eq!(batch.entries.len(), 1);
    assert!(
        batch
            .entries
            .iter()
            .all(|e| e.category != CleanupCategory::PackageSignature)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn installed_version_pin_keeps_archive_and_signature_pair() {
    use forgeclean::scan::protect_installed_versions;
    use std::collections::BTreeMap;

    let root = temp_dir("installed-signature-pin");
    for version in ["1", "2", "3"] {
        let pkg = root.join(format!("mesa-{version}-1-x86_64.pkg.tar.zst"));
        fs::write(&pkg, version).unwrap();
        fs::write(format!("{}.sig", pkg.display()), format!("sig-{version}")).unwrap();
    }
    let mut batch = scan_cache(
        &ScanOptions {
            cache_dir: root.clone(),
            keep_versions: 1,
            partial_age_days: 7,
        },
        &NumericCmp,
    )
    .unwrap();
    assert_eq!(batch.entries.len(), 4);
    let installed = BTreeMap::from([("mesa".to_owned(), "1-1".to_owned())]);
    let pins = protect_installed_versions(&mut batch, &installed);
    assert_eq!(pins, 1);
    assert_eq!(batch.entries.len(), 2);
    assert!(
        batch
            .entries
            .iter()
            .all(|e| !e.path.ends_with("mesa-1-1-x86_64.pkg.tar.zst"))
    );
    assert!(
        batch
            .entries
            .iter()
            .all(|e| !e.path.ends_with("mesa-1-1-x86_64.pkg.tar.zst.sig"))
    );
    assert!(
        batch
            .entries
            .iter()
            .any(|e| e.path.ends_with("mesa-2-1-x86_64.pkg.tar.zst"))
    );
    assert!(
        batch
            .entries
            .iter()
            .any(|e| e.path.ends_with("mesa-2-1-x86_64.pkg.tar.zst.sig"))
    );
    fs::remove_dir_all(root).unwrap();
}
