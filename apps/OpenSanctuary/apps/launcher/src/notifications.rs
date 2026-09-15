use eframe::egui::{self, Color32, RichText};
use sanctuary_launcher::{ActivityItem, BridgeState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NotificationTone {
    Info,
    Success,
    Warning,
    Error,
}

impl NotificationTone {
    fn color(self) -> Color32 {
        match self {
            Self::Info => Color32::from_rgb(72, 164, 224),
            Self::Success => Color32::from_rgb(73, 194, 140),
            Self::Warning => Color32::from_rgb(221, 153, 65),
            Self::Error => Color32::from_rgb(221, 91, 91),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NotificationEntry {
    pub(super) category: &'static str,
    pub(super) title: String,
    pub(super) detail: String,
    pub(super) tone: NotificationTone,
}

pub(super) fn notification_entries(
    bridge_state: BridgeState,
    activity: &[ActivityItem],
) -> Vec<NotificationEntry> {
    let mut entries = Vec::new();
    match bridge_state {
        BridgeState::Installing => entries.push(NotificationEntry {
            category: "GAME",
            title: "Diablo III installation in progress".into(),
            detail: "Battle.net is managing the installation while OpenSanctuary supervises it."
                .into(),
            tone: NotificationTone::Info,
        }),
        BridgeState::Updating => entries.push(NotificationEntry {
            category: "GAME",
            title: "Diablo III update in progress".into(),
            detail: "The installed game is being updated through Battle.net.".into(),
            tone: NotificationTone::Info,
        }),
        BridgeState::Verifying => entries.push(NotificationEntry {
            category: "OPEN SANCTUARY",
            title: "Diablo III installation is being verified".into(),
            detail: "OpenSanctuary is validating the detected installation.".into(),
            tone: NotificationTone::Info,
        }),
        BridgeState::Indexing => entries.push(NotificationEntry {
            category: "OPEN SANCTUARY",
            title: "Diablo III content is being indexed".into(),
            detail: "Local content metadata is being refreshed.".into(),
            tone: NotificationTone::Info,
        }),
        BridgeState::Ready => entries.push(NotificationEntry {
            category: "GAME",
            title: "Diablo III is ready".into(),
            detail: "The current installation is available to launch.".into(),
            tone: NotificationTone::Success,
        }),
        BridgeState::BrokenInstall | BridgeState::Error => entries.push(NotificationEntry {
            category: "GAME",
            title: "Diablo III needs attention".into(),
            detail: "Review diagnostics or repair the Battle.net bridge before launching.".into(),
            tone: NotificationTone::Error,
        }),
        BridgeState::NeedsSetup | BridgeState::MissingClient => entries.push(NotificationEntry {
            category: "BATTLE.NET",
            title: "Battle.net setup required".into(),
            detail: "Open the integrated Battle.net surface to sign in or install Diablo III."
                .into(),
            tone: NotificationTone::Warning,
        }),
        BridgeState::Starting | BridgeState::Running => {}
    }

    for item in activity.iter().rev().take(5) {
        entries.push(NotificationEntry {
            category: "OPEN SANCTUARY",
            title: item.label.clone(),
            detail: item.detail.clone(),
            tone: if item.failed {
                NotificationTone::Error
            } else if item.done {
                NotificationTone::Success
            } else {
                NotificationTone::Info
            },
        });
    }
    entries
}

pub(super) fn render_notifications(ui: &mut egui::Ui, entries: &[NotificationEntry]) {
    if entries.is_empty() {
        ui.label(
            RichText::new("All caught up")
                .strong()
                .color(Color32::from_rgb(202, 216, 227)),
        );
        ui.label(
            RichText::new("No launcher notifications are available.")
                .small()
                .color(Color32::from_rgb(116, 139, 158)),
        );
        return;
    }

    for entry in entries {
        egui::Frame::new()
            .fill(Color32::from_rgb(12, 20, 29))
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("●").color(entry.tone.color()));
                    ui.label(
                        RichText::new(entry.category)
                            .small()
                            .strong()
                            .color(Color32::from_rgb(103, 135, 160)),
                    );
                });
                ui.label(RichText::new(&entry.title).strong());
                ui.label(
                    RichText::new(&entry.detail)
                        .small()
                        .color(Color32::from_rgb(127, 149, 167)),
                );
            });
        ui.add_space(7.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_entries_include_active_game_work() {
        let entries = notification_entries(BridgeState::Indexing, &[]);
        assert!(
            entries
                .iter()
                .any(|entry| entry.title.contains("Diablo III"))
        );
    }

    #[test]
    fn notifications_do_not_fabricate_social_data() {
        let entries = notification_entries(BridgeState::Ready, &[]);
        assert!(entries.iter().all(|entry| !entry.title.contains("friend")));
    }
}
