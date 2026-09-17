use crate::editor::{Appearance, AssetRecord, ControlSlot, Page, Workspace};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::{self, FilterType};
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use std::path::Path;

pub const KEY_WIDTH: u32 = 120;
pub const KEY_HEIGHT: u32 = 120;
pub const WINDOW_WIDTH: u32 = 800;
pub const WINDOW_HEIGHT: u32 = 100;
const WINDOW_REGION_WIDTH: u32 = WINDOW_WIDTH / 4;

#[derive(Debug)]
pub(crate) struct RenderedDeck {
    pub keys: Vec<Vec<u8>>,
    pub window: Vec<u8>,
    pub warnings: Vec<String>,
}

pub(crate) fn render_workspace(
    workspace: &Workspace,
    active_app_id: Option<&str>,
) -> Result<RenderedDeck, String> {
    let page = active_page(workspace)?;
    let mut warnings = Vec::new();
    let mut keys = Vec::with_capacity(8);
    for slot in &page.slots.keys {
        keys.push(render_slot(
            slot,
            workspace,
            KEY_WIDTH,
            KEY_HEIGHT,
            &mut warnings,
        )?);
    }

    let window = if resolve_touch_presentation(page, active_app_id) == "unified" {
        render_slot_image(
            &page.touch_strip.unified_slot,
            workspace,
            WINDOW_WIDTH,
            WINDOW_HEIGHT,
            &mut warnings,
        )
    } else {
        let mut window =
            RgbaImage::from_pixel(WINDOW_WIDTH, WINDOW_HEIGHT, Rgba([10, 11, 14, 255]));
        for slot in &page.slots.touch_regions {
            if slot.position >= 4 {
                continue;
            }
            let region = render_slot_image(
                slot,
                workspace,
                WINDOW_REGION_WIDTH,
                WINDOW_HEIGHT,
                &mut warnings,
            );
            imageops::overlay(
                &mut window,
                &region,
                i64::from(slot.position as u32 * WINDOW_REGION_WIDTH),
                0,
            );
        }
        window
    };

    Ok(RenderedDeck {
        keys,
        window: encode_jpeg(&window)?,
        warnings,
    })
}

fn resolve_touch_presentation<'a>(page: &'a Page, active_app_id: Option<&str>) -> &'a str {
    match page.touch_strip.mode.as_str() {
        "segmented" => "segmented",
        "unified" => "unified",
        _ => {
            if let Some(app_id) = active_app_id {
                let app_id = app_id.trim().to_ascii_lowercase();
                if !app_id.is_empty() {
                    for rule in &page.touch_strip.adaptive_rules {
                        let pattern = rule.pattern.trim().to_ascii_lowercase();
                        if !pattern.is_empty() && app_id.contains(&pattern) {
                            return rule.presentation.as_str();
                        }
                    }
                }
            }
            page.touch_strip.adaptive_fallback.as_str()
        }
    }
}

fn active_page(workspace: &Workspace) -> Result<&Page, String> {
    let profile = workspace
        .profiles
        .iter()
        .find(|profile| profile.id == workspace.active_profile_id)
        .or_else(|| workspace.profiles.first())
        .ok_or_else(|| "workspace has no profile to render".to_string())?;
    profile
        .pages
        .iter()
        .find(|page| page.id == profile.active_page_id)
        .or_else(|| profile.pages.first())
        .ok_or_else(|| "active profile has no page to render".to_string())
}

fn render_slot(
    slot: &ControlSlot,
    workspace: &Workspace,
    width: u32,
    height: u32,
    warnings: &mut Vec<String>,
) -> Result<Vec<u8>, String> {
    let image = render_slot_image(slot, workspace, width, height, warnings);
    encode_jpeg(&image)
}

