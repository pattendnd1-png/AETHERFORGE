use crate::scan::ScanOptions;
use std::path::{Path, PathBuf};

pub const SYSTEM_PACMAN_CACHE: &str = "/var/cache/pacman/pkg";
pub const SYSTEM_MANIFEST_FILENAME: &str = "ForgeClean-v1.0.1-PACMAN-BATCH.txt";
pub const OFFLOAD_REPORT_FILENAME: &str = "ForgeClean-v1.0.1-OFFLOAD-RESULT.txt";

pub fn system_scan_options(
    keep_versions: usize,
    partial_age_days: u64,
) -> Result<ScanOptions, String> {
    if keep_versions == 0 {
        return Err("keep_versions must be at least 1".to_owned());
    }
    Ok(ScanOptions {
        cache_dir: PathBuf::from(SYSTEM_PACMAN_CACHE),
        keep_versions,
        partial_age_days,
    })
}

pub fn default_system_manifest_path(home: &Path) -> PathBuf {
    home.join("Downloads").join(SYSTEM_MANIFEST_FILENAME)
}

pub fn default_offload_report_path(home: &Path) -> PathBuf {
    home.join("Downloads").join(OFFLOAD_REPORT_FILENAME)
}
