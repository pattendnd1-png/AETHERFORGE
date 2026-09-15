use sanctuary_battlenet::{
    BridgeDiscovery, OFFICIAL_DOWNLOAD_URL, discover_with_environment, find_diablo_candidates,
};
use std::{ffi::OsStr, fs, path::PathBuf};

fn make_windows_tree(prefix: &std::path::Path) -> (PathBuf, PathBuf) {
    let launcher = prefix.join("drive_c/Program Files (x86)/Battle.net/Battle.net Launcher.exe");
    let game = prefix.join("drive_c/Program Files (x86)/Diablo III");
    fs::create_dir_all(launcher.parent().unwrap()).unwrap();
    fs::write(&launcher, b"fixture").unwrap();
    fs::create_dir_all(game.join("Data")).unwrap();
    fs::write(game.join(".build.info"), b"fixture").unwrap();
    fs::write(game.join("Diablo III.exe"), b"fixture").unwrap();
    (launcher, game)
}

#[test]
fn discovers_battlenet_and_diablo_from_default_wine_prefix() {
    let home = tempfile::tempdir().unwrap();
    let prefix = home.path().join(".wine");
    let (launcher, game) = make_windows_tree(&prefix);
    let bin = home.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    let wine = bin.join("wine");
    fs::write(&wine, b"fixture").unwrap();

    let discovery = discover_with_environment(home.path(), Some(bin.as_os_str()), None, None, None);

    assert_eq!(discovery.launcher_exe.as_deref(), Some(launcher.as_path()));
    assert_eq!(discovery.wine_prefix.as_deref(), Some(prefix.as_path()));
    assert_eq!(discovery.runner.as_deref(), Some(wine.as_path()));
    assert_eq!(discovery.diablo_install.as_deref(), Some(game.as_path()));
    assert!(
        discovery
            .diablo_exe
            .as_ref()
            .is_some_and(|p| p.ends_with("Diablo III.exe"))
    );
}

#[test]
fn environment_overrides_take_priority() {
    let home = tempfile::tempdir().unwrap();
    let prefix = home.path().join("custom-prefix");
    let (launcher, _) = make_windows_tree(&prefix);
    let runner = home.path().join("custom-wine");
    fs::write(&runner, b"fixture").unwrap();

    let discovery = discover_with_environment(
        home.path(),
        Some(OsStr::new("")),
        Some(launcher.as_os_str()),
        Some(runner.as_os_str()),
        Some(prefix.as_os_str()),
    );

    assert_eq!(discovery.launcher_exe.as_deref(), Some(launcher.as_path()));
    assert_eq!(discovery.runner.as_deref(), Some(runner.as_path()));
    assert_eq!(discovery.wine_prefix.as_deref(), Some(prefix.as_path()));
}

#[test]
fn candidate_search_is_bounded_to_known_roots() {
    let home = tempfile::tempdir().unwrap();
    let prefix = home.path().join(".wine");
    let (_, game) = make_windows_tree(&prefix);
    let discovery = BridgeDiscovery {
        launcher_exe: None,
        runner: None,
        wine_prefix: Some(prefix),
        diablo_install: Some(game.clone()),
        diablo_exe: Some(game.join("Diablo III.exe")),
    };

    let candidates = find_diablo_candidates(home.path(), &discovery);
    assert!(candidates.contains(&game));
    assert!(candidates.len() < 20);
}

#[test]
fn official_download_url_is_https_battlenet() {
    assert_eq!(
        OFFICIAL_DOWNLOAD_URL,
        "https://download.battle.net/en-us/desktop"
    );
}

#[test]
fn bridge_record_round_trips_and_validates() {
    use sanctuary_battlenet::{BridgeRecord, load_bridge_record, save_bridge_record};

    let home = tempfile::tempdir().unwrap();
    let prefix = home.path().join(".wine");
    let (launcher, game) = make_windows_tree(&prefix);
    let runner = home.path().join("bin/wine");
    fs::create_dir_all(runner.parent().unwrap()).unwrap();
    fs::write(&runner, b"fixture").unwrap();
    let discovery = BridgeDiscovery {
        launcher_exe: Some(launcher),
        runner: Some(runner),
        wine_prefix: Some(prefix),
        diablo_install: Some(game.clone()),
        diablo_exe: Some(game.join("Diablo III.exe")),
    };
    let record = BridgeRecord::from_discovery(&discovery).expect("complete discovery");
    let path = home.path().join("config/battlenet.toml");

    save_bridge_record(&path, &record).unwrap();
    let loaded = load_bridge_record(&path).unwrap().expect("saved record");

    assert_eq!(loaded, record);
    assert_eq!(loaded.validated().unwrap(), discovery);
}

#[test]
fn clear_bridge_record_removes_only_saved_record() {
    use sanctuary_battlenet::{
        BridgeRecord, clear_bridge_record, load_bridge_record, save_bridge_record,
    };

    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("config/battlenet.toml");
    let record = BridgeRecord {
        schema_version: 1,
        launcher_exe: PathBuf::from("/tmp/Battle.net.exe"),
        runner: PathBuf::from("/tmp/wine"),
        wine_prefix: PathBuf::from("/tmp/prefix"),
        diablo_install: Some(PathBuf::from("/tmp/Diablo III")),
        diablo_exe: Some(PathBuf::from("/tmp/Diablo III/Diablo III.exe")),
    };
    save_bridge_record(&path, &record).unwrap();
    assert!(load_bridge_record(&path).unwrap().is_some());

    clear_bridge_record(&path).unwrap();
    assert!(load_bridge_record(&path).unwrap().is_none());
    clear_bridge_record(&path).unwrap();
}

