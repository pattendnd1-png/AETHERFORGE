use aether_ui::DragonGlassTokens;

#[test]
fn canonical_dragonglass_uses_ninety_percent_transparency() {
    let tokens = DragonGlassTokens::canonical();
    assert_eq!(tokens.surface_transparency_percent, 90);
    assert_eq!(tokens.smoky_surface_percent, 10);
    assert!(tokens.rounded_geometry);
    assert_eq!(tokens.accent_primary.name, "violet");
    assert_eq!(tokens.accent_secondary.name, "indigo");
}
