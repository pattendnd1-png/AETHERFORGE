use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Stroke, Vec2,
};

use crate::widgets::{
    account_top_button, game_strip_placeholder, game_strip_tile, last_played_tile,
    native_status_badge, notification_badge_button, utility_top_button,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TopBarAction {
    None,
    Home,
    Games,
    BattleNet,
    Shop,
    Notifications,
    Downloads,
    Account,
    Search(String),
}

#[derive(Debug, Clone, Copy)]
pub(super) struct TopBarView {
    pub(super) home_active: bool,
    pub(super) games_active: bool,
    pub(super) battlenet_active: bool,
    pub(super) shop_active: bool,
    pub(super) notifications_active: bool,
    pub(super) downloads_active: bool,
    pub(super) account_active: bool,
    pub(super) native_ready: bool,
    pub(super) notification_count: usize,
    pub(super) active_downloads: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GameStripAction {
    None,
    Diablo,
    Games,
}

pub(super) struct GameStripView<'a> {
    pub(super) diablo_selected: bool,
    pub(super) last_played_detail: &'a str,
    pub(super) launcher_scope: bool,
    pub(super) reduced_motion: bool,
}

pub(super) fn render_top_bar(
    ui: &mut egui::Ui,
    search_text: &mut String,
    view: TopBarView,
) -> TopBarAction {
    let mut action = TopBarAction::None;
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        let (brand, response) =
            ui.allocate_exact_size(Vec2::new(164.0, 34.0), egui::Sense::click());
        if response.hovered() {
            ui.painter()
                .rect_filled(brand, CornerRadius::same(4), Color32::from_rgb(13, 27, 40));
        }
        let mark = Pos2::new(brand.left() + 17.0, brand.center().y);
        ui.painter()
            .circle_filled(mark, 12.0, Color32::from_rgb(15, 83, 145));
        ui.painter().circle_stroke(
            mark,
            12.0,
            Stroke::new(1.0, Color32::from_rgb(57, 153, 220)),
        );
        ui.painter().text(
            mark,
            Align2::CENTER_CENTER,
            "OS",
            FontId::proportional(8.5),
            Color32::WHITE,
        );
        ui.painter().text(
            Pos2::new(brand.left() + 38.0, brand.center().y),
            Align2::LEFT_CENTER,
            "OpenSanctuary",
            FontId::proportional(13.0),
            Color32::from_rgb(224, 233, 240),
        );
        if response.clicked() {
            action = TopBarAction::Home;
        }

        ui.add_space(10.0);
        top_nav(
            ui,
            "HOME",
            view.home_active,
            &mut action,
            TopBarAction::Home,
        );
        top_nav(
            ui,
            "GAMES",
            view.games_active,
            &mut action,
            TopBarAction::Games,
        );
        top_nav(
            ui,
            "BATTLE.NET",
            view.battlenet_active,
            &mut action,
            TopBarAction::BattleNet,
        );
        top_nav(
            ui,
            "SHOP",
            view.shop_active,
            &mut action,
            TopBarAction::Shop,
        );

        ui.add_space((ui.available_width() - 510.0).max(0.0));
        let search = ui.add_sized(
            [150.0, 28.0],
            egui::TextEdit::singleline(search_text)
                .hint_text("Search")
                .margin(egui::Margin::symmetric(8, 5)),
        );
        if search.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
            action = TopBarAction::Search(search_text.trim().to_owned());
        }

        ui.add_space(4.0);
        native_status_badge(ui, view.native_ready);
        if notification_badge_button(ui, view.notification_count, view.notifications_active) {
            action = TopBarAction::Notifications;
        }
        let downloads_label = if view.active_downloads > 0 {
            format!("↓ {}", view.active_downloads)
        } else {
            "↓".to_owned()
        };
        if utility_top_button(ui, &downloads_label, view.downloads_active) {
            action = TopBarAction::Downloads;
        }
        if account_top_button(ui, view.account_active) {
            action = TopBarAction::Account;
        }
    });
    action
}

fn top_nav(
    ui: &mut egui::Ui,
    label: &str,
    active: bool,
    output: &mut TopBarAction,
    action: TopBarAction,
) {
    let text = RichText::new(label).strong().color(if active {
        Color32::WHITE
    } else {
        Color32::from_rgb(132, 149, 165)
    });
    let response = ui.add_sized([82.0, 34.0], egui::Button::new(text).frame(false));
    if active || response.hovered() {
        ui.painter().rect_filled(
            Rect::from_min_max(
                Pos2::new(response.rect.left() + 10.0, response.rect.bottom() - 1.0),
                Pos2::new(response.rect.right() - 10.0, response.rect.bottom() + 1.0),
            ),
            CornerRadius::ZERO,
            if active {
                Color32::from_rgb(44, 157, 225)
            } else {
                Color32::from_rgb(57, 99, 131)
            },
        );
    }
    if response.clicked() {
        *output = action;
    }
}

pub(super) fn render_game_strip(ui: &mut egui::Ui, view: GameStripView<'_>) -> GameStripAction {
    let mut action = GameStripAction::None;
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        ui.label(
            RichText::new("FAVORITES")
                .small()
                .strong()
                .color(Color32::from_rgb(91, 116, 137)),
        );
        ui.add_space(8.0);
        if game_strip_tile(ui, "Diablo III", view.diablo_selected, view.reduced_motion) {
            action = GameStripAction::Diablo;
        }
        game_strip_placeholder(ui, "BN", "Battle.net");
        game_strip_placeholder(ui, "+", "Add game");
        ui.add_space(14.0);
        ui.separator();
        ui.add_space(12.0);
        ui.label(
            RichText::new("LAST PLAYED")
                .small()
                .strong()
                .color(Color32::from_rgb(91, 116, 137)),
        );
        if last_played_tile(
            ui,
            "DIABLO III",
            view.last_played_detail,
            view.reduced_motion,
        ) {
            action = GameStripAction::Diablo;
        }
        ui.add_space((ui.available_width() - 92.0).max(0.0));
        let color = if view.launcher_scope {
            Color32::from_rgb(174, 194, 210)
        } else {
            Color32::from_rgb(108, 130, 149)
        };
        if ui
            .add(
                egui::Button::new(RichText::new("ALL GAMES").small().strong().color(color))
                    .frame(false),
            )
            .clicked()
        {
            action = GameStripAction::Games;
        }
        ui.add_space(8.0);
    });
    action
}
