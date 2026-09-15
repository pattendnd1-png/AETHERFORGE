use std::path::Path;

use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};
use sanctuary_battlenet::NetworkBridgeReport;

const ADVANCED_LABELS: [&str; 8] = [
    "Bridge",
    "Runner",
    "Battle.net",
    "Agent",
    "Installation",
    "Content index",
    "Last recovery",
    "Prefix",
];

pub(super) struct AdvancedDiagnosticsView<'a> {
    pub(super) bridge_health: &'a str,
    pub(super) bridge_state: &'a str,
    pub(super) runner: Option<&'a Path>,
    pub(super) prefix: Option<&'a Path>,
    pub(super) launcher: Option<&'a Path>,
    pub(super) install: Option<&'a Path>,
    pub(super) battlenet_running: bool,
    pub(super) agent_running: bool,
    pub(super) content_state: &'a str,
    pub(super) last_recovery: Option<&'a str>,
    pub(super) network_report: &'a NetworkBridgeReport,
}

pub(super) fn render_advanced_diagnostics(ui: &mut egui::Ui, view: AdvancedDiagnosticsView<'_>) {
    egui::Frame::new()
        .fill(Color32::from_rgb(9, 16, 24))
        .stroke(Stroke::new(1.0, Color32::from_rgb(30, 51, 68)))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.label(
                RichText::new("ADVANCED BRIDGE STATUS")
                    .small()
                    .strong()
                    .color(Color32::from_rgb(108, 141, 166)),
            );
            ui.add_space(8.0);
            diagnostic_row(
                ui,
                ADVANCED_LABELS[0],
                format!("{} • {}", view.bridge_health, view.bridge_state),
            );
            diagnostic_row(ui, ADVANCED_LABELS[1], path_label(view.runner));
            diagnostic_row(ui, ADVANCED_LABELS[7], path_label(view.prefix));
            diagnostic_row(
                ui,
                ADVANCED_LABELS[2],
                if view.battlenet_running {
                    "Running".to_owned()
                } else {
                    path_label(view.launcher)
                },
            );
            diagnostic_row(
                ui,
                ADVANCED_LABELS[3],
                if view.agent_running {
                    "Running"
                } else {
                    "Idle"
                }
                .to_owned(),
            );
            diagnostic_row(ui, ADVANCED_LABELS[4], path_label(view.install));
            diagnostic_row(ui, ADVANCED_LABELS[5], view.content_state.to_owned());
            diagnostic_row(
                ui,
                ADVANCED_LABELS[6],
                view.last_recovery
                    .unwrap_or("No recovery required")
                    .to_owned(),
            );
            ui.add_space(10.0);
            ui.label(
                RichText::new("NETWORK BRIDGE")
                    .small()
                    .strong()
                    .color(Color32::from_rgb(108, 141, 166)),
            );
            diagnostic_row(
                ui,
                "State",
                format!(
                    "{} • {}",
                    view.network_report.state.label(),
                    view.network_report.summary
                ),
            );
            for check in &view.network_report.checks {
                let latency = check
                    .latency_ms
                    .map_or_else(String::new, |ms| format!(" • {ms} ms"));
                diagnostic_row(
                    ui,
                    &format!("{} {}", check.host, check.kind.label()),
                    format!(
                        "{} • {}{}",
                        if check.ok { "OK" } else { "FAIL" },
                        check.detail,
                        latency
                    ),
                );
            }
        });
}

fn diagnostic_row(ui: &mut egui::Ui, label: &str, value: String) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label)
                .small()
                .color(Color32::from_rgb(111, 137, 158)),
        );
        ui.add_space((ui.available_width() - 410.0).max(0.0));
        ui.label(
            RichText::new(value)
                .small()
                .monospace()
                .color(Color32::from_rgb(181, 198, 211)),
        );
    });
}

fn path_label(path: Option<&Path>) -> String {
    path.map_or_else(
        || "Not available".to_owned(),
        |value| value.display().to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advanced_diagnostics_keeps_bridge_fields() {
        assert!(ADVANCED_LABELS.contains(&"Bridge"));
        assert!(ADVANCED_LABELS.contains(&"Runner"));
        assert!(ADVANCED_LABELS.contains(&"Content index"));
    }
}
