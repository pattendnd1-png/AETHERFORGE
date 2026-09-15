use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_hex(input: &str) -> Result<Self, String> {
        let hex = input.strip_prefix('#').unwrap_or(input);
        if hex.len() != 6 && hex.len() != 8 {
            return Err(format!("expected RRGGBB or RRGGBBAA, got {input}"));
        }
        let parse = |range: std::ops::Range<usize>| {
            u8::from_str_radix(&hex[range], 16)
                .map_err(|error| format!("invalid color {input}: {error}"))
        };
        let r = parse(0..2)?;
        let g = parse(2..4)?;
        let b = parse(4..6)?;
        let a = if hex.len() == 8 { parse(6..8)? } else { 255 };
        Ok(Self { r, g, b, a })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AetherForgeVisualContract {
    pub schema_id: String,
    pub surface_opacity: f32,
    pub global_transparency: f32,
    pub main_surface_alpha: u8,
    pub titlebar_height_px: u16,
    pub window_control_size_px: u16,
    pub window_control_spacing_px: u16,
    pub window_control_left_inset_px: u16,
    pub value_text: Rgba,
    pub label_text: Rgba,
    pub heading_text: Rgba,
    pub body_weight_min: u16,
    pub heading_weight_min: u16,
}

impl AetherForgeVisualContract {
    pub fn terminal_canonical() -> Self {
        Self {
            schema_id: "AETHERFORGE_TERMINAL_GLOBAL_75".into(),
            surface_opacity: 0.25,
            global_transparency: 0.75,
            main_surface_alpha: 64,
            titlebar_height_px: 28,
            window_control_size_px: 20,
            window_control_spacing_px: 3,
            window_control_left_inset_px: 6,
            value_text: Rgba::from_hex("#F6F2FF").expect("canonical value text"),
            label_text: Rgba::from_hex("#B9AECF").expect("canonical label text"),
            heading_text: Rgba::from_hex("#F8F4FF").expect("canonical heading text"),
            body_weight_min: 740,
            heading_weight_min: 750,
        }
    }
}
