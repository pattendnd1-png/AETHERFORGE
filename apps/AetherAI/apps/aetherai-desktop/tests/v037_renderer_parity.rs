use aether_terminal_ui::DragonGlassTheme as TerminalTheme;
use aetherforge_ui::AetherForgeVisualContract;
use std::fs;
use std::path::{Path, PathBuf};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read(p: impl AsRef<Path>) -> String {
    fs::read_to_string(p).unwrap_or_default()
}
#[test]
fn linux_primary_renderer_is_terminal_eframe_wgpu_stack() {
    let r = root();
    let m = read(r.join("src/main.rs"));
    let n = read(r.join("src/native_ui.rs"));
    assert!(m.contains("native_ui::run(app)"));
    assert!(n.contains("eframe::run_native"));
    assert!(n.contains("eframe::Renderer::Wgpu"));
    assert!(n.contains("with_transparent(true)"));
    let clear_start = n
        .find("fn clear_color")
        .expect("eframe renderer must define transparent clear_color");
    let clear_window = &n[clear_start..n.len().min(clear_start + 260)];
    let clear_compact: String = clear_window
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect();
    assert!(
        clear_compact.contains("[0.0,0.0,0.0,0.0]"),
        "clear_color must return a fully transparent framebuffer clear"
    );
}
#[test]
fn shared_contract_matches_live_terminal() {
    let c = AetherForgeVisualContract::terminal_canonical();
    let t = TerminalTheme::canonical();
    assert_eq!(c.main_surface_alpha, t.background.a);
    assert_eq!(c.titlebar_height_px, t.decoration.title_height as u16);
    assert_eq!(c.window_control_size_px, t.decoration.button_width as u16);
    assert_eq!(
        c.window_control_spacing_px,
        t.decoration.button_spacing as u16
    );
    assert_eq!(
        c.window_control_left_inset_px,
        t.decoration.title_border_left as u16
    );
    assert_eq!(
        (
            c.value_text.r,
            c.value_text.g,
            c.value_text.b,
            c.value_text.a
        ),
        (
            t.value_text.r,
            t.value_text.g,
            t.value_text.b,
            t.value_text.a
        )
    );
    assert_eq!(
        (
            c.label_text.r,
            c.label_text.g,
            c.label_text.b,
            c.label_text.a
        ),
        (
            t.label_text.r,
            t.label_text.g,
            t.label_text.b,
            t.label_text.a
        )
    );
    assert_eq!(
        (
            c.heading_text.r,
            c.heading_text.g,
            c.heading_text.b,
            c.heading_text.a
        ),
        (
            t.heading_text.r,
            t.heading_text.g,
            t.heading_text.b,
            t.heading_text.a
        )
    );
    assert_eq!(c.body_weight_min, t.body_weight);
    assert_eq!(c.heading_weight_min, t.heading_weight);
}
#[test]
fn compatibility_theme_is_terminal_derived() {
    let a = aether_ui::current_theme();
    let t = TerminalTheme::canonical();
    assert_eq!(
        (a.app_fill.r, a.app_fill.g, a.app_fill.b, a.app_fill.a),
        (
            t.background.r,
            t.background.g,
            t.background.b,
            t.background.a
        )
    );
    assert_eq!(
        (
            a.terminal_fill.r,
            a.terminal_fill.g,
            a.terminal_fill.b,
            a.terminal_fill.a
        ),
        (
            t.terminal_surface.r,
            t.terminal_surface.g,
            t.terminal_surface.b,
            t.terminal_surface.a
        )
    );
    assert_eq!(a.corner_radius, t.decoration.window_radius);
    assert_eq!(a.compact_row_height, t.decoration.title_height);
}
#[test]
fn no_independent_palette_literals() {
    let u = read(root().join("../../crates/aether-ui/src/lib.rs"));
    assert!(u.contains("aether_terminal_ui::DragonGlassTheme::canonical()"));
    for s in [
        "app_fill: Rgba::new(",
        "panel_fill: Rgba::new(",
        "terminal_fill: Rgba::new(",
        "text_primary: Rgba::new(",
        "accent: Rgba::new(",
    ] {
        assert!(!u.contains(s));
    }
}
#[test]
fn raw_x11_removed() {
    let p = read(root().join("src/platform.rs"));
    for s in ["XCreateSimpleWindow", "XDrawString", "XFillRectangle"] {
        assert!(!p.contains(s));
    }
}
#[test]
fn terminal_is_lazy() {
    let r = root();
    assert!(!read(r.join("src/main.rs")).contains(".ensure_session("));
    assert!(!read(r.join("src/native_ui.rs")).contains(".ensure_session("));
}
