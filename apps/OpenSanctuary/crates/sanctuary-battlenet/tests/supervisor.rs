use sanctuary_battlenet::{
    SessionSupervisor, SupervisorState, build_fingerprint, snapshot_processes,
};
use std::fs;
use tempfile::tempdir;

fn write_process(proc_root: &std::path::Path, pid: u32, comm: &str, cmdline: &[u8]) {
    let dir = proc_root.join(pid.to_string());
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("comm"), format!("{comm}\n")).unwrap();
    fs::write(dir.join("cmdline"), cmdline).unwrap();
}

fn create_install(root: &std::path::Path, build: &str) -> std::path::PathBuf {
    let install = root.join("Diablo III");
    fs::create_dir_all(install.join("Data")).unwrap();
    fs::write(install.join(".build.info"), build).unwrap();
    install
}

#[test]
fn process_snapshot_classifies_battlenet_agent_and_diablo() {
    let temp = tempdir().unwrap();
    let proc_root = temp.path().join("proc");
    fs::create_dir_all(&proc_root).unwrap();

    write_process(
        &proc_root,
        101,
        "wine64-preloader",
        b"wine\0C:\\Program Files (x86)\\Battle.net\\Battle.net.exe\0",
    );
    write_process(
        &proc_root,
        102,
        "Agent.exe",
        b"C:\\ProgramData\\Battle.net\\Agent\\Agent.exe\0",
    );
    write_process(
        &proc_root,
        103,
        "Diablo III.exe",
        b"C:\\Program Files (x86)\\Diablo III\\Diablo III.exe\0",
    );

    let snapshot = snapshot_processes(&proc_root);
    assert!(snapshot.battlenet_running);
    assert!(snapshot.agent_running);
    assert!(snapshot.diablo_running);
}

#[test]
fn unrelated_agent_process_is_not_battlenet_agent() {
    let temp = tempdir().unwrap();
    let proc_root = temp.path().join("proc");
    fs::create_dir_all(&proc_root).unwrap();
    write_process(&proc_root, 104, "Agent.exe", b"/opt/vendor/Agent.exe\0");

    let snapshot = snapshot_processes(&proc_root);
    assert!(!snapshot.agent_running);
}

#[test]
fn agent_process_does_not_imply_visible_battlenet_client() {
    let temp = tempdir().unwrap();
    let proc_root = temp.path().join("proc");
    fs::create_dir_all(&proc_root).unwrap();
    write_process(
        &proc_root,
        105,
        "Agent.exe",
        b"C:\\ProgramData\\Battle.net\\Agent\\Agent.exe\0",
    );

    let snapshot = snapshot_processes(&proc_root);
    assert!(snapshot.agent_running);
    assert!(!snapshot.battlenet_running);
}

#[test]
fn build_fingerprint_changes_when_storage_metadata_changes() {
    let temp = tempdir().unwrap();
    let install = create_install(temp.path(), "Version!STRING: 1\n");
    let data = install.join("Data/data");
    fs::create_dir_all(&data).unwrap();
    fs::write(data.join("data.000"), b"one").unwrap();
    let before = build_fingerprint(&install).unwrap();
    fs::write(data.join("data.000"), b"payload changed").unwrap();
    let after = build_fingerprint(&install).unwrap();
    assert_ne!(before, after);
}

#[test]
fn build_fingerprint_changes_when_build_info_changes() {
    let temp = tempdir().unwrap();
    let install = create_install(temp.path(), "Version!STRING: 1\n");
    let before = build_fingerprint(&install).unwrap();
    fs::write(install.join(".build.info"), "Version!STRING: 2\n").unwrap();
    let after = build_fingerprint(&install).unwrap();
    assert_ne!(before, after);
}

#[test]
fn supervisor_settles_update_after_two_stable_samples() {
    let temp = tempdir().unwrap();
    let proc_root = temp.path().join("proc");
    fs::create_dir_all(&proc_root).unwrap();
    let install = create_install(temp.path(), "Version!STRING: 1\n");
    let mut supervisor = SessionSupervisor::new(2);

    let initial = supervisor.observe(&proc_root, Some(&install));
    assert_eq!(initial.state, SupervisorState::Ready);
    assert!(!initial.update_settled);

    write_process(
        &proc_root,
        102,
        "Agent.exe",
        b"C:\\ProgramData\\Battle.net\\Agent\\Agent.exe\0",
    );
    fs::write(install.join(".build.info"), "Version!STRING: 2\n").unwrap();

    let changing = supervisor.observe(&proc_root, Some(&install));
    assert_eq!(changing.state, SupervisorState::Updating);
    assert!(!changing.update_settled);

    let stable_once = supervisor.observe(&proc_root, Some(&install));
    assert_eq!(stable_once.state, SupervisorState::Updating);
    assert!(!stable_once.update_settled);

    let settled = supervisor.observe(&proc_root, Some(&install));
    assert_eq!(settled.state, SupervisorState::Ready);
    assert!(settled.update_settled);
}
