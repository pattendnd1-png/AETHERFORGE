use aether_lifecycle::{
    FileDigest, FsLifecycleManager, LifecycleManager, ReleaseInfo, ReleaseManifest, UpdateStatus,
    sha256_file,
};
use std::path::{Path, PathBuf};

#[cfg(unix)]
fn make_binary(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(
        path,
        "#!/bin/sh\necho AETHERAI_DESKTOP=LIVE\necho AETHERAI_NORMAL_USER_TERMINAL_REQUIRED=NO\nexit 0\n",
    )
    .unwrap();
    let mut permissions = std::fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(path, permissions).unwrap();
}

#[cfg(not(unix))]
fn make_binary(path: &Path) {
    std::fs::write(path, "test binary").unwrap();
}

fn create_release(parent: &Path, version: &str, arch: &str) -> PathBuf {
    let root = parent.join(version);
    std::fs::create_dir_all(root.join("bin")).unwrap();
    let binary_rel = if cfg!(windows) {
        "bin/aetherai-desktop.exe"
    } else {
        "bin/aetherai-desktop"
    };
    let binary = root.join(binary_rel);
    make_binary(&binary);

    let manifest = ReleaseManifest {
        schema: 1,
        version: version.into(),
        architecture: arch.into(),
        binary: binary_rel.into(),
        browser_webview: false,
        openai_api_required: false,
        paid_service_required: false,
        files: vec![FileDigest {
            path: binary_rel.into(),
            sha256: sha256_file(&binary).unwrap(),
        }],
    };
    std::fs::write(
        root.join("release.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let manifest_hash = sha256_file(&root.join("release.json")).unwrap();
    std::fs::write(
        root.join("release.json.sha256"),
        format!("{manifest_hash}\n"),
    )
    .unwrap();
    root
}

fn release_info(path: &Path, version: &str, arch: &str) -> ReleaseInfo {
    ReleaseInfo {
        version: version.into(),
        architecture: arch.into(),
        source: path.to_path_buf(),
    }
}

#[test]
fn activating_verified_release_moves_current_and_previous_atomically() {
    let temp = tempfile::tempdir().unwrap();
    let manager = FsLifecycleManager::new(temp.path().join("install"), "x86_64");
    let sources = temp.path().join("sources");
    let r036 = create_release(&sources, "0.3.6", "x86_64");
    let r037 = create_release(&sources, "0.3.7", "x86_64");

    let s036 = manager
        .stage_update(&release_info(&r036, "0.3.6", "x86_64"))
        .unwrap();
    manager.activate(&s036).unwrap();

    let s037 = manager
        .stage_update(&release_info(&r037, "0.3.7", "x86_64"))
        .unwrap();
    manager.activate(&s037).unwrap();

    assert_eq!(manager.current_version().unwrap().as_deref(), Some("0.3.7"));
    assert_eq!(
        manager.previous_version().unwrap().as_deref(),
        Some("0.3.6")
    );
}

#[test]
fn tampered_release_never_becomes_current() {
    let temp = tempfile::tempdir().unwrap();
    let manager = FsLifecycleManager::new(temp.path().join("install"), "x86_64");
    let sources = temp.path().join("sources");
    let r036 = create_release(&sources, "0.3.6", "x86_64");
    let r037 = create_release(&sources, "0.3.7", "x86_64");

    let s036 = manager
        .stage_update(&release_info(&r036, "0.3.6", "x86_64"))
        .unwrap();
    manager.activate(&s036).unwrap();

    let s037 = manager
        .stage_update(&release_info(&r037, "0.3.7", "x86_64"))
        .unwrap();
    std::fs::write(s037.root.join(&s037.manifest.binary), "tampered").unwrap();

    assert!(manager.verify_staged(&s037).is_err());
    assert!(manager.activate(&s037).is_err());
    assert_eq!(manager.current_version().unwrap().as_deref(), Some("0.3.6"));
}

#[test]
fn rollback_swaps_current_and_previous_only_after_verification() {
    let temp = tempfile::tempdir().unwrap();
    let manager = FsLifecycleManager::new(temp.path().join("install"), "x86_64");
    let sources = temp.path().join("sources");

    for version in ["0.3.6", "0.3.7"] {
        let release = create_release(&sources, version, "x86_64");
        let staged = manager
            .stage_update(&release_info(&release, version, "x86_64"))
            .unwrap();
        manager.activate(&staged).unwrap();
    }

    manager.rollback().unwrap();

    assert_eq!(manager.current_version().unwrap().as_deref(), Some("0.3.6"));
    assert_eq!(
        manager.previous_version().unwrap().as_deref(),
        Some("0.3.7")
    );
}

#[test]
fn local_incoming_newer_release_is_discovered_without_network() {
    let temp = tempfile::tempdir().unwrap();
    let manager = FsLifecycleManager::new(temp.path().join("install"), "x86_64");
    let sources = temp.path().join("sources");
    let r036 = create_release(&sources, "0.3.6", "x86_64");

    let s036 = manager
        .stage_update(&release_info(&r036, "0.3.6", "x86_64"))
        .unwrap();
    manager.activate(&s036).unwrap();

    std::fs::create_dir_all(manager.incoming_dir()).unwrap();
    create_release(&manager.incoming_dir(), "0.3.7", "x86_64");

    match manager.check_for_update().unwrap() {
        UpdateStatus::Available(release) => assert_eq!(release.version, "0.3.7"),
        other => panic!("expected available update, got {other:?}"),
    }
}
