use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Stroke, Vec2,
};
use sanctuary_casc::StorageKind;
use sanctuary_core::InstallState;
use sanctuary_launcher::{ActivityItem, BridgeState, InventoryState, LauncherModel};

use crate::theme::{ACCENT_BLUE, DANGER_RED, SUCCESS_GREEN, WARNING_AMBER};
use crate::ui_motion::{motion_amount, progress_from_detail, queue_progress, selection_emphasis};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ActivityVisualState {
    Active,
    Complete,
    Failed,
}

pub(super) fn activity_visual_state(item: &ActivityItem) -> ActivityVisualState {
    if item.failed {
        ActivityVisualState::Failed
    } else if item.done {
        ActivityVisualState::Complete
    } else {
        ActivityVisualState::Active
    }
}

pub(super) fn activity_progress_fraction(item: &ActivityItem, pulse: f32) -> f32 {
    if item.done || item.failed {
        1.0
    } else {
        progress_from_detail(&item.detail)
            .unwrap_or_else(|| queue_progress(item.done, item.failed, pulse))
    }
}

pub(super) fn activity_notification_count(model: &LauncherModel) -> usize {
    model.activity.iter().filter(|item| !item.done).count()
}

pub(super) fn activity_tray_summary(model: &LauncherModel) -> String {
    let active = model.activity.iter().filter(|item| !item.done).count();
    if active > 0 {
        return format!("{active} active");
    }
    model.activity.last().map_or_else(
        || "Idle".into(),
        |item| format!("{} • {}", item.label, item.detail),
    )
}
pub(super) fn subnav_button(
    ui: &mut egui::Ui,
    label: &str,
    active: bool,
    mut action: impl FnMut(),
) {
    let text = RichText::new(label).strong().small().color(if active {
        Color32::from_rgb(220, 232, 242)
    } else {
        Color32::from_rgb(119, 140, 159)
    });
    let response = ui.add(egui::Button::new(text).frame(false));
    if active {
        ui.painter().rect_filled(
            Rect::from_min_max(
                Pos2::new(response.rect.left() + 3.0, response.rect.bottom() - 1.0),
                Pos2::new(response.rect.right() - 3.0, response.rect.bottom() + 1.0),
            ),
            CornerRadius::ZERO,
            Color32::from_rgb(48, 157, 222),
        );
    }
    if response.clicked() {
        action();
    }
}

pub(super) fn native_status_badge(ui: &mut egui::Ui, ready: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(78.0, 26.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(4), Color32::from_rgb(13, 22, 30));
    ui.painter().circle_filled(
        Pos2::new(rect.left() + 11.0, rect.center().y),
        3.5,
        if ready {
            Color32::from_rgb(70, 190, 132)
        } else {
            Color32::from_rgb(221, 153, 65)
        },
    );
    ui.painter().text(
        Pos2::new(rect.left() + 20.0, rect.center().y),
        Align2::LEFT_CENTER,
        if ready { "NATIVE" } else { "CHECK" },
        FontId::proportional(9.5),
        Color32::from_rgb(153, 171, 186),
    );
}

pub(super) fn account_top_button(ui: &mut egui::Ui, active: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(70.0, 28.0), egui::Sense::click());
    if active || response.hovered() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(4),
            if active {
                Color32::from_rgb(27, 49, 67)
            } else {
                Color32::from_rgb(21, 31, 41)
            },
        );
    }
    let avatar = Pos2::new(rect.left() + 13.0, rect.center().y);
    ui.painter()
        .circle_filled(avatar, 8.0, Color32::from_rgb(35, 105, 157));
    ui.painter().text(
        avatar,
        Align2::CENTER_CENTER,
        "B",
        FontId::proportional(8.5),
        Color32::WHITE,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 26.0, rect.center().y),
        Align2::LEFT_CENTER,
        "ACCOUNT ▾",
        FontId::proportional(9.0),
        Color32::from_rgb(160, 177, 191),
    );
    response.clicked()
}

