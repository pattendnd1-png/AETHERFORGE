use crate::{
    editor::{Appearance, AssetRecord, ControlSlot, Page, Workspace},
    pack_manager, plugin_host,
};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::{self, FilterType};
use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use std::{path::Path, process::Command};

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

    let mut window = if resolve_touch_presentation(page, active_app_id) == "unified" {
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
    overlay_dial_stack_status(&mut window, page);

    Ok(RenderedDeck {
        keys,
        window: encode_jpeg(&window)?,
        warnings,
    })
}

fn overlay_dial_stack_status(window: &mut RgbaImage, page: &Page) {
    for dial in &page.slots.dials {
        let Some(stack) = dial.dial_stack.as_ref() else {
            continue;
        };
        if stack.entries.is_empty() || dial.position >= 4 {
            continue;
        }
        let index = stack.active_index.min(stack.entries.len() - 1);
        let entry = &stack.entries[index];
        let mut overlay =
            RgbaImage::from_pixel(WINDOW_REGION_WIDTH, WINDOW_HEIGHT, Rgba([0, 0, 0, 0]));
        let mut appearance = page
            .slots
            .touch_regions
            .get(dial.position)
            .map(|slot| slot.appearance.clone())
            .unwrap_or_else(|| dial.appearance.clone());
        appearance.title = format!("{} {}/{}", entry.label, index + 1, stack.entries.len());
        appearance.title_visible = true;
        appearance.font_size = 10;
        appearance.font_weight = 700;
        appearance.text_color = "#ffffff".into();
        appearance.horizontal_align = "center".into();
        appearance.vertical_align = "bottom".into();
        appearance.title_offset_x = 0;
        appearance.title_offset_y = -3;
        draw_title(&mut overlay, &appearance);
        imageops::overlay(
            window,
            &overlay,
            i64::from(dial.position as u32 * WINDOW_REGION_WIDTH),
            0,
        );
    }
    for dial in &page.slots.dials {
        let Some(wheel) = dial.action_wheel.as_ref() else {
            continue;
        };
        if wheel.entries.is_empty() || dial.position >= 4 {
            continue;
        }
        let index = wheel.active_index.min(wheel.entries.len() - 1);
        let entry = &wheel.entries[index];
        let mut overlay =
            RgbaImage::from_pixel(WINDOW_REGION_WIDTH, WINDOW_HEIGHT, Rgba([0, 0, 0, 0]));
        let mut appearance = page
            .slots
            .touch_regions
            .get(dial.position)
            .map(|slot| slot.appearance.clone())
            .unwrap_or_else(|| dial.appearance.clone());
        appearance.title = format!("{} {}/{}", entry.label, index + 1, wheel.entries.len());
        appearance.title_visible = true;
        appearance.font_size = 10;
        appearance.font_weight = 700;
        appearance.text_color = "#ffffff".into();
        appearance.horizontal_align = "center".into();
        appearance.vertical_align = "bottom".into();
        appearance.title_offset_x = 0;
        appearance.title_offset_y = -3;
        draw_title(&mut overlay, &appearance);
        imageops::overlay(
            window,
            &overlay,
            i64::from(dial.position as u32 * WINDOW_REGION_WIDTH),
            0,
        );
    }
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

fn binding_definition_id(value: &serde_json::Value) -> Option<&str> {
    value
        .get("definitionId")
        .and_then(serde_json::Value::as_str)
}

fn primary_action_id(slot: &ControlSlot) -> Option<&str> {
    for interaction in [
        "press",
        "touch",
        "rotateRight",
        "rotateLeft",
        "pressRotateRight",
        "pressRotateLeft",
    ] {
        if let Some(id) = slot
            .bindings
            .get(interaction)
            .and_then(binding_definition_id)
        {
            return Some(id);
        }
    }
    slot.bindings.values().find_map(binding_definition_id)
}

fn load_icon_path(path: &Path) -> Result<DynamicImage, String> {
    match image::open(path) {
        Ok(image) => Ok(image),
        Err(primary)
            if path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("svg")) =>
        {
            for (program, args) in [
                ("rsvg-convert", vec![path.as_os_str().to_os_string()]),
                (
                    "magick",
                    vec![path.as_os_str().to_os_string(), "png:-".into()],
                ),
            ] {
                if let Ok(output) = Command::new(program).args(args).output() {
                    if output.status.success() && !output.stdout.is_empty() {
                        if let Ok(image) = image::load_from_memory(&output.stdout) {
                            return Ok(image);
                        }
                    }
                }
            }
            Err(format!(
                "{}: {primary}; SVG rasterizer unavailable",
                path.display()
            ))
        }
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

fn composite_external_icon(
    canvas: &mut RgbaImage,
    path: &Path,
    fit_mode: &str,
    opacity: f32,
    warnings: &mut Vec<String>,
    role: &str,
) -> bool {
    match load_icon_path(path) {
        Ok(image) => {
            let max_width = canvas.width().saturating_mul(68) / 100;
            let max_height = canvas.height().saturating_mul(62) / 100;
            let fitted = fit_image(&image, max_width.max(1), max_height.max(1), fit_mode);
            let mut layer = fitted.to_rgba8();
            let clamped = opacity.clamp(0.0, 1.0);
            if clamped < 1.0 {
                for pixel in layer.pixels_mut() {
                    pixel.0[3] = ((f32::from(pixel.0[3])) * clamped).round() as u8;
                }
            }
            let x = i64::from((canvas.width().saturating_sub(layer.width())) / 2);
            let y =
                i64::from((canvas.height().saturating_sub(layer.height())) / 2).saturating_sub(8);
            imageops::overlay(canvas, &layer, x, y);
            true
        }
        Err(error) => {
            warnings.push(format!("{role} {}: {error}", path.display()));
            false
        }
    }
}

fn put_pixel(canvas: &mut RgbaImage, x: i32, y: i32, color: Rgba<u8>) {
    if x >= 0 && y >= 0 && (x as u32) < canvas.width() && (y as u32) < canvas.height() {
        canvas.put_pixel(x as u32, y as u32, color);
    }
}

fn line(
    canvas: &mut RgbaImage,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    width: i32,
    color: Rgba<u8>,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        let radius = (width.max(1) - 1) / 2;
        for oy in -radius..=radius {
            for ox in -radius..=radius {
                put_pixel(canvas, x0 + ox, y0 + oy, color);
            }
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn circle(canvas: &mut RgbaImage, cx: i32, cy: i32, radius: i32, width: i32, color: Rgba<u8>) {
    let r2 = radius * radius;
    let inner = (radius - width.max(1)).max(0);
    let inner2 = inner * inner;
    for y in (cy - radius)..=(cy + radius) {
        for x in (cx - radius)..=(cx + radius) {
            let d = (x - cx) * (x - cx) + (y - cy) * (y - cy);
            if d <= r2 && d >= inner2 {
                put_pixel(canvas, x, y, color);
            }
        }
    }
}

fn fill_circle(canvas: &mut RgbaImage, cx: i32, cy: i32, radius: i32, color: Rgba<u8>) {
    let r2 = radius * radius;
    for y in (cy - radius)..=(cy + radius) {
        for x in (cx - radius)..=(cx + radius) {
            if (x - cx) * (x - cx) + (y - cy) * (y - cy) <= r2 {
                put_pixel(canvas, x, y, color);
            }
        }
    }
}

fn builtin_icon_kind(action_id: Option<&str>, title: &str) -> &'static str {
    let value = format!("{} {}", action_id.unwrap_or_default(), title).to_ascii_lowercase();
    if value.contains("microphone") || value.contains("mic") || value.contains("mute") {
        "mic"
    } else if value.contains("browser") || value.contains("web") {
        "globe"
    } else if value.contains("spotify") || value.contains("music") {
        "music"
    } else if value.contains("discord") {
        "discord"
    } else if value.contains("twitch") || value.contains("chat") {
        "chat"
    } else if value.contains("obs") {
        "obs"
    } else if value.contains("volume") || value.contains("audio") {
        "volume"
    } else if value.contains("brightness") || value.contains("scene") || value.contains("system") {
        "sun"
    } else if value.contains("media") || value.contains("play") {
        "play"
    } else {
        "generic"
    }
}

fn draw_builtin_icon(
    canvas: &mut RgbaImage,
    action_id: Option<&str>,
    title: &str,
    color: Rgba<u8>,
) {
    let cx = canvas.width() as i32 / 2;
    let cy = canvas.height() as i32 * 38 / 100;
    match builtin_icon_kind(action_id, title) {
        "mic" => {
            circle(canvas, cx, cy - 5, 12, 3, color);
            line(canvas, cx - 18, cy - 2, cx - 18, cy + 5, 3, color);
            line(canvas, cx + 18, cy - 2, cx + 18, cy + 5, 3, color);
            line(canvas, cx - 18, cy + 5, cx, cy + 15, 3, color);
            line(canvas, cx + 18, cy + 5, cx, cy + 15, 3, color);
            line(canvas, cx, cy + 15, cx, cy + 24, 3, color);
            line(canvas, cx - 10, cy + 24, cx + 10, cy + 24, 3, color);
        }
        "globe" => {
            circle(canvas, cx, cy, 23, 3, color);
            line(canvas, cx - 22, cy, cx + 22, cy, 2, color);
            circle(canvas, cx, cy, 10, 2, color);
        }
        "music" => {
            line(canvas, cx - 4, cy - 22, cx + 17, cy - 28, 4, color);
            line(canvas, cx - 4, cy - 22, cx - 4, cy + 11, 4, color);
            line(canvas, cx + 17, cy - 28, cx + 17, cy + 5, 4, color);
            fill_circle(canvas, cx - 11, cy + 13, 8, color);
            fill_circle(canvas, cx + 10, cy + 7, 8, color);
        }
        "obs" => {
            circle(canvas, cx, cy - 12, 13, 4, color);
            circle(canvas, cx - 13, cy + 10, 13, 4, color);
            circle(canvas, cx + 13, cy + 10, 13, 4, color);
        }
        "chat" => {
            line(canvas, cx - 24, cy - 18, cx + 24, cy - 18, 3, color);
            line(canvas, cx - 24, cy - 18, cx - 24, cy + 14, 3, color);
            line(canvas, cx + 24, cy - 18, cx + 24, cy + 14, 3, color);
            line(canvas, cx - 24, cy + 14, cx + 8, cy + 14, 3, color);
            line(canvas, cx + 8, cy + 14, cx + 20, cy + 25, 3, color);
            line(canvas, cx + 20, cy + 25, cx + 20, cy + 14, 3, color);
        }
        "discord" => {
            circle(canvas, cx, cy, 23, 3, color);
            fill_circle(canvas, cx - 9, cy, 4, color);
            fill_circle(canvas, cx + 9, cy, 4, color);
            line(canvas, cx - 12, cy + 12, cx + 12, cy + 12, 3, color);
        }
        "volume" => {
            line(canvas, cx - 22, cy - 8, cx - 12, cy - 8, 4, color);
            line(canvas, cx - 12, cy - 8, cx, cy - 20, 4, color);
            line(canvas, cx, cy - 20, cx, cy + 20, 4, color);
            line(canvas, cx, cy + 20, cx - 12, cy + 8, 4, color);
            line(canvas, cx - 12, cy + 8, cx - 22, cy + 8, 4, color);
            circle(canvas, cx + 9, cy, 11, 3, color);
            circle(canvas, cx + 9, cy, 20, 3, color);
        }
        "sun" => {
            circle(canvas, cx, cy, 11, 4, color);
            for (dx, dy) in [
                (0, -28),
                (0, 28),
                (-28, 0),
                (28, 0),
                (-20, -20),
                (20, -20),
                (-20, 20),
                (20, 20),
            ] {
                line(
                    canvas,
                    cx + dx * 2 / 3,
                    cy + dy * 2 / 3,
                    cx + dx,
                    cy + dy,
                    3,
                    color,
                );
            }
        }
        "play" => {
            line(canvas, cx - 13, cy - 21, cx - 13, cy + 21, 4, color);
            line(canvas, cx - 13, cy - 21, cx + 23, cy, 4, color);
            line(canvas, cx + 23, cy, cx - 13, cy + 21, 4, color);
        }
        _ => {
            circle(canvas, cx, cy, 23, 3, color);
            fill_circle(canvas, cx, cy, 5, color);
        }
    }
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
    let action_id = primary_action_id(slot);
    let mut icon_rendered = false;
    if let Some(asset_id) = appearance.icon_asset_id.as_deref() {
        icon_rendered = composite_asset(
            &mut canvas,
            workspace,
            asset_id,
            &appearance.fit_mode,
            appearance.icon_opacity,
            warnings,
            "icon",
        );
    } else if let Some(action_id) = action_id {
        if let Some(path) = plugin_host::action_state_image_path(action_id) {
            icon_rendered = composite_external_icon(
                &mut canvas,
                &path,
                &appearance.fit_mode,
                appearance.icon_opacity,
                warnings,
                "plugin icon",
            );
        }
    }
    if !icon_rendered {
        if let Some(path) =
            pack_manager::active_icon_path(action_id, &appearance.title, slot.position)
        {
            icon_rendered = composite_external_icon(
                &mut canvas,
                &path,
                &appearance.fit_mode,
                appearance.icon_opacity,
                warnings,
                "icon pack",
            );
        }
    }
    if !icon_rendered && (!appearance.title.trim().is_empty() || action_id.is_some()) {
        let color = parse_hex_color(&appearance.text_color).unwrap_or([255, 255, 255, 255]);
        draw_builtin_icon(&mut canvas, action_id, &appearance.title, Rgba(color));
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
) -> bool {
    let Some(asset) = workspace.assets.iter().find(|asset| asset.id == asset_id) else {
        warnings.push(format!(
            "{role} asset {asset_id} is not in the workspace index"
        ));
        return false;
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
            true
        }
        Err(error) => {
            warnings.push(format!("{role} asset {}: {error}", asset.name));
            false
        }
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
    fn hardware_key_renderer_draws_an_icon_even_without_an_explicit_asset() {
        let mut workspace = default_workspace();
        let slot = &mut workspace.profiles[0].pages[0].slots.keys[0];
        slot.appearance.title = "OBS".into();
        slot.appearance.background_color = "#101010".into();
        let rendered = render_workspace(&workspace, None).unwrap();
        let decoded = image::load_from_memory(&rendered.keys[0])
            .unwrap()
            .to_rgba8();
        let bright_upper_pixels = decoded
            .enumerate_pixels()
            .filter(|(_, y, pixel)| {
                *y < 82 && pixel.0[0] > 150 && pixel.0[1] > 150 && pixel.0[2] > 150
            })
            .count();
        assert!(
            bright_upper_pixels > 20,
            "device key image must contain visible icon pixels above the title"
        );
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

    #[test]
    fn dial_stack_status_changes_touch_window_pixels() {
        let base = render_workspace(&default_workspace(), None).unwrap();
        let mut workspace = default_workspace();
        workspace.profiles[0].pages[0].slots.dials[0].dial_stack = Some(crate::editor::DialStack {
            behavior: "pressCycle".into(),
            active_index: 0,
            entries: vec![crate::editor::DialStackEntry {
                id: "stack-volume".into(),
                label: "VOLUME".into(),
                bindings: serde_json::Map::new(),
            }],
        });
        let stacked = render_workspace(&workspace, None).unwrap();
        assert_ne!(base.window, stacked.window);
    }

    #[test]
    fn action_wheel_status_changes_touch_window_pixels() {
        let base = render_workspace(&default_workspace(), None).unwrap();
        let mut workspace = default_workspace();
        workspace.profiles[0].pages[0].slots.dials[1].action_wheel =
            Some(crate::editor::ActionWheel {
                behavior: "rotateSelectPressExecute".into(),
                active_index: 0,
                entries: vec![crate::editor::ActionWheelEntry {
                    id: "wheel-obs".into(),
                    label: "OBS".into(),
                    bindings: serde_json::Map::new(),
                }],
            });
        let wheeled = render_workspace(&workspace, None).unwrap();
        assert_ne!(base.window, wheeled.window);
    }
}
