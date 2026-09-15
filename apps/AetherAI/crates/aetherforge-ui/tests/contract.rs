use aetherforge_ui::{AetherForgeVisualContract, Rgba};

#[test]
fn terminal_contract_is_exact() {
    let c = AetherForgeVisualContract::terminal_canonical();

    assert_eq!(c.schema_id, "AETHERFORGE_TERMINAL_GLOBAL_75");
    assert_eq!(c.surface_opacity, 0.25);
    assert_eq!(c.global_transparency, 0.75);
    assert_eq!(c.main_surface_alpha, 64);
    assert_eq!(c.titlebar_height_px, 28);
    assert_eq!(c.window_control_size_px, 20);
    assert_eq!(c.window_control_spacing_px, 3);
    assert_eq!(c.window_control_left_inset_px, 6);
    assert_eq!(c.value_text, Rgba::from_hex("#F6F2FF").unwrap());
    assert_eq!(c.label_text, Rgba::from_hex("#B9AECF").unwrap());
    assert_eq!(c.heading_text, Rgba::from_hex("#F8F4FF").unwrap());
    assert!(c.body_weight_min >= 650);
    assert!(c.heading_weight_min >= 700);
}

#[test]
fn theme_is_derived_from_terminal_contract() {
    let theme = aetherforge_ui::DragonGlassTheme::terminal_canonical();
    assert_eq!(
        theme.contract(),
        &AetherForgeVisualContract::terminal_canonical()
    );
    assert_eq!(theme.surface.a, 64);
    assert_eq!(theme.value_text, Rgba::from_hex("#F6F2FF").unwrap());
}