pub(super) fn paint_mini_diablo_icon(ui: &mut egui::Ui, rect: Rect, selected: bool) {
    ui.painter()
        .rect_filled(rect, CornerRadius::same(5), Color32::from_rgb(18, 31, 42));
    ui.painter()
        .circle_filled(rect.center(), 10.0, Color32::from_rgb(28, 74, 108));
    ui.painter().circle_stroke(
        rect.center(),
        11.5,
        Stroke::new(
            1.0,
            if selected {
                Color32::from_rgb(201, 111, 51)
            } else {
                Color32::from_rgb(83, 104, 122)
            },
        ),
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "III",
        FontId::proportional(9.0),
        Color32::from_rgb(232, 227, 216),
    );
}

pub(super) fn game_strip_tile(
    ui: &mut egui::Ui,
    tooltip: &str,
    selected: bool,
    reduced_motion: bool,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(142.0, 46.0), egui::Sense::click());
    let now = ui.input(|input| input.time);
    let motion = motion_amount(reduced_motion, now, 2.1);
    let emphasis = selection_emphasis(selected, response.hovered(), reduced_motion, now);
    let lift = if response.hovered() && !reduced_motion {
        1.0 + emphasis * 1.6
    } else {
        0.0
    };
    let painted = rect.translate(Vec2::new(0.0, -lift));
    let fill = if selected {
        Color32::from_rgb(20, 34, 47)
    } else if response.hovered() {
        Color32::from_rgb(22, 32, 43)
    } else {
        Color32::from_rgb(14, 21, 29)
    };

    if selected || response.hovered() {
        ui.painter().rect_stroke(
            painted.expand(2.0),
            CornerRadius::same(5),
            Stroke::new(
                1.0 + emphasis * 0.65,
                Color32::from_rgba_unmultiplied(45, 154, 224, (28.0 + emphasis * 54.0) as u8),
            ),
            egui::StrokeKind::Inside,
        );
    }
    ui.painter()
        .rect_filled(painted, CornerRadius::same(5), fill);
    ui.painter().rect_stroke(
        painted,
        CornerRadius::same(5),
        Stroke::new(1.0, Color32::from_rgb(48, 62, 76)),
        egui::StrokeKind::Inside,
    );

    let emblem_center = Pos2::new(painted.left() + 25.0, painted.center().y);
    let ember_alpha = (88.0 + motion * 54.0) as u8;
    ui.painter().circle_filled(
        emblem_center,
        13.0,
        Color32::from_rgba_unmultiplied(27, 73, 108, 220),
    );
    ui.painter().circle_stroke(
        emblem_center,
        14.5 + motion,
        Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(211, 111, 47, ember_alpha),
        ),
    );
    ui.painter().text(
        emblem_center,
        Align2::CENTER_CENTER,
        "III",
        FontId::proportional(10.5),
        Color32::from_rgb(238, 232, 220),
    );
    ui.painter().text(
        Pos2::new(painted.left() + 48.0, painted.center().y - 6.0),
        Align2::LEFT_CENTER,
        "DIABLO III",
        FontId::proportional(11.5),
        if selected {
            Color32::from_rgb(219, 229, 237)
        } else {
            Color32::from_rgb(154, 172, 187)
        },
    );
    ui.painter().text(
        Pos2::new(painted.left() + 48.0, painted.center().y + 9.0),
        Align2::LEFT_CENTER,
        "Native runtime",
        FontId::proportional(9.0),
        Color32::from_rgb(91, 118, 141),
    );

    if selected {
        ui.painter().rect_filled(
            Rect::from_min_max(
                Pos2::new(painted.left() + 4.0, painted.bottom() - 3.0),
                Pos2::new(painted.right() - 4.0, painted.bottom()),
            ),
            CornerRadius::same(1),
            Color32::from_rgb(43, 159, 226),
        );
    }
    let clicked = response.clicked();
    let _ = response.on_hover_text(tooltip);
    clicked
}

