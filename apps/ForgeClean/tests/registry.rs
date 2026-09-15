use forgeclean::registry::{ProjectRegistry, create_legacy_alias};
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
fn registry_round_trip_resolves_active_path() {
    let root = temp_root("registry");
    fs::create_dir_all(&root).unwrap();
    let file = root.join("registry.tsv");
    let active = root.join("ForgeClean/Projects/Demo/Active");
    let legacy = root.join("Demo-v1.0.0");
    let mut registry = ProjectRegistry::default();
    registry.set_active("Demo", active.clone(), vec![legacy.clone()]);
    registry.write(&file).unwrap();
    let read = ProjectRegistry::read(&file).unwrap();
    let record = read.resolve("demo").unwrap();
    assert_eq!(record.active_path, active);
    assert_eq!(record.legacy_paths, vec![legacy]);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn legacy_alias_never_overwrites_real_path() {
    let root = temp_root("legacy-guard");
    let active = root.join("ForgeClean/Projects/Demo/Active");
    fs::create_dir_all(&active).unwrap();
    let legacy = root.join("Demo-v1.0.0");
    fs::write(&legacy, b"real file").unwrap();
    let err = create_legacy_alias(&root, &legacy, &active).unwrap_err();
    assert!(err.contains("refusing to overwrite real legacy path"));
    fs::remove_dir_all(root).unwrap();
}
