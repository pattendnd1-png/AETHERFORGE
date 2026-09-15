use eframe::egui::{self, Color32, RichText};
use sanctuary_launcher::{ActivityItem, BridgeState, InventoryState};

use crate::widgets::{activity_row, activity_status_icon, bridge_state_tone, compact_status_chip};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DownloadsAction {
    None,
    Toggle,
    ViewAll,
    Details,
}

pub(super) struct DownloadsView<'a> {
    pub(super) bridge_state: BridgeState,
    pub(super) inventory_state: InventoryState,
    pub(super) activity: &'a [ActivityItem],
    pub(super) reduced_motion: bool,
    pub(super) expanded: bool,
}

pub(super) fn downloads_summary(
    bridge_state: BridgeState,
    inventory_state: InventoryState,
    activity: &[ActivityItem],
) -> String {
    let lifecycle = match bridge_state {
        BridgeState::Installing => Some("Diablo III • Installing"),
        BridgeState::Updating => Some("Diablo III • Updating"),
        BridgeState::Verifying => Some("Diablo III • Verifying"),
        BridgeState::Indexing => Some("Diablo III • Indexing"),
        BridgeState::Starting => Some("Diablo III • Starting"),
        BridgeState::Running => Some("Diablo III • Playing"),
        _ => None,
    };
    if let Some(summary) = lifecycle {
        return summary.to_owned();
    }
    let active = activity.iter().filter(|item| !item.done).count();
    if active > 0 {
        return format!(
            "{active} launcher task{} active",
            if active == 1 { "" } else { "s" }
        );
    }
    if inventory_state == InventoryState::Indexed {
        "No active downloads • Diablo III current".to_owned()
    } else {
        "No active downloads".to_owned()
    }
}

pub(super) fn render_downloads_tray(ui: &mut egui::Ui, view: DownloadsView<'_>) -> DownloadsAction {
    let mut action = DownloadsAction::None;
    let active = view.activity.iter().filter(|item| !item.done).count();
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        activity_status_icon(ui, active, view.reduced_motion);
        ui.label(
            RichText::new("DOWNLOADS")
                .small()
                .strong()
                .color(Color32::from_rgb(186, 202, 215)),
        );
        if active > 0 {
            ui.label(
                RichText::new(format!("{active} ACTIVE"))
                    .small()
                    .strong()
                    .color(Color32::from_rgb(67, 163, 226)),
            );
        }
        ui.add_space(7.0);
        compact_status_chip(
            ui,
            view.bridge_state.label(),
            bridge_state_tone(view.bridge_state),
        );
        ui.add_space(7.0);
        ui.label(
            RichText::new(downloads_summary(
                view.bridge_state,
                view.inventory_state,
                view.activity,
            ))
            .small()
            .color(Color32::from_rgb(116, 141, 161)),
        );
        ui.add_space((ui.available_width() - 144.0).max(0.0));
        if ui
            .add(egui::Button::new(RichText::new("VIEW ALL").small()).frame(false))
            .clicked()
        {
            action = DownloadsAction::ViewAll;
        }
        if ui
            .add(egui::Button::new(if view.expanded { "▾" } else { "▴" }).frame(false))
            .clicked()
        {
            action = DownloadsAction::Toggle;
        }
        ui.add_space(6.0);
    });

    if view.expanded {
        ui.separator();
        ui.horizontal(|ui| {
            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("DIABLO III")
                        .strong()
                        .color(Color32::from_rgb(209, 221, 231)),
                );
                ui.label(
                    RichText::new(view.bridge_state.label())
                        .small()
                        .color(Color32::from_rgb(100, 139, 167)),
                );
            });
            ui.add_space((ui.available_width() - 98.0).max(0.0));
            if ui.button("DETAILS").clicked() {
                action = DownloadsAction::Details;
            }
        });
        ui.add_space(6.0);
        if view.activity.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(14.0);
                ui.label(
                    RichText::new("No active launcher tasks")
                        .small()
                        .color(Color32::from_rgb(112, 136, 155)),
                );
            });
        }
        for item in view.activity.iter().rev().take(6) {
            activity_row(ui, item, view.reduced_motion, false);
        }
    }

    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_bridge_work_beats_idle_summary() {
        let summary = downloads_summary(BridgeState::Updating, InventoryState::Indexed, &[]);
        assert_eq!(summary, "Diablo III • Updating");
    }

    #[test]
    fn indexed_idle_game_reports_current() {
        let summary = downloads_summary(BridgeState::Ready, InventoryState::Indexed, &[]);
        assert!(summary.contains("current"));
    }
}