pub(super) fn home_last_played_card(
    ui: &mut egui::Ui,
    title: &str,
    detail: &str,
    ready: bool,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(238.0, 72.0), egui::Sense::click());
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(5),
        if response.hovered() {
            Color32::from_rgb(24, 38, 50)
        } else {
            Color32::from_rgb(15, 24, 33)
        },
    );
    let icon = Rect::from_min_size(
        Pos2::new(rect.left() + 10.0, rect.top() + 10.0),
        Vec2::splat(52.0),
    );
    ui.painter()
        .rect_filled(icon, CornerRadius::same(4), Color32::from_rgb(23, 55, 78));
    ui.painter().circle_stroke(
        icon.center(),
        16.0,
        Stroke::new(1.0, Color32::from_rgb(201, 111, 51)),
    );
    ui.painter().text(
        icon.center(),
        Align2::CENTER_CENTER,
        "III",
        FontId::proportional(12.0),
        Color32::from_rgb(236, 230, 219),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 76.0, rect.top() + 17.0),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(13.0),
        Color32::from_rgb(210, 223, 233),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 76.0, rect.top() + 39.0),
        Align2::LEFT_TOP,
        detail,
        FontId::proportional(9.5),
        Color32::from_rgb(104, 129, 150),
    );
    ui.painter().circle_filled(
        Pos2::new(rect.right() - 16.0, rect.top() + 17.0),
        3.0,
        if ready {
            Color32::from_rgb(73, 194, 140)
        } else {
            Color32::from_rgb(221, 153, 65)
        },
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(5),
        Stroke::new(1.0, Color32::from_rgb(42, 57, 70)),
        egui::StrokeKind::Inside,
    );
    response.clicked()
}

pub(super) fn home_last_played_placeholder(ui: &mut egui::Ui, label: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(178.0, 72.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(5), Color32::from_rgb(12, 18, 25));
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(5),
        Stroke::new(1.0, Color32::from_rgb(31, 42, 53)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(10.0),
        Color32::from_rgb(78, 96, 111),
    );
}

pub(super) fn last_played_tile(
    ui: &mut egui::Ui,
    title: &str,
    detail: &str,
    reduced_motion: bool,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(126.0, 40.0), egui::Sense::click());
    let now = ui.input(|input| input.time);
    let emphasis = selection_emphasis(false, response.hovered(), reduced_motion, now);
    let painted = if response.hovered() && !reduced_motion {
        rect.translate(Vec2::new(0.0, -emphasis * 1.5))
    } else {
        rect
    };
    ui.painter().rect_filled(
        painted,
        CornerRadius::same(4),
        if response.hovered() {
            Color32::from_rgb(23, 35, 47)
        } else {
            Color32::from_rgb(15, 23, 31)
        },
    );
    ui.painter().rect_stroke(
        painted,
        CornerRadius::same(4),
        Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(60, 128, 172, (54.0 + emphasis * 44.0) as u8),
        ),
        egui::StrokeKind::Inside,
    );
    let icon = Pos2::new(painted.left() + 19.0, painted.center().y);
    ui.painter()
        .circle_filled(icon, 10.0, Color32::from_rgb(39, 76, 103));
    ui.painter().text(
        icon,
        Align2::CENTER_CENTER,
        "III",
        FontId::proportional(8.5),
        Color32::from_rgb(232, 227, 216),
    );
    ui.painter().text(
        Pos2::new(painted.left() + 36.0, painted.center().y - 6.0),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(10.0),
        Color32::from_rgb(199, 214, 225),
    );
    ui.painter().text(
        Pos2::new(painted.left() + 36.0, painted.center().y + 8.0),
        Align2::LEFT_CENTER,
        detail,
        FontId::proportional(8.5),
        Color32::from_rgb(97, 122, 143),
    );
    response.clicked()
}

