use forgeclean::organizer::{
    Category, ForgeLayout, build_identity_from_entry, classify_download, organize_once,
    top_level_downloads,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_root(tag: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("forgeclean-{tag}-{}-{stamp}", std::process::id()))
}

#[test]
fn classifies_project_tree_and_document() {
    let root = temp_root("organizer-classify");
    let project = root.join("Demo-v1.2.3");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("Cargo.toml"),
        "[package]\nname='demo'\nversion='1.2.3'\n",
    )
    .unwrap();
    let classified = classify_download(&project);
    assert!(classified.is_project_tree);
    assert_eq!(classified.project.as_deref(), Some("Demo"));

    let document = root.join("notes.pdf");
    fs::write(&document, b"pdf").unwrap();
    assert_eq!(classify_download(&document).category, Category::Documents);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn top_level_scan_never_reenters_forgeclean_root_or_legacy_symlink() {
    let root = temp_root("organizer-scan");
    fs::create_dir_all(&root).unwrap();
    let layout = ForgeLayout::new(&root);
    layout.ensure().unwrap();
    fs::write(root.join("notes.txt"), b"hello").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(layout.root.join("Projects"), root.join("legacy-project")).unwrap();
    let paths = top_level_downloads(&layout).unwrap();
    assert_eq!(paths, vec![root.join("notes.txt")]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn parses_build_identity_across_aetherforge_artifact_names() {
    let cases = [
        ("ForgeClean-v0.5.2-HIT-IT.sh", "ForgeClean", "v0.5.2"),
        ("ForgeHX-10.0.30-VERIFY.txt", "ForgeHX", "10.0.30"),
        (
            "AetherForge-Control-Center-V10_2_66-AETHERDISPLAY-SHA256SUMS.txt",
            "AetherForge-Control-Center",
            "v10.2.66",
        ),
    ];
    for (name, project, version) in cases {
        let identity = build_identity_from_entry(PathBuf::from(name).as_path()).unwrap();
        assert_eq!(identity.project, project);
        assert_eq!(identity.version, version);
    }
}

#[test]
fn organizer_groups_build_bundle_and_leaves_legacy_aliases() {
    let root = temp_root("organizer-build-bundle");
    fs::create_dir_all(&root).unwrap();
    let layout = ForgeLayout::new(&root);
    let names = [
        "ForgeClean-v0.5.2-SOURCE.zip",
        "ForgeClean-v0.5.2-HIT-IT.sh",
        "ForgeClean-v0.5.2-SHA256SUMS.txt",
        "ForgeClean-v0.5.2-VERIFY.txt",
    ];
    for name in names {
        fs::write(root.join(name), format!("payload:{name}\n")).unwrap();
    }

    let report = organize_once(&layout, std::time::Duration::ZERO);
    assert!(report.is_ok(), "issues: {:?}", report.issues);
    assert_eq!(report.sorted_entries, names.len());
    assert_eq!(report.cold_archives, 0);

    let build = layout.project_build("ForgeClean", "v0.5.2");
    for name in names {
        let canonical = build.join(name);
        let legacy = root.join(name);
        assert!(
            canonical.is_file(),
            "missing canonical {}",
            canonical.display()
        );
        let meta = fs::symlink_metadata(&legacy).unwrap();
        assert!(
            meta.file_type().is_symlink(),
            "legacy path is not symlink: {}",
            legacy.display()
        );
        assert_eq!(
            fs::canonicalize(&legacy).unwrap(),
            fs::canonicalize(&canonical).unwrap()
        );
        assert!(
            !layout
                .project_cold("ForgeClean")
                .join(format!("{name}.fcoldpack"))
                .exists()
        );
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn organizer_reconciles_existing_legacy_buckets_into_project_builds() {
    let root = temp_root("organizer-existing-builds");
    fs::create_dir_all(&root).unwrap();
    let layout = ForgeLayout::new(&root);
    layout.ensure().unwrap();

    let legacy = [
        (layout.root.join("Archives"), "ForgeHX-10.0.30-SOURCE.zip"),
        (layout.root.join("Documents"), "ForgeHX-10.0.30-VERIFY.txt"),
        (layout.root.join("Installers"), "ForgeHX-10.0.30-HIT-IT.sh"),
    ];
    for (dir, name) in &legacy {
        fs::write(dir.join(name), format!("payload:{name}\n")).unwrap();
    }

    let report = organize_once(&layout, std::time::Duration::ZERO);
    assert!(report.is_ok(), "issues: {:?}", report.issues);
    assert_eq!(report.sorted_entries, legacy.len());

    let build = layout.project_build("ForgeHX", "10.0.30");
    for (dir, name) in &legacy {
        let canonical = build.join(name);
        let old = dir.join(name);
        assert!(
            canonical.is_file(),
            "missing canonical {}",
            canonical.display()
        );
        let meta = fs::symlink_metadata(&old).unwrap();
        assert!(
            meta.file_type().is_symlink(),
            "legacy path is not symlink: {}",
            old.display()
        );
        assert_eq!(
            fs::canonicalize(&old).unwrap(),
            fs::canonicalize(&canonical).unwrap()
        );
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn organizer_continuously_reconciles_new_artifacts_in_legacy_project_areas() {
    let root = temp_root("organizer-continuous-builds");
    fs::create_dir_all(&root).unwrap();
    let layout = ForgeLayout::new(&root);
    layout.ensure().unwrap();

    let old_releases = layout.project_releases("AetherAI");
    fs::create_dir_all(&old_releases).unwrap();
    let first = old_releases.join("AetherAI-v0.3.0-SOURCE.zip");
    fs::write(&first, b"source").unwrap();
    let first_report = organize_once(&layout, std::time::Duration::ZERO);
    assert!(first_report.is_ok(), "issues: {:?}", first_report.issues);
    assert!(
        layout
            .project_build("AetherAI", "v0.3.0")
            .join("AetherAI-v0.3.0-SOURCE.zip")
            .is_file()
    );

    let later = old_releases.join("AetherAI-v0.3.0-HIT-IT.sh");
    fs::write(&later, b"#!/bin/sh\n").unwrap();
    let second_report = organize_once(&layout, std::time::Duration::ZERO);
    assert!(second_report.is_ok(), "issues: {:?}", second_report.issues);
    assert!(
        layout
            .project_build("AetherAI", "v0.3.0")
            .join("AetherAI-v0.3.0-HIT-IT.sh")
            .is_file()
    );
    assert!(
        fs::symlink_metadata(&later)
            .unwrap()
            .file_type()
            .is_symlink()
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn incomplete_downloads_never_become_build_artifacts() {
    for name in [
        "ForgeClean-v0.5.2-SOURCE.zip.part",
        "ForgeHX-10.0.30-HIT-IT.sh.partial",
        "AetherAI-v0.3.0-SOURCE.zip.tmp",
    ] {
        assert!(
            build_identity_from_entry(PathBuf::from(name).as_path()).is_none(),
            "transient download was treated as completed build artifact: {name}"
        );
    }
}