fn render_slot_image(
    slot: &ControlSlot,
    workspace: &Workspace,
    width: u32,
    height: u32,
    warnings: &mut Vec<String>,
) -> RgbaImage {
    let appearance = &slot.appearance;
    let background = parse_hex_color(&appearance.background_color).unwrap_or([22, 24, 29, 255]);
    let mut canvas = RgbaImage::from_pixel(width, height, Rgba(background));

    if let Some(asset_id) = appearance.background_asset_id.as_deref() {
        composite_asset(
            &mut canvas,
            workspace,
            asset_id,
            "cover",
            1.0,
            warnings,
            "background",
        );
    }
    if let Some(asset_id) = appearance.icon_asset_id.as_deref() {
        composite_asset(
            &mut canvas,
            workspace,
            asset_id,
            &appearance.fit_mode,
            appearance.icon_opacity,
            warnings,
            "icon",
        );
    }
    if appearance.title_visible && !appearance.title.trim().is_empty() {
        draw_title(&mut canvas, appearance);
    }
    canvas
}

fn composite_asset(
    canvas: &mut RgbaImage,
    workspace: &Workspace,
    asset_id: &str,
    fit_mode: &str,
    opacity: f32,
    warnings: &mut Vec<String>,
    role: &str,
) {
    let Some(asset) = workspace.assets.iter().find(|asset| asset.id == asset_id) else {
        warnings.push(format!(
            "{role} asset {asset_id} is not in the workspace index"
        ));
        return;
    };
    match load_asset(asset) {
        Ok(image) => {
            let fitted = fit_image(&image, canvas.width(), canvas.height(), fit_mode);
            let mut layer = fitted.to_rgba8();
            let clamped = opacity.clamp(0.0, 1.0);
            if clamped < 1.0 {
                for pixel in layer.pixels_mut() {
                    pixel.0[3] = ((f32::from(pixel.0[3])) * clamped).round() as u8;
                }
            }
            let x = i64::from((canvas.width().saturating_sub(layer.width())) / 2);
            let y = i64::from((canvas.height().saturating_sub(layer.height())) / 2);
            imageops::overlay(canvas, &layer, x, y);
        }
        Err(error) => warnings.push(format!("{role} asset {}: {error}", asset.name)),
    }
}

fn load_asset(asset: &AssetRecord) -> Result<DynamicImage, String> {
    let path = Path::new(&asset.path);
    if !path.is_file() {
        return Err(format!("{} does not exist", path.display()));
    }
    image::open(path).map_err(|error| error.to_string())
}

fn fit_image(image: &DynamicImage, width: u32, height: u32, fit_mode: &str) -> DynamicImage {
    match fit_mode {
        "stretch" => image.resize_exact(width, height, FilterType::Lanczos3),
        "cover" => {
            let (source_w, source_h) = image.dimensions();
            if source_w == 0 || source_h == 0 {
                return image.clone();
            }
            let scale = (width as f64 / source_w as f64).max(height as f64 / source_h as f64);
            let scaled_w = ((source_w as f64 * scale).ceil() as u32).max(width);
            let scaled_h = ((source_h as f64 * scale).ceil() as u32).max(height);
            let resized = image.resize_exact(scaled_w, scaled_h, FilterType::Lanczos3);
            let left = (scaled_w.saturating_sub(width)) / 2;
            let top = (scaled_h.saturating_sub(height)) / 2;
            resized.crop_imm(left, top, width, height)
        }
        _ => image.resize(width, height, FilterType::Lanczos3),
    }
}

fn encode_jpeg(image: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, 90)
        .encode_image(&DynamicImage::ImageRgba8(image.clone()))
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

fn parse_hex_color(value: &str) -> Option<[u8; 4]> {
    let hex = value.strip_prefix('#')?;
    if !hex.is_ascii() {
        return None;
    }
    match hex.len() {
        6 => Some([
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            255,
        ]),
        8 => Some([
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            u8::from_str_radix(&hex[6..8], 16).ok()?,
        ]),
        _ => None,
    }
}