pub(super) fn game_strip_placeholder(ui: &mut egui::Ui, text: &str, tooltip: &str) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(42.0, 36.0), egui::Sense::hover());
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(4),
        if response.hovered() {
            Color32::from_rgb(26, 34, 44)
        } else {
            Color32::from_rgb(17, 23, 31)
        },
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0, Color32::from_rgb(43, 55, 68)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(13.0),
        Color32::from_rgb(117, 136, 153),
    );
    let _ = response.on_hover_text(tooltip);
}

pub(super) fn activity_status_icon(ui: &mut egui::Ui, active: usize, reduced_motion: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(22.0, 22.0), egui::Sense::hover());
    if active == 0 {
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            "↓",
            FontId::proportional(15.0),
            Color32::from_rgb(70, 160, 220),
        );
        return;
    }

    let now = ui.input(|input| input.time);
    let pulse = motion_amount(reduced_motion, now, 2.4);
    ui.painter()
        .circle_filled(rect.center(), 4.0, Color32::from_rgb(64, 160, 222));
    ui.painter().circle_stroke(
        rect.center(),
        7.0 + pulse * 3.0,
        Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(64, 160, 222, (105.0 - pulse * 45.0) as u8),
        ),
    );
}

pub(super) fn notification_badge_button(ui: &mut egui::Ui, count: usize, active: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(86.0, 28.0), egui::Sense::click());
    if active || response.hovered() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(4),
            if active {
                Color32::from_rgb(27, 49, 67)
            } else {
                Color32::from_rgb(23, 34, 46)
            },
        );
    }
    ui.painter().text(
        Pos2::new(rect.left() + 8.0, rect.center().y),
        Align2::LEFT_CENTER,
        "NOTICES",
        FontId::proportional(10.5),
        Color32::from_rgb(167, 184, 198),
    );
    if count > 0 {
        let badge = Pos2::new(rect.right() - 12.0, rect.center().y);
        ui.painter()
            .circle_filled(badge, 8.0, Color32::from_rgb(36, 137, 207));
        ui.painter().text(
            badge,
            Align2::CENTER_CENTER,
            count.min(9).to_string(),
            FontId::proportional(9.0),
            Color32::WHITE,
        );
    }
    response.clicked()
}

pub(super) fn utility_top_button(ui: &mut egui::Ui, label: &str, active: bool) -> bool {
    ui.add(
        egui::Button::new(RichText::new(label).small().color(if active {
            Color32::from_rgb(216, 228, 237)
        } else {
            Color32::from_rgb(151, 169, 184)
        }))
        .frame(false),
    )
    .clicked()
}
pub(super) fn drawer_message(ui: &mut egui::Ui, title: &str, detail: &str) {
    ui.group(|ui| {
        ui.set_min_width(258.0);
        ui.label(
            RichText::new(title)
                .strong()
                .color(Color32::from_rgb(205, 217, 228)),
        );
        ui.label(
            RichText::new(detail)
                .small()
                .color(Color32::from_rgb(126, 147, 166)),
        );
    });
}
pub(super) fn paint_featured_accent(
    ui: &mut egui::Ui,
    rect: Rect,
    index: usize,
    reduced_motion: bool,
    now: f64,
) {
    let pulse = motion_amount(reduced_motion, now + index as f64 * 0.7, 0.75);
    let painter = ui.painter();
    match index % 3 {
        0 => {
            let center = Pos2::new(rect.right() - rect.width() * 0.24, rect.center().y);
            for ring in 0..4 {
                painter.circle_stroke(
                    center,
                    54.0 + ring as f32 * 26.0 + pulse * 4.0,
                    Stroke::new(
                        1.0,
                        Color32::from_rgba_unmultiplied(52, 151, 219, 62 - ring as u8 * 10),
                    ),
                );
            }
        }
        1 => {
            for bar in 0..6 {
                let x = rect.right() - 270.0 + bar as f32 * 34.0;
                let height = 60.0 + bar as f32 * 18.0 + pulse * 12.0;
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(x, rect.center().y - height * 0.5),
                        Pos2::new(x + 6.0, rect.center().y + height * 0.5),
                    ),
                    CornerRadius::same(2),
                    Color32::from_rgba_unmultiplied(206, 111, 48, 64),
                );
            }
        }
        _ => {
            for node in 0..8 {
                let angle = node as f32 * 0.785 + pulse * 0.08;
                let center = Pos2::new(rect.right() - 165.0, rect.center().y);
                let point = center
                    + Vec2::new(angle.cos(), angle.sin()) * (68.0 + (node % 2) as f32 * 28.0);
                painter.circle_filled(
                    point,
                    4.0,
                    Color32::from_rgba_unmultiplied(83, 188, 148, 120),
                );
                painter.line_segment(
                    [center, point],
                    Stroke::new(1.0, Color32::from_rgba_unmultiplied(83, 188, 148, 38)),
                );
            }
        }
    }
}

