use eframe::egui;
use reforge_core::RgbColor;
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Copy)]
struct KeyDef {
    id: u8,
    label: &'static str,
    width: f32,
}

impl KeyDef {
    const fn new(id: u8, label: &'static str, width: f32) -> Self {
        Self { id, label, width }
    }
}

fn rows() -> Vec<Vec<KeyDef>> {
    vec![
        vec![
            KeyDef::new(41, "Esc", 42.0), KeyDef::new(58, "F1", 38.0), KeyDef::new(59, "F2", 38.0),
            KeyDef::new(60, "F3", 38.0), KeyDef::new(61, "F4", 38.0), KeyDef::new(62, "F5", 38.0),
            KeyDef::new(63, "F6", 38.0), KeyDef::new(64, "F7", 38.0), KeyDef::new(65, "F8", 38.0),
            KeyDef::new(66, "F9", 38.0), KeyDef::new(67, "F10", 42.0), KeyDef::new(68, "F11", 42.0),
            KeyDef::new(69, "F12", 42.0),
        ],
        vec![
            KeyDef::new(53, "`", 38.0), KeyDef::new(30, "1", 38.0), KeyDef::new(31, "2", 38.0),
            KeyDef::new(32, "3", 38.0), KeyDef::new(33, "4", 38.0), KeyDef::new(34, "5", 38.0),
            KeyDef::new(35, "6", 38.0), KeyDef::new(36, "7", 38.0), KeyDef::new(37, "8", 38.0),
            KeyDef::new(38, "9", 38.0), KeyDef::new(39, "0", 38.0), KeyDef::new(45, "-", 38.0),
            KeyDef::new(46, "=", 38.0), KeyDef::new(42, "Back", 68.0),
        ],
        vec![
            KeyDef::new(43, "Tab", 58.0), KeyDef::new(20, "Q", 38.0), KeyDef::new(26, "W", 38.0),
            KeyDef::new(8, "E", 38.0), KeyDef::new(21, "R", 38.0), KeyDef::new(23, "T", 38.0),
            KeyDef::new(28, "Y", 38.0), KeyDef::new(24, "U", 38.0), KeyDef::new(12, "I", 38.0),
            KeyDef::new(18, "O", 38.0), KeyDef::new(19, "P", 38.0), KeyDef::new(47, "[", 38.0),
            KeyDef::new(48, "]", 38.0), KeyDef::new(49, "\\", 48.0),
        ],
        vec![
            KeyDef::new(57, "Caps", 68.0), KeyDef::new(4, "A", 38.0), KeyDef::new(22, "S", 38.0),
            KeyDef::new(7, "D", 38.0), KeyDef::new(9, "F", 38.0), KeyDef::new(10, "G", 38.0),
            KeyDef::new(11, "H", 38.0), KeyDef::new(13, "J", 38.0), KeyDef::new(14, "K", 38.0),
            KeyDef::new(15, "L", 38.0), KeyDef::new(51, ";", 38.0), KeyDef::new(52, "'", 38.0),
            KeyDef::new(40, "Enter", 78.0),
        ],
        vec![
            KeyDef::new(225, "Shift", 88.0), KeyDef::new(29, "Z", 38.0), KeyDef::new(27, "X", 38.0),
            KeyDef::new(6, "C", 38.0), KeyDef::new(25, "V", 38.0), KeyDef::new(5, "B", 38.0),
            KeyDef::new(17, "N", 38.0), KeyDef::new(16, "M", 38.0), KeyDef::new(54, ",", 38.0),
            KeyDef::new(55, ".", 38.0), KeyDef::new(56, "/", 38.0), KeyDef::new(229, "Shift", 92.0),
        ],
        vec![
            KeyDef::new(224, "Ctrl", 54.0), KeyDef::new(227, "Meta", 54.0), KeyDef::new(226, "Alt", 54.0),
            KeyDef::new(44, "Space", 250.0), KeyDef::new(230, "Alt", 54.0), KeyDef::new(231, "Meta", 54.0),
            KeyDef::new(228, "Ctrl", 54.0), KeyDef::new(80, "←", 38.0), KeyDef::new(81, "↓", 38.0),
            KeyDef::new(82, "↑", 38.0), KeyDef::new(79, "→", 38.0),
        ],
    ]
}

fn text_color(color: RgbColor) -> egui::Color32 {
    let luminance = (u32::from(color.r) * 299 + u32::from(color.g) * 587 + u32::from(color.b) * 114) / 1000;
    if luminance > 135 { egui::Color32::BLACK } else { egui::Color32::WHITE }
}

pub fn show(
    ui: &mut egui::Ui,
    supported_keys: &[u8],
    colors: &mut BTreeMap<u8, RgbColor>,
    base: RgbColor,
    brush: RgbColor,
) -> bool {
    let supported: HashSet<u8> = supported_keys.iter().copied().collect();
    let mut changed = false;
    let layout = rows();
    let known: HashSet<u8> = layout.iter().flatten().map(|key| key.id).collect();
    egui::ScrollArea::horizontal().show(ui, |ui| {
        for row in layout {
            ui.horizontal(|ui| {
                for key in row {
                    let available = supported.contains(&key.id);
                    let color = colors.get(&key.id).copied().unwrap_or(base);
                    let text = egui::RichText::new(key.label).color(text_color(color));
                    let button = egui::Button::new(text).fill(egui::Color32::from_rgb(color.r, color.g, color.b));
                    let response = ui.add_enabled(available, button.min_size(egui::vec2(key.width, 34.0)));
                    if response.clicked() {
                        colors.insert(key.id, brush);
                        changed = true;
                    }
                    if available {
                        response.on_hover_text(format!("HID lighting zone 0x{:02X}", key.id));
                    }
                }
            });
            ui.add_space(3.0);
        }
        let extras: Vec<u8> = supported_keys
            .iter()
            .copied()
            .filter(|key| !known.contains(key))
            .collect();
        if !extras.is_empty() {
            ui.separator();
            ui.small("Additional device zones");
            ui.horizontal_wrapped(|ui| {
                for key in extras {
                    let color = colors.get(&key).copied().unwrap_or(base);
                    let text = egui::RichText::new(format!("{:02X}", key)).color(text_color(color));
                    let response = ui.add(
                        egui::Button::new(text)
                            .fill(egui::Color32::from_rgb(color.r, color.g, color.b))
                            .min_size(egui::vec2(40.0, 32.0)),
                    );
                    if response.clicked() {
                        colors.insert(key, brush);
                        changed = true;
                    }
                    response.on_hover_text(format!("Device-specific HID lighting zone 0x{key:02X}"));
                }
            });
        }
    });
    changed
}
