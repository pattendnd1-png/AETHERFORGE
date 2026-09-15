use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};
use sanctuary_battlenet::{DesktopSessionState, HostStatus, WindowHostMode};

const PRIMARY_SURFACE_LABELS: [&str; 3] = ["ACCOUNT", "AGENT", "DIABLO III"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleNetPageAction {
    None,
    Primary,
    OpenClient,
    SignIn,
    Account,
    GamePage,
    RepairBridge,
    RepairNetworkBridge,
}

pub struct BattleNetPageView<'a> {
    pub account_label: &'a str,
    pub session: DesktopSessionState,
    pub bridge_health: &'a str,
    pub bridge_state: &'a str,
    pub bridge_detail: &'a str,
    pub battlenet_running: bool,
    pub agent_running: bool,
    pub diablo_running: bool,
    pub install_state: &'a str,
    pub content_state: &'a str,
    pub primary_label: &'a str,
    pub primary_enabled: bool,
    pub repair_recommended: bool,
    pub network_state: &'a str,
    pub network_summary: &'a str,
    pub network_repair_recommended: bool,
    pub network_probe_in_flight: bool,
    pub host_status: &'a HostStatus,
}

pub struct BattleNetPageOutput {
    pub action: BattleNetPageAction,
    pub host_rect: egui::Rect,
}

