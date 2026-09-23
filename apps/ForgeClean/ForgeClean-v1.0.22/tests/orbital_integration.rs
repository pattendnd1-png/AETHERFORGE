use forgeclean::orbital::{MigrationAction, OrbitalAction, local_storage_summary, parse_status};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_home() -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "forgeclean-orbital-integration-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn status_parser_exposes_orbit_state() {
    let status = parse_status("mode=full\nresult=PASS\nqueue_depth=0\n");
    assert_eq!(status.get("mode"), "full");
    assert_eq!(status.get("result"), "PASS");
}

#[test]
fn orbital_and_migration_actions_are_typed() {
    assert_eq!(OrbitalAction::parse("retry"), Some(OrbitalAction::Retry));
    assert_eq!(
        MigrationAction::parse("verify"),
        Some(MigrationAction::Verify)
    );
    assert_eq!(
        MigrationAction::parse("stage"),
        Some(MigrationAction::Stage)
    );
}

#[test]
fn external_storage_summary_counts_manifests() {
    let home = temp_home();
    let artifacts = home.join("Downloads/AETHERFORGE/artifacts");
    fs::create_dir_all(&artifacts).unwrap();
    fs::write(artifacts.join("INDEX.tsv"), "h\na\nb\n").unwrap();
    fs::write(artifacts.join("LARGE-FILES.tsv"), "h\na\n").unwrap();
    fs::write(artifacts.join("restore-large-assets.sh"), "#!/bin/sh\n").unwrap();
    let status = local_storage_summary(&home);
    assert_eq!(status.indexed_artifacts, 2);
    assert_eq!(status.externalized_large_files, 1);
    assert!(status.restore_helper_present);
    let _ = fs::remove_dir_all(home);
}
