use sanctuary_battlenet::{
    BridgeDiscovery, BridgeHealthState, BridgeRecord, FailureTracker, evaluate_bridge_health,
    quarantine_bridge_record, recover_bridge,
};
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

fn make_bridge(root: &std::path::Path) -> (BridgeDiscovery, BridgeRecord) {
    let prefix = root.join("prefix");
    let launcher = prefix.join("drive_c/Program Files (x86)/Battle.net/Battle.net Launcher.exe");
    let game = prefix.join("drive_c/Program Files (x86)/Diablo III");
    let exe = game.join("Diablo III.exe");
    let runner = root.join("bin/wine");
    fs::create_dir_all(launcher.parent().unwrap()).unwrap();
    fs::create_dir_all(game.join("Data")).unwrap();
    fs::create_dir_all(runner.parent().unwrap()).unwrap();
    fs::write(&launcher, b"fixture").unwrap();
    fs::write(game.join(".build.info"), b"fixture").unwrap();
    fs::write(&exe, b"fixture").unwrap();
    fs::write(&runner, b"fixture").unwrap();
    let discovery = BridgeDiscovery {
        launcher_exe: Some(launcher.clone()),
        runner: Some(runner.clone()),
        wine_prefix: Some(prefix.clone()),
        diablo_install: Some(game.clone()),
        diablo_exe: Some(exe.clone()),
    };
    let record = BridgeRecord {
        schema_version: 1,
        launcher_exe: launcher,
        runner,
        wine_prefix: prefix,
        diablo_install: Some(game),
        diablo_exe: Some(exe),
    };
    (discovery, record)
}

#[test]
fn complete_bridge_reports_healthy() {
    let temp = tempfile::tempdir().unwrap();
    let (discovery, _) = make_bridge(temp.path());
    let report = evaluate_bridge_health(&discovery);
    assert_eq!(report.state, BridgeHealthState::Healthy);
    assert!(report.runner_ready);
    assert!(report.prefix_ready);
    assert!(report.launcher_ready);
    assert!(report.diablo_install_ready);
    assert!(report.diablo_exe_ready);
}

#[test]
fn missing_game_with_valid_client_reports_degraded() {
    let temp = tempfile::tempdir().unwrap();
    let (mut discovery, _) = make_bridge(temp.path());
    discovery.diablo_install = Some(PathBuf::from("/missing/Diablo III"));
    discovery.diablo_exe = None;
    let report = evaluate_bridge_health(&discovery);
    assert_eq!(report.state, BridgeHealthState::Degraded);
    assert!(report.runner_ready);
    assert!(report.prefix_ready);
    assert!(report.launcher_ready);
    assert!(!report.diablo_install_ready);
}

#[test]
fn missing_runner_reports_broken() {
    let temp = tempfile::tempdir().unwrap();
    let (mut discovery, _) = make_bridge(temp.path());
    discovery.runner = Some(PathBuf::from("/missing/wine"));
    let report = evaluate_bridge_health(&discovery);
    assert_eq!(report.state, BridgeHealthState::Broken);
    assert!(!report.runner_ready);
}

#[test]
fn stale_record_recovers_moved_game_inside_saved_prefix() {
    let temp = tempfile::tempdir().unwrap();
    let (_, mut record) = make_bridge(temp.path());
    let moved = record.wine_prefix.join("drive_c/Program Files/Diablo III");
    fs::create_dir_all(moved.join("Data")).unwrap();
    fs::write(moved.join(".build.info"), b"moved").unwrap();
    fs::write(moved.join("Diablo III.exe"), b"fixture").unwrap();
    let original_install = record.diablo_install.as_ref().expect("fixture install");
    fs::remove_dir_all(original_install).unwrap();
    record.diablo_install = Some(PathBuf::from("/old/Diablo III"));
    record.diablo_exe = Some(PathBuf::from("/old/Diablo III.exe"));

    let outcome = recover_bridge(temp.path(), Some(&record));
    assert!(outcome.recovered);
    assert_eq!(outcome.report.state, BridgeHealthState::Healthy);
    assert_eq!(
        outcome.discovery.diablo_install.as_deref(),
        Some(moved.as_path())
    );
    assert!(outcome.record.is_some());
}

#[test]
fn corrupt_record_is_quarantined_without_touching_neighbor_files() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("battlenet.toml");
    let neighbor = temp.path().join("keep-me");
    fs::write(&path, b"not = [valid toml").unwrap();
    fs::write(&neighbor, b"safe").unwrap();

    let quarantined = quarantine_bridge_record(&path)
        .unwrap()
        .expect("quarantine path");
    assert!(!path.exists());
    assert!(quarantined.exists());
    assert_eq!(fs::read(&neighbor).unwrap(), b"safe");
}

#[test]
fn three_failures_enter_cooldown_and_success_resets_tracker() {
    let base = Instant::now();
    let mut tracker = FailureTracker::default();
    assert!(tracker.can_attempt(base));
    tracker.record_failure(base);
    tracker.record_failure(base + Duration::from_secs(1));
    assert!(tracker.can_attempt(base + Duration::from_secs(2)));
    tracker.record_failure(base + Duration::from_secs(2));
    assert!(!tracker.can_attempt(base + Duration::from_secs(3)));
    assert!(
        tracker
            .cooldown_remaining(base + Duration::from_secs(3))
            .is_some()
    );
    assert!(tracker.can_attempt(base + Duration::from_secs(33)));
    tracker.record_failure(base + Duration::from_secs(34));
    tracker.record_success();
    assert!(tracker.can_attempt(base + Duration::from_secs(34)));
    assert_eq!(tracker.recent_failures(), 0);
}

#[test]
fn repair_succeeds_for_usable_client_without_diablo_install() {
    let temp = tempfile::tempdir().unwrap();
    let prefix = temp.path().join("Games/battlenet");
    let launcher = prefix.join("drive_c/Program Files (x86)/Battle.net/Battle.net Launcher.exe");
    fs::create_dir_all(launcher.parent().unwrap()).unwrap();
    fs::write(&launcher, b"fixture").unwrap();
    let runner = temp.path().join("bin/wine");
    fs::create_dir_all(runner.parent().unwrap()).unwrap();
    fs::write(&runner, b"fixture").unwrap();

    let saved = BridgeRecord {
        schema_version: 1,
        launcher_exe: launcher,
        runner,
        wine_prefix: prefix,
        diablo_install: Some(PathBuf::from("/missing/Diablo III")),
        diablo_exe: Some(PathBuf::from("/missing/Diablo III.exe")),
    };

    let outcome = recover_bridge(temp.path(), Some(&saved));
    assert!(outcome.recovered);
    assert_eq!(outcome.report.state, BridgeHealthState::Degraded);
    assert!(outcome.record.is_some());
    assert!(outcome.discovery.battlenet_ready());
    assert!(!outcome.discovery.official_game_ready());
}
