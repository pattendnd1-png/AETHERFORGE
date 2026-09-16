use forgeclean::pre_rebase::{
    AutoReason, PreRebasePolicy, PreRebaseState, STORAGE_PRESSURE_PERCENT, auto_reason, purge, scan,
};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture(name: &str) -> PathBuf {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "forgeclean-pre-rebase-{name}-{}-{now}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn old_timestamp(path: &Path) {
    let status = Command::new("touch")
        .args(["-a", "-m", "-d", "@1700000000"])
        .arg(path)
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn aggressive_any_timestamp_marks_file_as_candidate() {
    let home = fixture("any-timestamp");
    let documents = home.join("Documents");
    fs::create_dir_all(&documents).unwrap();
    let file = documents.join("copied-after-rebase.bin");
    fs::write(&file, b"data").unwrap();
    let status = Command::new("touch")
        .args(["-a", "-d", "@1700000000"])
        .arg(&file)
        .status()
        .unwrap();
    assert!(status.success());

    let policy = PreRebasePolicy::with_roots(&home, vec![home.clone()]).unwrap();
    let report = scan(&policy);
    assert!(
        report
            .candidates
            .iter()
            .any(|candidate| candidate.path == file)
    );
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn protected_live_state_active_projects_and_symlinks_are_never_candidates() {
    let home = fixture("protected");
    let config = home.join(".config/app/settings.json");
    let active = home.join("Downloads/ForgeClean/Projects/Demo/Active/source.rs");
    let saves = home.join("Games/Demo/saves/save1.dat");
    let repo_file = home.join("Projects/CurrentRepo/src/old.rs");
    let current_source = home
        .join("Downloads")
        .join(format!("ForgeClean-v{}", env!("CARGO_PKG_VERSION")))
        .join("src/lib.rs");
    let documents = home.join("Documents");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::create_dir_all(active.parent().unwrap()).unwrap();
    fs::create_dir_all(saves.parent().unwrap()).unwrap();
    fs::create_dir_all(repo_file.parent().unwrap()).unwrap();
    fs::create_dir_all(current_source.parent().unwrap()).unwrap();
    fs::create_dir_all(home.join("Projects/CurrentRepo/.git")).unwrap();
    fs::create_dir_all(&documents).unwrap();
    fs::write(&config, b"config").unwrap();
    fs::write(&active, b"source").unwrap();
    fs::write(&saves, b"save").unwrap();
    fs::write(&repo_file, b"source").unwrap();
    fs::write(&current_source, b"current-source").unwrap();
    old_timestamp(&config);
    old_timestamp(&active);
    old_timestamp(&saves);
    old_timestamp(&repo_file);
    old_timestamp(&current_source);
    let target = documents.join("target.bin");
    fs::write(&target, b"target").unwrap();
    old_timestamp(&target);
    let link = documents.join("linked.bin");
    symlink(&target, &link).unwrap();

    let policy = PreRebasePolicy::with_roots(&home, vec![home.clone()]).unwrap();
    let report = scan(&policy);
    assert!(
        !report
            .candidates
            .iter()
            .any(|candidate| candidate.path == config)
    );
    assert!(
        !report
            .candidates
            .iter()
            .any(|candidate| candidate.path == active)
    );
    assert!(
        !report
            .candidates
            .iter()
            .any(|candidate| candidate.path == saves)
    );
    assert!(
        !report
            .candidates
            .iter()
            .any(|candidate| candidate.path == repo_file)
    );
    assert!(
        !report
            .candidates
            .iter()
            .any(|candidate| candidate.path == current_source)
    );
    assert!(
        !report
            .candidates
            .iter()
            .any(|candidate| candidate.path == link)
    );
    assert!(
        report
            .candidates
            .iter()
            .any(|candidate| candidate.path == target)
    );
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn purge_revalidates_identity_before_direct_unlink() {
    let home = fixture("revalidate");
    let documents = home.join("Documents");
    fs::create_dir_all(&documents).unwrap();
    let stale = documents.join("stale.bin");
    let changed = documents.join("changed.bin");
    fs::write(&stale, b"stale").unwrap();
    fs::write(&changed, b"old").unwrap();
    old_timestamp(&stale);
    old_timestamp(&changed);

    let policy = PreRebasePolicy::with_roots(&home, vec![home.clone()]).unwrap();
    let report = scan(&policy);
    assert_eq!(report.candidates.len(), 2);
    fs::write(&changed, b"changed identity after scan").unwrap();

    let purge_report = purge(&policy, &report.candidates);
    assert!(!stale.exists());
    assert!(changed.exists());
    assert_eq!(purge_report.deleted_files, 1);
    assert_eq!(purge_report.changed_since_scan, 1);
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn auto_policy_runs_on_first_start_daily_and_storage_pressure() {
    let blank = PreRebaseState::default();
    assert_eq!(
        auto_reason(&blank, 100_000, Some("boot-a"), 65),
        Some(AutoReason::FirstRun)
    );

    let state = PreRebaseState {
        last_run_unix_secs: Some(100_000),
        last_boot_id: Some("boot-a".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        auto_reason(&state, 100_100, Some("boot-b"), 65),
        Some(AutoReason::Startup)
    );
    assert_eq!(
        auto_reason(&state, 186_401, Some("boot-a"), 65),
        Some(AutoReason::Daily)
    );
    assert_eq!(
        auto_reason(&state, 100_100, Some("boot-a"), STORAGE_PRESSURE_PERCENT),
        Some(AutoReason::StoragePressure(STORAGE_PRESSURE_PERCENT))
    );
    assert_eq!(auto_reason(&state, 100_100, Some("boot-a"), 65), None);
}

#[test]
fn purge_refuses_parent_directory_replaced_by_symlink() {
    let home = fixture("parent-symlink-swap");
    let documents = home.join("Documents");
    let original_parent = documents.join("payload");
    fs::create_dir_all(&original_parent).unwrap();
    let original = original_parent.join("old.bin");
    fs::write(&original, b"old").unwrap();
    old_timestamp(&original);

    let policy = PreRebasePolicy::with_roots(&home, vec![home.clone()]).unwrap();
    let report = scan(&policy);
    assert_eq!(report.candidates.len(), 1);

    let moved_parent = documents.join("payload-real");
    fs::rename(&original_parent, &moved_parent).unwrap();
    let attacker_parent = documents.join("attacker");
    fs::create_dir_all(&attacker_parent).unwrap();
    let attacker_file = attacker_parent.join("old.bin");
    fs::write(&attacker_file, b"must-survive").unwrap();
    old_timestamp(&attacker_file);
    symlink(&attacker_parent, &original_parent).unwrap();

    let purge_report = purge(&policy, &report.candidates);
    assert_eq!(purge_report.deleted_files, 0);
    assert_eq!(purge_report.changed_since_scan, 1);
    assert!(attacker_file.exists());
    assert!(moved_parent.join("old.bin").exists());
    fs::remove_dir_all(home).unwrap();
}
