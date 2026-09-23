use forgeclean::gui_state::{
    CoreAction, GuiSettings, collect_project_snapshots, load_settings_from, run_core_action,
    save_settings_to,
};
use forgeclean::organizer::ForgeLayout;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_root(label: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "forgeclean-gui-{label}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn gui_settings_round_trip_preserves_runtime_controls() {
    let root = temp_root("settings");
    let path = root.join("gui-settings.json");
    let settings = GuiSettings {
        stable_seconds: 45,
        poll_seconds: 3,
        coldpack_maintenance_hours: 12,
        logging_level: "verbose".to_owned(),
        ..GuiSettings::default()
    };
    save_settings_to(&path, &settings).unwrap();
    let loaded = load_settings_from(&path).unwrap();
    assert_eq!(loaded, settings);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_snapshot_groups_versions_and_build_files() {
    let root = temp_root("projects");
    let downloads = root.join("Downloads");
    let layout = ForgeLayout::new(&downloads);
    layout.ensure().unwrap();
    let v1 = layout.project_build("ForgeHX", "10.0.30");
    let v2 = layout.project_build("ForgeHX", "10.0.31");
    fs::create_dir_all(&v1).unwrap();
    fs::create_dir_all(&v2).unwrap();
    fs::write(v1.join("ForgeHX-10.0.30-SOURCE.zip"), b"source").unwrap();
    fs::write(v1.join("ForgeHX-10.0.30-HIT-IT.sh"), b"#!/bin/sh\n").unwrap();
    fs::write(v2.join("ForgeHX-10.0.31-VERIFY.txt"), b"PASS\n").unwrap();

    let projects = collect_project_snapshots(&layout).unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "ForgeHX");
    assert_eq!(projects[0].builds.len(), 2);
    assert_eq!(projects[0].builds[0].version, "10.0.31");
    assert_eq!(projects[0].builds[1].files.len(), 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn relocated_build_artifact_runs_from_its_canonical_build_folder() {
    let root = temp_root("artifact-cwd");
    let downloads = root.join("Downloads");
    let build = downloads.join("ForgeClean/Projects/Demo/Builds/1.2.3");
    fs::create_dir_all(&build).unwrap();
    let script = build.join("Demo-v1.2.3-HIT-IT.sh");
    fs::write(
        &script,
        b"#!/usr/bin/bash\nprintf '%s\\n' \"$PWD\" > artifact-pwd.txt\n",
    )
    .unwrap();

    let result = run_core_action(CoreAction::RunArtifact(script), &downloads);
    assert!(result.success, "{}", result.output);
    let observed = fs::read_to_string(build.join("artifact-pwd.txt")).unwrap();
    assert_eq!(observed.trim(), build.to_string_lossy());
    fs::remove_dir_all(root).unwrap();
}