pub(super) fn latest_feature_card(
    ui: &mut egui::Ui,
    category: &str,
    title: &str,
    detail: &str,
    ready: bool,
) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 202.0), egui::Sense::click());
    let fill = if response.hovered() {
        Color32::from_rgb(24, 38, 51)
    } else {
        Color32::from_rgb(16, 25, 35)
    };
    ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
    let art = Rect::from_min_max(rect.min, Pos2::new(rect.right(), rect.top() + 88.0));
    for band in 0..8 {
        let y0 = art.top() + art.height() * band as f32 / 8.0;
        let y1 = art.top() + art.height() * (band + 1) as f32 / 8.0 + 1.0;
        ui.painter().rect_filled(
            Rect::from_min_max(Pos2::new(art.left(), y0), Pos2::new(art.right(), y1)),
            CornerRadius::ZERO,
            Color32::from_rgb(
                11 + band as u8 * 2,
                28 + band as u8 * 2,
                42 + band as u8 * 3,
            ),
        );
    }
    ui.painter().circle_stroke(
        Pos2::new(art.right() - 65.0, art.center().y),
        34.0,
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(63, 161, 222, 88)),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 15.0, rect.top() + 16.0),
        Align2::LEFT_TOP,
        category,
        FontId::proportional(10.0),
        Color32::from_rgb(73, 169, 228),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 15.0, rect.top() + 108.0),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(16.0),
        Color32::from_rgb(215, 226, 235),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 15.0, rect.top() + 137.0),
        Align2::LEFT_TOP,
        detail,
        FontId::proportional(10.8),
        Color32::from_rgb(121, 143, 162),
    );
    ui.painter().text(
        Pos2::new(rect.right() - 15.0, rect.bottom() - 13.0),
        Align2::RIGHT_BOTTOM,
        if ready { "READY  ›" } else { "OPEN  ›" },
        FontId::proportional(9.5),
        Color32::from_rgb(82, 169, 225),
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(1.0, Color32::from_rgb(41, 57, 71)),
        egui::StrokeKind::Inside,
    );
    response.clicked()
}

pub(super) fn compact_news_card(
    ui: &mut egui::Ui,
    category: &str,
    title: &str,
    detail: &str,
    enabled: bool,
) -> bool {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 97.0), sense);
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(5),
        if enabled && response.hovered() {
            Color32::from_rgb(24, 36, 48)
        } else {
            Color32::from_rgb(15, 23, 32)
        },
    );
    ui.painter().rect_filled(
        Rect::from_min_max(rect.min, Pos2::new(rect.left() + 4.0, rect.bottom())),
        CornerRadius::same(2),
        Color32::from_rgb(37, 134, 199),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 14.0, rect.top() + 12.0),
        Align2::LEFT_TOP,
        category,
        FontId::proportional(9.0),
        Color32::from_rgb(72, 163, 219),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 14.0, rect.top() + 34.0),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(13.5),
        Color32::from_rgb(210, 222, 232),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 14.0, rect.top() + 59.0),
        Align2::LEFT_TOP,
        detail,
        FontId::proportional(10.0),
        Color32::from_rgb(116, 139, 158),
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(5),
        Stroke::new(1.0, Color32::from_rgb(39, 53, 66)),
        egui::StrokeKind::Inside,
    );
    enabled && response.clicked()
}