fn draw_title(canvas: &mut RgbaImage, appearance: &Appearance) {
    let text = appearance.title.trim().to_uppercase();
    if text.is_empty() {
        return;
    }
    let scale = (appearance.font_size.max(8) as u32 / 7).clamp(1, 4);
    let glyph_width = 5 * scale;
    let spacing = scale;
    let max_chars = ((canvas.width() + spacing) / (glyph_width + spacing)).max(1) as usize;
    let text: String = text.chars().take(max_chars).collect();
    let count = text.chars().count() as u32;
    let text_width = count
        .saturating_mul(glyph_width + spacing)
        .saturating_sub(spacing);
    let text_height = 7 * scale;
    let base_x = match appearance.horizontal_align.as_str() {
        "left" => 4,
        "right" => canvas.width().saturating_sub(text_width + 4),
        _ => canvas.width().saturating_sub(text_width) / 2,
    };
    let base_y = match appearance.vertical_align.as_str() {
        "top" => 4,
        "middle" => canvas.height().saturating_sub(text_height) / 2,
        _ => canvas.height().saturating_sub(text_height + 5),
    };
    let x = offset_coordinate(base_x, appearance.title_offset_x, canvas.width());
    let y = offset_coordinate(base_y, appearance.title_offset_y, canvas.height());
    let color = parse_hex_color(&appearance.text_color).unwrap_or([255, 255, 255, 255]);

    for (index, ch) in text.chars().enumerate() {
        draw_glyph(
            canvas,
            ch,
            x.saturating_add(index as u32 * (glyph_width + spacing)),
            y,
            scale,
            Rgba(color),
        );
    }
}

fn offset_coordinate(base: u32, delta: i16, bound: u32) -> u32 {
    let moved = i64::from(base) + i64::from(delta);
    moved.clamp(0, i64::from(bound.saturating_sub(1))) as u32
}

fn draw_glyph(canvas: &mut RgbaImage, ch: char, x: u32, y: u32, scale: u32, color: Rgba<u8>) {
    let rows = glyph(ch);
    for (row, bits) in rows.iter().enumerate() {
        for col in 0..5u32 {
            if bits & (1 << (4 - col)) == 0 {
                continue;
            }
            for dy in 0..scale {
                for dx in 0..scale {
                    let px = x + col * scale + dx;
                    let py = y + row as u32 * scale + dy;
                    if px < canvas.width() && py < canvas.height() {
                        canvas.put_pixel(px, py, color);
                    }
                }
            }
        }
    }
}

