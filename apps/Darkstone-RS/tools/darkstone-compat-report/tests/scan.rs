use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use darkstone_compat_report::{ScanError, scan_install};

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn synthetic_mtf(name: &[u8], payload: &[u8]) -> Vec<u8> {
    let directory_len = 4 + 4 + name.len() + 4 + 4;
    let mut out = Vec::with_capacity(directory_len + payload.len());
    push_u32(&mut out, 1);
    push_u32(&mut out, name.len() as u32);
    out.extend_from_slice(name);
    push_u32(&mut out, directory_len as u32);
    push_u32(&mut out, payload.len() as u32);
    out.extend_from_slice(payload);
    out
}

struct TestDir(PathBuf);

impl TestDir {
    fn new(label: &str) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "darkstone-rs-{label}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn scans_archives_read_only_and_hashes_them() {
    let dir = TestDir::new("scan");
    let data = synthetic_mtf(b"MODEL\\TEST.O3D\0", b"o3d-bytes");
    let music = synthetic_mtf(b"MUSIC\\TEST.MP2\0", b"music-bytes");
    let data_path = dir.path().join("DATA.MTF");
    let music_path = dir.path().join("MUSIC.MTF");
    let unrelated_path = dir.path().join("notes.txt");
    fs::write(&data_path, &data).unwrap();
    fs::write(&music_path, &music).unwrap();
    fs::write(&unrelated_path, b"leave me alone").unwrap();

    let before_data = fs::read(&data_path).unwrap();
    let before_music = fs::read(&music_path).unwrap();
    let before_unrelated = fs::read(&unrelated_path).unwrap();
    let before_data_mtime = fs::metadata(&data_path).unwrap().modified().unwrap();
    let before_music_mtime = fs::metadata(&music_path).unwrap().modified().unwrap();
    let before_unrelated_mtime = fs::metadata(&unrelated_path).unwrap().modified().unwrap();

    let report = scan_install(dir.path()).unwrap();

    assert_eq!(report.schema_version, 1);
    assert_eq!(report.install_path, dir.path());
    assert_eq!(report.archives.len(), 2);
    assert!(report.archives.iter().all(|item| item.sha256.len() == 64));
    assert_eq!(report.summary.archives_parsed, 2);
    assert_eq!(report.summary.entries_total, 2);
    assert!(
        report
            .assets
            .iter()
            .any(|item| item.internal_path == "MODEL\\TEST.O3D")
    );
    assert!(
        report
            .assets
            .iter()
            .any(|item| item.internal_path == "MUSIC\\TEST.MP2")
    );

    assert_eq!(fs::read(&data_path).unwrap(), before_data);
    assert_eq!(fs::read(&music_path).unwrap(), before_music);
    assert_eq!(fs::read(&unrelated_path).unwrap(), before_unrelated);
    assert_eq!(
        fs::metadata(&data_path).unwrap().modified().unwrap(),
        before_data_mtime
    );
    assert_eq!(
        fs::metadata(&music_path).unwrap().modified().unwrap(),
        before_music_mtime
    );
    assert_eq!(
        fs::metadata(&unrelated_path).unwrap().modified().unwrap(),
        before_unrelated_mtime
    );
}

#[test]
fn missing_optional_archives_do_not_fail_scan() {
    let dir = TestDir::new("optional");
    fs::write(
        dir.path().join("DATA.MTF"),
        synthetic_mtf(b"ONLY\\ONE.BIN\0", b"x"),
    )
    .unwrap();

    let report = scan_install(dir.path()).unwrap();

    assert_eq!(report.summary.archives_parsed, 1);
    assert_eq!(report.summary.entries_total, 1);
    assert_eq!(report.archives.len(), 1);
    assert_eq!(report.archives[0].name, "DATA.MTF");
}

#[test]
fn missing_data_archive_is_an_invalid_install() {
    let dir = TestDir::new("missing-data");
    let err = scan_install(dir.path()).unwrap_err();

    assert!(matches!(err, ScanError::MissingRequiredArchive { .. }));
}

#[test]
fn malformed_required_data_archive_is_a_required_parse_error() {
    let dir = TestDir::new("bad-data");
    fs::write(dir.path().join("DATA.MTF"), b"not-an-mtf").unwrap();

    let err = scan_install(dir.path()).unwrap_err();

    assert!(matches!(err, ScanError::RequiredArchiveParse { .. }));
}

#[test]
fn malformed_optional_archive_is_reported_without_aborting() {
    let dir = TestDir::new("bad-optional");
    fs::write(
        dir.path().join("DATA.MTF"),
        synthetic_mtf(b"GOOD.BIN\0", b"ok"),
    )
    .unwrap();
    fs::write(dir.path().join("MUSIC.MTF"), b"not-an-mtf").unwrap();

    let report = scan_install(dir.path()).unwrap();

    assert_eq!(report.summary.archives_parsed, 1);
    assert_eq!(report.summary.errors, 1);
    assert!(report.assets.iter().any(|item| {
        item.archive_name == "MUSIC.MTF"
            && item.internal_path == "<archive>"
            && item.diagnostic.is_some()
    }));
}
