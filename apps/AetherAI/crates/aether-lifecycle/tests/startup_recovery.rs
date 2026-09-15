use aether_lifecycle::{
    FileDigest, FsLifecycleManager, LifecycleManager, ReleaseInfo, ReleaseManifest,
    StartupRecovery, sha256_file,
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

fn create_release(parent: &Path, version: &str) -> PathBuf {
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
        architecture: std::env::consts::ARCH.into(),
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
    let hash = sha256_file(&root.join("release.json")).unwrap();
    std::fs::write(root.join("release.json.sha256"), format!("{hash}\n")).unwrap();
    root
}

fn info(root: PathBuf, version: &str) -> ReleaseInfo {
    ReleaseInfo {
        version: version.into(),
        architecture: std::env::consts::ARCH.into(),
        source: root,
    }
}

fn install(manager: &FsLifecycleManager, sources: &Path, version: &str) {
    let source = create_release(sources, version);
    let staged = manager.stage_update(&info(source, version)).unwrap();
    manager.activate(&staged).unwrap();
}

#[test]
fn second_verified_activation_is_pending_until_startup_health_marks_it_green() {
    let temp = tempfile::tempdir().unwrap();
    let manager = FsLifecycleManager::new(temp.path().join("install"), std::env::consts::ARCH);
    let sources = temp.path().join("sources");

    install(&manager, &sources, "0.3.6");
    assert!(manager.pending_startup_version().unwrap().is_none());

    install(&manager, &sources, "0.3.7");
    assert_eq!(
        manager.pending_startup_version().unwrap().as_deref(),
        Some("0.3.7")
    );

    assert_eq!(
        manager.mark_startup_healthy().unwrap().as_deref(),
        Some("0.3.7")
    );
    assert!(manager.pending_startup_version().unwrap().is_none());
    assert_eq!(manager.current_version().unwrap().as_deref(), Some("0.3.7"));
}

#[test]
fn failed_pending_startup_rolls_back_binary_pointer_and_preserves_user_data() {
    let temp = tempfile::tempdir().unwrap();
    let manager = FsLifecycleManager::new(temp.path().join("install"), std::env::consts::ARCH);
    let sources = temp.path().join("sources");
    let user_data = temp.path().join("user-data");
    std::fs::create_dir_all(&user_data).unwrap();
    let database = user_data.join("aetherai.db");
    std::fs::write(&database, b"USER-DATA-MUST-NOT-CHANGE").unwrap();

    install(&manager, &sources, "0.3.6");
    install(&manager, &sources, "0.3.7");

    let recovery = manager
        .recover_pending_startup_failure("database migration failed")
        .unwrap()
        .unwrap();

    match recovery {
        StartupRecovery::RolledBack {
            failed_version,
            restored_version,
            binary,
            ..
        } => {
            assert_eq!(failed_version, "0.3.7");
            assert_eq!(restored_version, "0.3.6");
            assert!(binary.is_file());
        }
    }

    assert_eq!(manager.current_version().unwrap().as_deref(), Some("0.3.6"));
    assert!(manager.pending_startup_version().unwrap().is_none());
    assert_eq!(
        std::fs::read(&database).unwrap(),
        b"USER-DATA-MUST-NOT-CHANGE"
    );
}

#[test]
fn manual_rollback_clears_pending_startup_marker() {
    let temp = tempfile::tempdir().unwrap();
    let manager = FsLifecycleManager::new(temp.path().join("install"), std::env::consts::ARCH);
    let sources = temp.path().join("sources");

    install(&manager, &sources, "0.3.6");
    install(&manager, &sources, "0.3.7");
    assert!(manager.pending_startup_version().unwrap().is_some());

    manager.rollback().unwrap();

    assert!(manager.pending_startup_version().unwrap().is_none());
    assert_eq!(manager.current_version().unwrap().as_deref(), Some("0.3.6"));
}