pub(super) fn story_strip_card(
    ui: &mut egui::Ui,
    category: &str,
    title: &str,
    detail: &str,
) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 72.0), egui::Sense::click());
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(4),
        if response.hovered() {
            Color32::from_rgb(23, 35, 46)
        } else {
            Color32::from_rgb(14, 22, 30)
        },
    );
    ui.painter().text(
        Pos2::new(rect.left() + 12.0, rect.top() + 10.0),
        Align2::LEFT_TOP,
        category,
        FontId::proportional(8.5),
        Color32::from_rgb(73, 162, 218),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 12.0, rect.top() + 29.0),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(11.5),
        Color32::from_rgb(205, 218, 228),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 12.0, rect.top() + 49.0),
        Align2::LEFT_TOP,
        detail,
        FontId::proportional(9.0),
        Color32::from_rgb(110, 132, 150),
    );
    ui.painter().text(
        Pos2::new(rect.right() - 10.0, rect.center().y),
        Align2::RIGHT_CENTER,
        "›",
        FontId::proportional(17.0),
        Color32::from_rgb(74, 157, 211),
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0, Color32::from_rgb(36, 50, 63)),
        egui::StrokeKind::Inside,
    );
    response.clicked()
}

pub(super) fn activity_row(
    ui: &mut egui::Ui,
    item: &ActivityItem,
    reduced_motion: bool,
    full: bool,
) {
    let state = activity_visual_state(item);
    let now = ui.input(|input| input.time);
    let pulse = motion_amount(reduced_motion, now, 1.8);
    let progress = activity_progress_fraction(item, pulse);
    let status = match state {
        ActivityVisualState::Active => "ACTIVE",
        ActivityVisualState::Complete => "COMPLETE",
        ActivityVisualState::Failed => "FAILED",
    };
    let status_color = match state {
        ActivityVisualState::Active => Color32::from_rgb(70, 164, 224),
        ActivityVisualState::Complete => Color32::from_rgb(73, 194, 140),
        ActivityVisualState::Failed => Color32::from_rgb(221, 91, 91),
    };
    let width = if full {
        ui.available_width().min(720.0)
    } else {
        ui.available_width().min(570.0)
    };
    ui.group(|ui| {
        ui.set_min_width(width);
        ui.horizontal(|ui| {
            ui.label(RichText::new(status).small().strong().color(status_color));
            ui.label(RichText::new(&item.label).small().strong());
            ui.add_space((ui.available_width() - 200.0).max(0.0));
            ui.label(
                RichText::new(&item.detail)
                    .small()
                    .color(Color32::from_rgb(123, 145, 164)),
            );
        });
        let (bar, _) = ui.allocate_exact_size(Vec2::new(width - 18.0, 4.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(bar, CornerRadius::same(2), Color32::from_rgb(22, 30, 39));
        let filled = Rect::from_min_max(
            bar.min,
            Pos2::new(bar.left() + bar.width() * progress, bar.bottom()),
        );
        ui.painter()
            .rect_filled(filled, CornerRadius::same(2), status_color);
    });
}

pub(super) fn news_card(
    ui: &mut egui::Ui,
    category: &str,
    title: &str,
    detail: &str,
    enabled: bool,
) -> bool {
    let width = ui.available_width();
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 138.0), sense);
    let fill = if enabled && response.hovered() {
        Color32::from_rgb(26, 37, 49)
    } else {
        Color32::from_rgb(17, 24, 33)
    };
    ui.painter().rect_filled(rect, CornerRadius::same(5), fill);
    ui.painter().rect_filled(
        Rect::from_min_max(rect.min, Pos2::new(rect.right(), rect.top() + 34.0)),
        CornerRadius::same(5),
        Color32::from_rgb(20, 31, 42),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 14.0, rect.top() + 11.0),
        Align2::LEFT_TOP,
        category,
        FontId::proportional(10.0),
        Color32::from_rgb(79, 169, 226),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 14.0, rect.top() + 49.0),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(15.0),
        Color32::from_rgb(213, 224, 233),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 14.0, rect.top() + 78.0),
        Align2::LEFT_TOP,
        detail,
        FontId::proportional(11.0),
        Color32::from_rgb(121, 143, 162),
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(5),
        Stroke::new(1.0, Color32::from_rgb(42, 55, 68)),
        egui::StrokeKind::Inside,
    );
    enabled && response.clicked()
}

