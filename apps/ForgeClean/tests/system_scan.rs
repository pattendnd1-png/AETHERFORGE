use forgeclean::system_scan::{
    SYSTEM_MANIFEST_FILENAME, SYSTEM_PACMAN_CACHE, default_system_manifest_path,
    system_scan_options,
};
use std::path::{Path, PathBuf};

#[test]
fn system_scan_uses_fixed_pacman_cache() {
    let options = system_scan_options(3, 9).unwrap();
    assert_eq!(options.cache_dir, PathBuf::from(SYSTEM_PACMAN_CACHE));
    assert_eq!(options.keep_versions, 3);
    assert_eq!(options.partial_age_days, 9);
}

#[test]
fn system_scan_rejects_zero_retention() {
    assert!(system_scan_options(0, 7).is_err());
}

#[test]
fn system_manifest_is_versioned_under_downloads() {
    let home = Path::new("/home/tester");
    assert_eq!(
        default_system_manifest_path(home),
        PathBuf::from("/home/tester/Downloads").join(SYSTEM_MANIFEST_FILENAME)
    );
}
