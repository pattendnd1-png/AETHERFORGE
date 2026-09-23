use forgeclean::project_storage::scan_project_download_storage;
use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::symlink;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_root(label: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "forgeclean-project-storage-{label}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn counts_only_project_delivery_locations_and_excludes_generated_trees() {
    let root = temp_root("scope");
    let downloads = root.join("Downloads");
    let project = downloads.join("ForgeClean/Projects/AetherBrowser");
    let builds = project.join("Builds/3.0.31");
    let project_downloads = project.join("Downloads");
    let releases = project.join("Releases");
    fs::create_dir_all(builds.join("target/release")).unwrap();
    fs::create_dir_all(builds.join("build/cache")).unwrap();
    fs::create_dir_all(project.join("Active/target")).unwrap();
    fs::create_dir_all(&project_downloads).unwrap();
    fs::create_dir_all(&releases).unwrap();
    fs::write(
        builds.join("AetherBrowser-v3.0.31-SOURCE.zip"),
        vec![1u8; 3000],
    )
    .unwrap();
    fs::write(project_downloads.join("WidevineCdm.zip"), vec![2u8; 5000]).unwrap();
    fs::write(
        releases.join("AetherBrowser-v3.0.31-HIT-IT.sh"),
        vec![3u8; 7000],
    )
    .unwrap();
    fs::write(
        builds.join("target/release/aether-browser"),
        vec![4u8; 11000],
    )
    .unwrap();
    fs::write(builds.join("build/cache/object.o"), vec![5u8; 13000]).unwrap();
    fs::write(project.join("Active/source.rs"), vec![6u8; 17000]).unwrap();

    let report = scan_project_download_storage(&downloads).unwrap();
    assert_eq!(report.projects.len(), 1);
    assert_eq!(report.projects[0].name, "AetherBrowser");
    assert_eq!(report.projects[0].artifact_paths, 3);
    assert_eq!(report.projects[0].logical_bytes, 15_000);
    assert_eq!(report.artifact_paths, 3);
    assert_eq!(report.logical_bytes, 15_000);
    assert!(report.allocated_bytes > 0);
    assert!(report.excluded_generated_dirs >= 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reports_actual_allocated_blocks_and_deduplicates_hardlinks() {
    let root = temp_root("allocated");
    let downloads = root.join("Downloads");
    let build = downloads.join("ForgeClean/Projects/ForgeClean/Builds/1.0.19");
    fs::create_dir_all(&build).unwrap();

    let sparse = build.join("ForgeClean-v1.0.19-SOURCE.img");
    let mut file = File::create(&sparse).unwrap();
    file.seek(SeekFrom::Start(8 * 1024 * 1024 - 1)).unwrap();
    file.write_all(&[0]).unwrap();
    let hardlink = build.join("ForgeClean-v1.0.19-SOURCE-copy.img");
    fs::hard_link(&sparse, &hardlink).unwrap();

    let report = scan_project_download_storage(&downloads).unwrap();
    assert_eq!(report.artifact_paths, 2);
    assert_eq!(report.unique_files, 1);
    assert_eq!(report.logical_bytes, 8 * 1024 * 1024);
    assert!(report.allocated_bytes < report.logical_bytes);
    assert!(report.deduplicated_paths >= 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn includes_recognized_unrouted_top_level_project_downloads_but_not_personal_files_or_symlinks() {
    let root = temp_root("unrouted");
    let downloads = root.join("Downloads");
    fs::create_dir_all(&downloads).unwrap();
    fs::write(
        downloads.join("OpenDeck-v2.0.71-SOURCE.zip"),
        vec![1u8; 4096],
    )
    .unwrap();
    fs::write(downloads.join("vacation-photo.jpg"), vec![2u8; 4096]).unwrap();
    symlink(
        downloads.join("OpenDeck-v2.0.71-SOURCE.zip"),
        downloads.join("OpenDeck-v2.0.71-copy.zip"),
    )
    .unwrap();

    let report = scan_project_download_storage(&downloads).unwrap();
    assert_eq!(report.projects.len(), 1);
    assert_eq!(report.projects[0].name, "OpenDeck");
    assert_eq!(report.projects[0].artifact_paths, 1);
    assert_eq!(report.unrouted_artifact_paths, 1);
    assert_eq!(report.symlinks_skipped, 1);
    fs::remove_dir_all(root).unwrap();
}