pub(super) fn paint_hero(ui: &mut egui::Ui, rect: Rect, animated: bool) {
    let painter = ui.painter();
    for band in 0..18 {
        let y0 = rect.top() + rect.height() * band as f32 / 18.0;
        let y1 = rect.top() + rect.height() * (band + 1) as f32 / 18.0 + 1.0;
        let fade = band as u8;
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(rect.left(), y0), Pos2::new(rect.right(), y1)),
            CornerRadius::ZERO,
            Color32::from_rgb(8 + fade / 2, 13 + fade / 2, 20 + fade),
        );
    }
    let t = if animated {
        ui.input(|input| input.time) as f32
    } else {
        0.0
    };
    let focus = Pos2::new(rect.right() - rect.width() * 0.28, rect.center().y - 20.0);
    for index in 0..10 {
        let radius = 46.0 + index as f32 * 34.0 + (t * 0.6 + index as f32).sin() * 3.0;
        painter.circle_stroke(
            focus,
            radius,
            Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(45, 125, 191, 25 + index as u8 * 3),
            ),
        );
    }
    for index in 0..54 {
        let phase = index as f32 * 1.731 + t * 0.14;
        let x = rect.left() + rect.width() * (0.56 + 0.38 * (phase.sin() * 0.5 + 0.5));
        let y = rect.top() + rect.height() * (0.12 + 0.76 * ((phase * 1.41).cos() * 0.5 + 0.5));
        let color = if index % 11 == 0 {
            Color32::from_rgba_unmultiplied(216, 135, 56, 105)
        } else {
            Color32::from_rgba_unmultiplied(74, 149, 215, 80)
        };
        painter.circle_filled(Pos2::new(x, y), 1.0 + (index % 3) as f32 * 0.4, color);
    }
    painter.rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(1.0, Color32::from_rgb(39, 55, 70)),
        egui::StrokeKind::Inside,
    );
}

pub(super) fn library_card(ui: &mut egui::Ui, title: &str, detail: &str, enabled: bool) -> bool {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 116.0), egui::Sense::click());
    let fill = if enabled {
        if response.hovered() {
            Color32::from_rgb(25, 48, 67)
        } else {
            Color32::from_rgb(20, 34, 47)
        }
    } else {
        Color32::from_rgb(14, 20, 28)
    };
    ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(6),
        Stroke::new(1.0, Color32::from_rgb(43, 58, 72)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 18.0, rect.top() + 24.0),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(17.0),
        if enabled {
            Color32::from_rgb(221, 230, 238)
        } else {
            Color32::from_rgb(113, 128, 143)
        },
    );
    ui.painter().text(
        Pos2::new(rect.left() + 18.0, rect.top() + 55.0),
        Align2::LEFT_TOP,
        detail,
        FontId::proportional(12.0),
        Color32::from_rgb(112, 135, 154),
    );
    if enabled {
        ui.painter().text(
            Pos2::new(rect.left() + 18.0, rect.bottom() - 18.0),
            Align2::LEFT_BOTTOM,
            "PLAYABLE",
            FontId::proportional(11.0),
            Color32::from_rgb(68, 169, 228),
        );
    }
    enabled && response.clicked()
}
pub(super) fn inventory_badge(ui: &mut egui::Ui, state: InventoryState) {
    let color = match state {
        InventoryState::Indexed => Color32::from_rgb(73, 194, 140),
        InventoryState::Stale | InventoryState::NotIndexed => Color32::from_rgb(221, 153, 65),
        InventoryState::Indexing => Color32::from_rgb(91, 190, 242),
        InventoryState::Failed => Color32::from_rgb(222, 91, 91),
    };
    ui.label(
        RichText::new(format!("● {}", state.label()))
            .strong()
            .color(color),
    );
}

