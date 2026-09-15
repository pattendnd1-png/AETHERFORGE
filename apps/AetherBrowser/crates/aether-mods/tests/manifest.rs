use aether_mods::ModManifest;

#[test]
fn mods_receive_no_implicit_privileges() {
    let manifest = ModManifest::new("dev.aether.example", "Example", "1.0.0");
    assert!(manifest.permissions().is_empty());
}
