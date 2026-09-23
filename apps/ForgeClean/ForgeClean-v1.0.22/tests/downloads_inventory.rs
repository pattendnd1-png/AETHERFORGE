use forgeclean::downloads_inventory::{scan_downloads_inventory, scan_downloads_inventory_summary};
use std::fs;
use std::os::unix::fs::symlink;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_root(label: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "forgeclean-downloads-inventory-{label}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn recursively_inventories_hidden_unknown_generated_and_symlink_paths() {
    let root = temp_root("everything");
    let downloads = root.join("Downloads");
    fs::create_dir_all(downloads.join("misc/nested")).unwrap();
    fs::write(downloads.join(".hidden-unknown.zzz"), b"hidden").unwrap();
    fs::write(downloads.join("misc/nested/blob.odd"), b"blob").unwrap();
    symlink("misc/nested/blob.odd", downloads.join("blob-link")).unwrap();

    let report = scan_downloads_inventory(&downloads).unwrap();
    assert!(
        report
            .entries
            .iter()
            .any(|entry| entry.relative_path == std::path::Path::new(".hidden-unknown.zzz"))
    );
    assert!(
        report
            .entries
            .iter()
            .any(|entry| entry.relative_path == std::path::Path::new("misc/nested/blob.odd"))
    );
    assert!(report.entries.iter().any(|entry| entry.relative_path
        == std::path::Path::new("blob-link")
        && entry.kind == "SYMLINK"));
    assert_eq!(report.project_files.file_paths, 0);
    assert!(report.general.file_paths >= 2);
    assert!(report.scan_threads >= 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_files_are_separate_and_generated_content_is_visible() {
    let root = temp_root("projects");
    let downloads = root.join("Downloads");
    let canonical = downloads.join("ForgeClean/Projects/OpenDeck/Active");
    fs::create_dir_all(canonical.join("target/release")).unwrap();
    fs::write(
        canonical.join("Cargo.toml"),
        b"[package]\nname='opendeck'\n",
    )
    .unwrap();
    fs::write(canonical.join("src.rs"), b"fn main() {}\n").unwrap();
    fs::write(canonical.join("target/release/opendeck"), b"binary").unwrap();
    fs::write(downloads.join("vacation-photo.jpg"), b"personal").unwrap();

    let report = scan_downloads_inventory(&downloads).unwrap();
    assert!(report.project_files.file_paths >= 3);
    assert!(report.general.file_paths >= 1);
    let generated = report
        .entries
        .iter()
        .find(|entry| entry.relative_path.ends_with("target/release/opendeck"))
        .unwrap();
    assert_eq!(generated.project.as_deref(), Some("OpenDeck"));
    assert_eq!(generated.project_class.as_deref(), Some("GENERATED"));
    assert!(
        report
            .projects
            .iter()
            .any(|project| project.name == "OpenDeck")
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn collapses_loose_artifacts_by_known_project_name_and_leaves_unrelated_downloads_general() {
    let root = temp_root("family-collapse");
    let downloads = root.join("Downloads");
    let source = downloads.join("AetherForge");
    fs::create_dir_all(source.join("src")).unwrap();
    fs::write(source.join("Cargo.toml"), b"[workspace]\n").unwrap();
    fs::write(source.join("src/lib.rs"), b"pub fn x() {}\n").unwrap();
    fs::write(
        downloads.join("AetherForge-DOWNLOADS40-THEME-DIAGNOSTICS.txt"),
        b"diag",
    )
    .unwrap();
    fs::write(
        downloads.join("HIT-IT-AETHERFORGE-OBS-GUARD.sh"),
        b"#!/bin/sh\n",
    )
    .unwrap();
    fs::write(downloads.join("Battle.net-Setup.exe"), b"installer").unwrap();

    let report = scan_downloads_inventory(&downloads).unwrap();
    let diag = report
        .entries
        .iter()
        .find(|entry| {
            entry.relative_path
                == std::path::Path::new("AetherForge-DOWNLOADS40-THEME-DIAGNOSTICS.txt")
        })
        .unwrap();
    assert_eq!(diag.project.as_deref(), Some("AetherForge"));
    let hit = report
        .entries
        .iter()
        .find(|entry| {
            entry.relative_path == std::path::Path::new("HIT-IT-AETHERFORGE-OBS-GUARD.sh")
        })
        .unwrap();
    assert_eq!(hit.project.as_deref(), Some("AetherForge"));
    let unrelated = report
        .entries
        .iter()
        .find(|entry| entry.relative_path == std::path::Path::new("Battle.net-Setup.exe"))
        .unwrap();
    assert_eq!(unrelated.project, None);
    assert!(
        report
            .projects
            .iter()
            .all(|project| !project.name.contains("DOWNLOADS40"))
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn summary_path_uses_bounded_entry_samples() {
    let root = temp_root("summary");
    let downloads = root.join("Downloads/misc");
    fs::create_dir_all(&downloads).unwrap();
    for index in 0..1_000u32 {
        fs::write(downloads.join(format!("file-{index}.dat")), b"x").unwrap();
    }
    let report = scan_downloads_inventory_summary(&root.join("Downloads")).unwrap();
    assert!(report.total.file_paths >= 1_000);
    assert!(report.entries.len() <= 512);
    fs::remove_dir_all(root).unwrap();
}
