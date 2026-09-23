use forgeclean::project_routing::{ProjectIndex, derive_family_name};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_root(label: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "forgeclean-project-routing-{label}-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn derives_family_from_versions_and_milestones() {
    assert_eq!(
        derive_family_name("OpenDeck-v2.0.71"),
        Some("OpenDeck".to_owned())
    );
    assert_eq!(
        derive_family_name("ForgeHX-10.0.31"),
        Some("ForgeHX".to_owned())
    );
    assert_eq!(
        derive_family_name("OpenRA-Rust-M6-RTS-Selection-Orders"),
        Some("OpenRA-Rust".to_owned())
    );
    assert_eq!(
        derive_family_name("Darkstone-RS-M1D4-Halfword-Placement"),
        Some("Darkstone-RS".to_owned())
    );
}

#[test]
fn longest_named_project_family_wins() {
    let root = temp_root("longest");
    let downloads = root.join("Downloads");
    for name in ["AetherForge", "AetherForge-BEACN-Control"] {
        let dir = downloads.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Cargo.toml"), b"[workspace]\n").unwrap();
    }
    let index = ProjectIndex::discover(&downloads);
    assert_eq!(
        index
            .family_for_artifact_name("AetherForge-BEACN-Control-v0.1.20-VERIFY.txt")
            .as_deref(),
        Some("AetherForge-BEACN-Control")
    );
    assert_eq!(
        index
            .family_for_artifact_name("AetherForge-DOWNLOADS40-THEME-DIAGNOSTICS.txt")
            .as_deref(),
        Some("AetherForge")
    );
    assert_eq!(index.family_for_artifact_name("vacation-photo.jpg"), None);
    fs::remove_dir_all(root).unwrap();
}
