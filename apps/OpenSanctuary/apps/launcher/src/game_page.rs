use crate::widgets::paint_hero;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Stroke, Vec2,
};

const PLAYER_STATUS_LABELS: [&str; 3] = ["Battle.net", "Installation", "Content"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GamePageAction {
    None,
    Primary,
    Settings,
    Details,
    OpenBattleNet,
    Content,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GameReadiness {
    Ready,
    Working,
    NeedsAttention,
}

impl GameReadiness {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Ready => "READY TO PLAY",
            Self::Working => "PREPARING GAME",
            Self::NeedsAttention => "ATTENTION REQUIRED",
        }
    }

    fn color(self) -> Color32 {
        match self {
            Self::Ready => Color32::from_rgb(70, 190, 132),
            Self::Working => Color32::from_rgb(70, 157, 218),
            Self::NeedsAttention => Color32::from_rgb(221, 153, 65),
        }
    }
}

pub(super) struct GamePageView {
    pub(super) primary_label: String,
    pub(super) primary_enabled: bool,
    pub(super) readiness: GameReadiness,
    pub(super) build_version: String,
    pub(super) install_caption: String,
    pub(super) battle_net_caption: String,
    pub(super) content_caption: String,
    pub(super) message: String,
    pub(super) session_status: String,
    pub(super) reduced_motion: bool,
}

pub(super) fn render_game_hero(ui: &mut egui::Ui, view: &GamePageView) -> GamePageAction {
    let width = ui.available_width().max(720.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 320.0), egui::Sense::hover());
    paint_hero(ui, rect, !view.reduced_motion);

    let shade = Rect::from_min_max(
        rect.min,
        Pos2::new(rect.left() + rect.width() * 0.62, rect.bottom()),
    );
    ui.painter().rect_filled(
        shade,
        CornerRadius::same(8),
        Color32::from_rgba_unmultiplied(4, 8, 13, 214),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 34.0, rect.top() + 34.0),
        Align2::LEFT_TOP,
        "DIABLO III",
        FontId::proportional(34.0),
        Color32::from_rgb(239, 242, 245),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 36.0, rect.top() + 78.0),
        Align2::LEFT_TOP,
        "REAPER OF SOULS",
        FontId::proportional(13.0),
        Color32::from_rgb(177, 196, 211),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 36.0, rect.top() + 111.0),
        Align2::LEFT_TOP,
        format!("Local • {}", view.build_version),
        FontId::proportional(11.0),
        Color32::from_rgb(111, 139, 161),
    );

    let readiness_y = rect.bottom() - 110.0;
    ui.painter().circle_filled(
        Pos2::new(rect.left() + 39.0, readiness_y),
        4.0,
        view.readiness.color(),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 52.0, readiness_y),
        Align2::LEFT_CENTER,
        view.readiness.label(),
        FontId::proportional(11.0),
        view.readiness.color(),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 36.0, readiness_y + 20.0),
        Align2::LEFT_CENTER,
        &view.session_status,
        FontId::proportional(12.0),
        Color32::from_rgb(168, 188, 204),
    );

    let primary_rect = Rect::from_min_size(
        Pos2::new(rect.left() + 34.0, rect.bottom() - 72.0),
        Vec2::new(246.0, 48.0),
    );
    let mut action = GamePageAction::None;
    let primary = ui.put(
        primary_rect,
        egui::Button::new(RichText::new(&view.primary_label).strong().size(16.0))
            .fill(if view.primary_enabled {
                Color32::from_rgb(20, 112, 187)
            } else {
                Color32::from_rgb(36, 47, 58)
            })
            .sense(if view.primary_enabled {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            }),
    );
    if view.primary_enabled && primary.clicked() {
        action = GamePageAction::Primary;
    }

    let settings_rect = Rect::from_min_size(
        Pos2::new(primary_rect.right() + 9.0, primary_rect.top()),
        Vec2::new(46.0, 48.0),
    );
    if ui
        .put(settings_rect, egui::Button::new("⚙"))
        .on_hover_text("Game settings")
        .clicked()
    {
        action = GamePageAction::Settings;
    }
    let details_rect = Rect::from_min_size(
        Pos2::new(settings_rect.right() + 8.0, settings_rect.top()),
        Vec2::new(46.0, 48.0),
    );
    if ui
        .put(details_rect, egui::Button::new("⋮"))
        .on_hover_text("Game options and diagnostics")
        .clicked()
    {
        action = GamePageAction::Details;
    }

    action
}

pub(super) fn render_game_status(ui: &mut egui::Ui, view: &GamePageView) -> GamePageAction {
    let mut action = GamePageAction::None;
    egui::Frame::new()
        .fill(Color32::from_rgb(10, 18, 27))
        .stroke(Stroke::new(1.0, Color32::from_rgb(32, 55, 73)))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_min_width(260.0);
            ui.label(
                RichText::new("GAME STATUS")
                    .small()
                    .strong()
                    .color(Color32::from_rgb(113, 145, 169)),
            );
            ui.add_space(7.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("●").color(view.readiness.color()));
                ui.label(RichText::new(view.readiness.label()).strong());
            });
            ui.add_space(10.0);
            status_row(ui, PLAYER_STATUS_LABELS[0], &view.battle_net_caption);
            status_row(ui, PLAYER_STATUS_LABELS[1], &view.install_caption);
            status_row(ui, PLAYER_STATUS_LABELS[2], &view.content_caption);
            ui.add_space(10.0);
            ui.label(
                RichText::new(&view.message)
                    .small()
                    .color(Color32::from_rgb(109, 134, 154)),
            );
            ui.add_space(10.0);
            if ui.button("DETAILS").clicked() {
                action = GamePageAction::Details;
            }
            ui.horizontal(|ui| {
                if ui.button("BATTLE.NET").clicked() {
                    action = GamePageAction::OpenBattleNet;
                }
                if ui.button("CONTENT").clicked() {
                    action = GamePageAction::Content;
                }
            });
        });
    action
}

fn status_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label)
                .small()
                .color(Color32::from_rgb(116, 139, 157)),
        );
        ui.add_space((ui.available_width() - 132.0).max(0.0));
        ui.label(RichText::new(value).small().strong());
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_status_hides_advanced_bridge_details() {
        assert!(PLAYER_STATUS_LABELS.contains(&"Battle.net"));
        assert!(PLAYER_STATUS_LABELS.contains(&"Installation"));
        assert!(PLAYER_STATUS_LABELS.contains(&"Content"));
        assert!(!PLAYER_STATUS_LABELS.contains(&"Runner"));
        assert!(!PLAYER_STATUS_LABELS.contains(&"Prefix"));
    }

    #[test]
    fn readiness_labels_are_player_facing() {
        assert_eq!(GameReadiness::Ready.label(), "READY TO PLAY");
        assert_eq!(GameReadiness::Working.label(), "PREPARING GAME");
    }
}