#[test]
fn missing_bridge_record_is_not_an_error() {
    use sanctuary_battlenet::load_bridge_record;

    let home = tempfile::tempdir().unwrap();
    assert!(
        load_bridge_record(&home.path().join("missing.toml"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn stale_bridge_record_is_rejected() {
    use sanctuary_battlenet::BridgeRecord;

    let record = BridgeRecord {
        schema_version: 1,
        launcher_exe: PathBuf::from("/missing/Battle.net.exe"),
        runner: PathBuf::from("/missing/wine"),
        wine_prefix: PathBuf::from("/missing/prefix"),
        diablo_install: Some(PathBuf::from("/missing/Diablo III")),
        diablo_exe: Some(PathBuf::from("/missing/Diablo III.exe")),
    };

    assert!(record.validated().is_err());
}

#[test]
fn bridge_record_path_honors_xdg_config_home() {
    use sanctuary_battlenet::bridge_record_path;

    let home = PathBuf::from("/home/tester");
    assert_eq!(
        bridge_record_path(&home, Some(OsStr::new("/tmp/config"))),
        PathBuf::from("/tmp/config/opensanctuary/battlenet.toml")
    );
    assert_eq!(
        bridge_record_path(&home, None),
        PathBuf::from("/home/tester/.config/opensanctuary/battlenet.toml")
    );
}

#[test]
fn retry_policy_defaults_to_three_attempts() {
    use sanctuary_battlenet::RetryPolicy;

    let policy = RetryPolicy::default();
    assert_eq!(policy.attempts(), 3);
}

#[test]
fn saved_custom_prefix_is_rescanned_when_game_path_moves() {
    use sanctuary_battlenet::{BridgeRecord, discover_for_home_with_record};

    let home = tempfile::tempdir().unwrap();
    let prefix = home.path().join("custom-prefix");
    let launcher = prefix.join("drive_c/Program Files (x86)/Battle.net/Battle.net Launcher.exe");
    fs::create_dir_all(launcher.parent().unwrap()).unwrap();
    fs::write(&launcher, b"fixture").unwrap();
    let moved_game = prefix.join("drive_c/Program Files/Diablo III");
    fs::create_dir_all(moved_game.join("Data")).unwrap();
    fs::write(moved_game.join(".build.info"), b"fixture").unwrap();
    fs::write(moved_game.join("Diablo III.exe"), b"fixture").unwrap();
    let runner = home.path().join("runner/wine");
    fs::create_dir_all(runner.parent().unwrap()).unwrap();
    fs::write(&runner, b"fixture").unwrap();
    let record = BridgeRecord {
        schema_version: 1,
        launcher_exe: launcher,
        runner,
        wine_prefix: prefix,
        diablo_install: Some(PathBuf::from("/old/Diablo III")),
        diablo_exe: Some(PathBuf::from("/old/Diablo III.exe")),
    };

    let discovery = discover_for_home_with_record(home.path(), Some(&record));
    assert_eq!(
        discovery.diablo_install.as_deref(),
        Some(moved_game.as_path())
    );
    assert!(discovery.official_game_ready());
}

#[test]
fn client_only_bridge_record_round_trips_and_validates() {
    use sanctuary_battlenet::{BridgeRecord, load_bridge_record, save_bridge_record};

    let home = tempfile::tempdir().unwrap();
    let prefix = home.path().join("custom-prefix");
    let launcher = prefix.join("drive_c/Program Files (x86)/Battle.net/Battle.net Launcher.exe");
    fs::create_dir_all(launcher.parent().unwrap()).unwrap();
    fs::write(&launcher, b"fixture").unwrap();
    let runner = home.path().join("bin/wine");
    fs::create_dir_all(runner.parent().unwrap()).unwrap();
    fs::write(&runner, b"fixture").unwrap();
    let discovery = BridgeDiscovery {
        launcher_exe: Some(launcher.clone()),
        runner: Some(runner.clone()),
        wine_prefix: Some(prefix.clone()),
        diablo_install: None,
        diablo_exe: None,
    };

    let record = BridgeRecord::from_discovery(&discovery).expect("client bridge record");
    let path = home.path().join("config/battlenet.toml");
    save_bridge_record(&path, &record).unwrap();
    let loaded = load_bridge_record(&path).unwrap().expect("saved record");
    let validated = loaded.validated().expect("client bridge validates");

    assert_eq!(validated.launcher_exe.as_deref(), Some(launcher.as_path()));
    assert_eq!(validated.runner.as_deref(), Some(runner.as_path()));
    assert_eq!(validated.wine_prefix.as_deref(), Some(prefix.as_path()));
    assert!(validated.diablo_install.is_none());
    assert!(validated.diablo_exe.is_none());
}