fn glyph(ch: char) -> [u8; 7] {
    match ch {
        'A' => [0x0e, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        'B' => [0x1e, 0x11, 0x11, 0x1e, 0x11, 0x11, 0x1e],
        'C' => [0x0e, 0x11, 0x10, 0x10, 0x10, 0x11, 0x0e],
        'D' => [0x1e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1e],
        'E' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x1f],
        'F' => [0x1f, 0x10, 0x10, 0x1e, 0x10, 0x10, 0x10],
        'G' => [0x0f, 0x10, 0x10, 0x17, 0x11, 0x11, 0x0f],
        'H' => [0x11, 0x11, 0x11, 0x1f, 0x11, 0x11, 0x11],
        'I' => [0x0e, 0x04, 0x04, 0x04, 0x04, 0x04, 0x0e],
        'J' => [0x07, 0x02, 0x02, 0x02, 0x12, 0x12, 0x0c],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1f],
        'M' => [0x11, 0x1b, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'O' => [0x0e, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        'P' => [0x1e, 0x11, 0x11, 0x1e, 0x10, 0x10, 0x10],
        'Q' => [0x0e, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0d],
        'R' => [0x1e, 0x11, 0x11, 0x1e, 0x14, 0x12, 0x11],
        'S' => [0x0f, 0x10, 0x10, 0x0e, 0x01, 0x01, 0x1e],
        'T' => [0x1f, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0e],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0a, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x15, 0x0a],
        'X' => [0x11, 0x11, 0x0a, 0x04, 0x0a, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0a, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1f],
        '0' => [0x0e, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0e],
        '1' => [0x04, 0x0c, 0x04, 0x04, 0x04, 0x04, 0x0e],
        '2' => [0x0e, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1f],
        '3' => [0x1e, 0x01, 0x01, 0x0e, 0x01, 0x01, 0x1e],
        '4' => [0x02, 0x06, 0x0a, 0x12, 0x1f, 0x02, 0x02],
        '5' => [0x1f, 0x10, 0x10, 0x1e, 0x01, 0x01, 0x1e],
        '6' => [0x0e, 0x10, 0x10, 0x1e, 0x11, 0x11, 0x0e],
        '7' => [0x1f, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0e, 0x11, 0x11, 0x0e, 0x11, 0x11, 0x0e],
        '9' => [0x0e, 0x11, 0x11, 0x0f, 0x01, 0x01, 0x0e],
        '-' => [0x00, 0x00, 0x00, 0x1f, 0x00, 0x00, 0x00],
        '_' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1f],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x0c],
        ':' => [0x00, 0x0c, 0x0c, 0x00, 0x0c, 0x0c, 0x00],
        '/' => [0x01, 0x02, 0x02, 0x04, 0x08, 0x08, 0x10],
        ' ' => [0; 7],
        _ => [0x0e, 0x11, 0x02, 0x04, 0x04, 0x00, 0x04],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::default_workspace;

    #[test]
    fn renders_eight_120_square_keys_and_800_by_100_window() {
        let rendered = render_workspace(&default_workspace(), None).unwrap();
        assert_eq!(rendered.keys.len(), 8);
        for key in rendered.keys {
            let decoded = image::load_from_memory(&key).unwrap();
            assert_eq!(decoded.dimensions(), (KEY_WIDTH, KEY_HEIGHT));
        }
        let decoded_window = image::load_from_memory(&rendered.window).unwrap();
        assert_eq!(decoded_window.dimensions(), (WINDOW_WIDTH, WINDOW_HEIGHT));
        assert!(rendered.warnings.is_empty());
    }

    #[test]
    fn missing_asset_is_warning_not_render_failure() {
        let mut workspace = default_workspace();
        workspace.profiles[0].pages[0].slots.keys[0]
            .appearance
            .icon_asset_id = Some("missing".into());
        let rendered = render_workspace(&workspace, None).unwrap();
        assert_eq!(rendered.keys.len(), 8);
        assert_eq!(rendered.warnings.len(), 1);
        assert!(rendered.warnings[0].contains("missing"));
    }

    #[test]
    fn parses_renderer_colors() {
        assert_eq!(parse_hex_color("#112233"), Some([0x11, 0x22, 0x33, 0xff]));
        assert_eq!(parse_hex_color("#11223344"), Some([0x11, 0x22, 0x33, 0x44]));
        assert_eq!(parse_hex_color("112233"), None);
    }

    #[test]
    fn unified_touch_strip_renders_one_full_800_by_100_surface() {
        let mut workspace = default_workspace();
        workspace.profiles[0].pages[0].touch_strip.mode = "unified".into();
        workspace.profiles[0].pages[0]
            .touch_strip
            .unified_slot
            .appearance
            .background_color = "#3366ff".into();
        let rendered = render_workspace(&workspace, None).unwrap();
        let decoded = image::load_from_memory(&rendered.window).unwrap();
        assert_eq!(decoded.dimensions(), (WINDOW_WIDTH, WINDOW_HEIGHT));
    }

    #[test]
    fn adaptive_touch_strip_uses_first_matching_application_rule() {
        let mut workspace = default_workspace();
        let touch = &mut workspace.profiles[0].pages[0].touch_strip;
        touch.mode = "adaptive".into();
        touch.adaptive_fallback = "segmented".into();
        touch.adaptive_rules.push(crate::editor::TouchStripRule {
            pattern: "spotify".into(),
            presentation: "unified".into(),
        });
        assert_eq!(
            resolve_touch_presentation(&workspace.profiles[0].pages[0], Some("com.spotify.Client")),
            "unified"
        );
        assert_eq!(
            resolve_touch_presentation(
                &workspace.profiles[0].pages[0],
                Some("org.mozilla.firefox")
            ),
            "segmented"
        );
    }
}