pub fn render_battlenet_page(
    ui: &mut egui::Ui,
    view: BattleNetPageView<'_>,
) -> BattleNetPageOutput {
    let mut action = BattleNetPageAction::None;
    let mut host_rect = egui::Rect::NOTHING;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(22.0);
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("BATTLE.NET")
                            .size(30.0)
                            .strong()
                            .color(Color32::from_rgb(233, 239, 244)),
                    );
                    ui.label(
                        RichText::new("Integrated authentication, installation, updates, and official-client handoff")
                            .color(Color32::from_rgb(116, 144, 165)),
                    );
                });
                ui.add_space((ui.available_width() - 270.0).max(0.0));
                if ui.button("ACCOUNT").clicked() {
                    action = BattleNetPageAction::Account;
                }
                if ui.button("DIABLO III").clicked() {
                    action = BattleNetPageAction::GamePage;
                }
            });

            ui.add_space(16.0);
            ui.columns(4, |columns| {
                compact_status(
                    &mut columns[0],
                    PRIMARY_SURFACE_LABELS[0],
                    view.account_label,
                    view.session.label(),
                    view.session.interactive(),
                );
                compact_status(
                    &mut columns[1],
                    PRIMARY_SURFACE_LABELS[1],
                    if view.agent_running { "Ready" } else { "Idle" },
                    view.bridge_health,
                    view.agent_running || view.battlenet_running,
                );
                compact_status(
                    &mut columns[2],
                    PRIMARY_SURFACE_LABELS[2],
                    view.install_state,
                    view.content_state,
                    view.diablo_running || view.install_state.eq_ignore_ascii_case("Ready"),
                );
                compact_status(
                    &mut columns[3],
                    "NETWORK",
                    view.network_state,
                    view.network_summary,
                    view.network_state.eq_ignore_ascii_case("Online"),
                );
            });

            ui.add_space(14.0);
            let width = ui.available_width().max(520.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 520.0), egui::Sense::hover());
            host_rect = rect;
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(7),
                Color32::from_rgb(4, 9, 15),
            );
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(7),
                Stroke::new(1.0, Color32::from_rgb(34, 58, 77)),
                egui::StrokeKind::Inside,
            );

            if !view.host_status.embedded {
                let title = match view.host_status.mode {
                    WindowHostMode::X11Embedded => "WAITING FOR BATTLE.NET WINDOW",
                    WindowHostMode::WaylandCompanion => "BATTLE.NET COMPANION SURFACE",
                    WindowHostMode::Companion => "BATTLE.NET MANAGED SURFACE",
                };
                ui.painter().text(
                    rect.center() - Vec2::new(0.0, 24.0),
                    egui::Align2::CENTER_CENTER,
                    title,
                    egui::FontId::proportional(20.0),
                    Color32::from_rgb(199, 215, 227),
                );
                ui.painter().text(
                    rect.center() + Vec2::new(0.0, 8.0),
                    egui::Align2::CENTER_CENTER,
                    &view.host_status.detail,
                    egui::FontId::proportional(12.0),
                    Color32::from_rgb(113, 139, 160),
                );
                ui.painter().text(
                    rect.center() + Vec2::new(0.0, 30.0),
                    egui::Align2::CENTER_CENTER,
                    "Blizzard-controlled login/install interaction appears here or in the managed companion window.",
                    egui::FontId::proportional(11.0),
                    Color32::from_rgb(95, 118, 137),
                );
            }

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        view.primary_enabled,
                        egui::Button::new(RichText::new(view.primary_label).strong().size(15.0))
                            .fill(Color32::from_rgb(21, 111, 182))
                            .min_size(Vec2::new(220.0, 42.0)),
                    )
                    .clicked()
                {
                    action = BattleNetPageAction::Primary;
                }
                if ui.button("OPEN BATTLE.NET").clicked() {
                    action = BattleNetPageAction::OpenClient;
                }
                if !view.session.interactive() && ui.button("SIGN IN").clicked() {
                    action = BattleNetPageAction::SignIn;
                }
                if view.repair_recommended
                    && ui.button("REPAIR SOFTWARE BRIDGE").clicked()
                {
                    action = BattleNetPageAction::RepairBridge;
                }
                if (view.network_repair_recommended || view.network_probe_in_flight)
                    && ui
                        .add_enabled(
                            !view.network_probe_in_flight,
                            egui::Button::new(if view.network_probe_in_flight {
                                "CHECKING NETWORK…"
                            } else {
                                "REPAIR NETWORK BRIDGE"
                            }),
                        )
                        .clicked()
                {
                    action = BattleNetPageAction::RepairNetworkBridge;
                }
                ui.add_space((ui.available_width() - 220.0).max(0.0));
                ui.label(
                    RichText::new(format!("{} • {}", view.bridge_state, view.bridge_health))
                        .small()
                        .color(Color32::from_rgb(107, 136, 158)),
                );
            });

            ui.collapsing("More", |ui| {
                ui.label(
                    RichText::new(view.bridge_detail)
                        .small()
                        .color(Color32::from_rgb(113, 137, 157)),
                );
                ui.label(
                    RichText::new(format!("Network: {} — {}", view.network_state, view.network_summary))
                        .small()
                        .color(Color32::from_rgb(113, 137, 157)),
                );
                if ui.button("REPAIR SOFTWARE BRIDGE").clicked() {
                    action = BattleNetPageAction::RepairBridge;
                }
                if ui
                    .add_enabled(
                        !view.network_probe_in_flight,
                        egui::Button::new("REPAIR NETWORK BRIDGE"),
                    )
                    .clicked()
                {
                    action = BattleNetPageAction::RepairNetworkBridge;
                }
            });
        });

    BattleNetPageOutput { action, host_rect }
}

fn compact_status(ui: &mut egui::Ui, label: &str, primary: &str, secondary: &str, good: bool) {
    egui::Frame::new()
        .fill(Color32::from_rgb(10, 18, 27))
        .stroke(Stroke::new(1.0, Color32::from_rgb(30, 52, 69)))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(egui::Margin::same(11))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(label)
                        .small()
                        .strong()
                        .color(Color32::from_rgb(103, 135, 159)),
                );
                ui.add_space((ui.available_width() - 28.0).max(0.0));
                ui.label(RichText::new("●").color(if good {
                    Color32::from_rgb(73, 194, 140)
                } else {
                    Color32::from_rgb(135, 154, 170)
                }));
            });
            ui.label(RichText::new(primary).strong().size(15.0));
            ui.label(
                RichText::new(secondary)
                    .small()
                    .color(Color32::from_rgb(111, 136, 156)),
            );
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_surface_labels_do_not_expose_runner_or_prefix() {
        assert!(!PRIMARY_SURFACE_LABELS.contains(&"RUNNER"));
        assert!(!PRIMARY_SURFACE_LABELS.contains(&"PREFIX"));
        assert!(PRIMARY_SURFACE_LABELS.contains(&"DIABLO III"));
    }
}