pub(super) fn kind_rich_text(kind: StorageKind) -> RichText {
    let color = match kind {
        StorageKind::Archive => Color32::from_rgb(216, 153, 82),
        StorageKind::Index => Color32::from_rgb(91, 190, 242),
        StorageKind::Config => Color32::from_rgb(158, 133, 214),
        StorageKind::Other => Color32::from_rgb(148, 163, 177),
    };
    RichText::new(kind.label()).strong().color(color)
}

pub(super) fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let value = bytes as f64;
    if value >= GIB {
        format!("{:.2} GiB", value / GIB)
    } else if value >= MIB {
        format!("{:.1} MiB", value / MIB)
    } else if value >= KIB {
        format!("{:.1} KiB", value / KIB)
    } else {
        format!("{bytes} B")
    }
}

pub(super) fn compact_fingerprint(fingerprint: &str) -> &str {
    if fingerprint.len() > 20 {
        &fingerprint[..20]
    } else {
        fingerprint
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StatusTone {
    Good,
    Info,
    Warn,
    Error,
}

impl StatusTone {
    fn color(self) -> Color32 {
        match self {
            Self::Good => SUCCESS_GREEN,
            Self::Info => ACCENT_BLUE,
            Self::Warn => WARNING_AMBER,
            Self::Error => DANGER_RED,
        }
    }
}
pub(super) fn bridge_state_tone(state: BridgeState) -> StatusTone {
    match state {
        BridgeState::Ready | BridgeState::Running => StatusTone::Good,
        BridgeState::Installing
        | BridgeState::Updating
        | BridgeState::Verifying
        | BridgeState::Indexing
        | BridgeState::Starting => StatusTone::Info,
        BridgeState::MissingClient | BridgeState::NeedsSetup => StatusTone::Warn,
        BridgeState::BrokenInstall | BridgeState::Error => StatusTone::Error,
    }
}
pub(super) fn compact_status_chip(ui: &mut egui::Ui, label: &str, tone: StatusTone) {
    let text = label.to_ascii_uppercase();
    let width = (text.len() as f32 * 6.0 + 24.0).clamp(72.0, 180.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 22.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(3), Color32::from_rgb(13, 22, 30));
    ui.painter().circle_filled(
        Pos2::new(rect.left() + 9.0, rect.center().y),
        2.8,
        tone.color(),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 17.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(8.5),
        tone.color(),
    );
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(3),
        Stroke::new(1.0, Color32::from_rgb(35, 49, 61)),
        egui::StrokeKind::Inside,
    );
}
pub(super) fn state_caption(state: InstallState) -> &'static str {
    match state {
        InstallState::NotConfigured => "Game installation not configured",
        InstallState::Searching => "Scanning local installation metadata…",
        InstallState::FoundUnindexed => "Game found • content index required",
        InstallState::Indexing => "Indexing local content…",
        InstallState::Ready => "Local content inventory ready",
        InstallState::NeedsRepair => "Content needs attention",
        InstallState::UnsupportedBuild => "Build metadata not yet supported",
        InstallState::Error => "Launcher error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_tones_keep_ready_good_and_errors_red() {
        assert_eq!(bridge_state_tone(BridgeState::Ready), StatusTone::Good);
        assert_eq!(bridge_state_tone(BridgeState::Updating), StatusTone::Info);
        assert_eq!(bridge_state_tone(BridgeState::NeedsSetup), StatusTone::Warn);
        assert_eq!(bridge_state_tone(BridgeState::Error), StatusTone::Error);
    }
}
