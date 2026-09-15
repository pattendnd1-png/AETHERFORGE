use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};
use sanctuary_battlenet::{AccountProfile, DesktopSessionState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountPageAction {
    None,
    SaveProfile,
    SignIn,
    OpenBattleNet,
    ManageAccount,
    Diagnostics,
}

pub struct AccountPageView<'a> {
    pub session: DesktopSessionState,
    pub game_access: &'a str,
    pub game_status: &'a str,
    pub bridge_health: &'a str,
    pub oauth_client_configured: bool,
}

pub fn render_account_page(
    ui: &mut egui::Ui,
    profile: &mut AccountProfile,
    view: AccountPageView<'_>,
) -> AccountPageAction {
    let mut action = AccountPageAction::None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(28.0);
            ui.horizontal(|ui| {
                ui.add_space(30.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("BATTLE.NET ACCOUNT")
                            .size(30.0)
                            .strong()
                            .color(Color32::from_rgb(233, 239, 244)),
                    );
                    ui.label(
                        RichText::new("Identity and desktop-session status for the integrated Battle.net workflow")
                            .color(Color32::from_rgb(119, 146, 167)),
                    );
                });
            });
            ui.add_space(22.0);

            egui::Frame::new()
                .fill(Color32::from_rgb(10, 18, 27))
                .stroke(Stroke::new(1.0, Color32::from_rgb(31, 55, 74)))
                .corner_radius(CornerRadius::same(7))
                .inner_margin(egui::Margin::same(18))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width().min(900.0));
                    ui.label(
                        RichText::new(profile.battletag.as_deref().unwrap_or("Battle.net account"))
                            .size(22.0)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(profile.region.to_ascii_uppercase())
                            .small()
                            .color(Color32::from_rgb(109, 137, 158)),
                    );
                    ui.add_space(12.0);
                    status_row(ui, "BATTLE.NET SESSION", view.session.label(), view.session.interactive());
                    status_row(ui, "GAME ACCESS", view.game_access, view.game_access != "Not installed");
                    status_row(ui, "DIABLO III", view.game_status, view.game_status == "Ready");
                });

            ui.add_space(16.0);
            egui::Frame::new()
                .fill(Color32::from_rgb(8, 14, 21))
                .stroke(Stroke::new(1.0, Color32::from_rgb(27, 45, 60)))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::same(14))
                .show(ui, |ui| {
                    ui.label(RichText::new("ACCOUNT HINT").small().strong());
                    ui.label(
                        RichText::new("Optional non-secret account identification used by OpenSanctuary only.")
                            .small()
                            .color(Color32::from_rgb(113, 136, 155)),
                    );
                    ui.add_space(8.0);
                    let email = profile.email_hint.get_or_insert_default();
                    ui.horizontal(|ui| {
                        ui.label("Email hint");
                        ui.add_sized(
                            [350.0, 30.0],
                            egui::TextEdit::singleline(email).hint_text("name@example.com"),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Region");
                        egui::ComboBox::from_id_salt("battle_net_region")
                            .selected_text(profile.region.to_ascii_uppercase())
                            .show_ui(ui, |ui| {
                                for region in ["us", "eu", "kr", "tw"] {
                                    ui.selectable_value(
                                        &mut profile.region,
                                        region.to_string(),
                                        region.to_ascii_uppercase(),
                                    );
                                }
                            });
                    });
                    if ui.button("SAVE ACCOUNT HINT").clicked() {
                        action = AccountPageAction::SaveProfile;
                    }
                });

            ui.add_space(16.0);
            ui.horizontal(|ui| {
                if ui
                    .add_sized(
                        [224.0, 42.0],
                        egui::Button::new(RichText::new("SIGN IN WITH BATTLE.NET").strong())
                            .fill(Color32::from_rgb(22, 111, 183)),
                    )
                    .clicked()
                {
                    action = AccountPageAction::SignIn;
                }
                if ui.button("OPEN BATTLE.NET").clicked() {
                    action = AccountPageAction::OpenBattleNet;
                }
                if ui.button("MANAGE ACCOUNT").clicked() {
                    action = AccountPageAction::ManageAccount;
                }
            });

            ui.add_space(18.0);
            ui.collapsing("Advanced session details", |ui| {
                ui.label(format!("Bridge health: {}", view.bridge_health));
                ui.label(if view.oauth_client_configured {
                    "Developer OAuth client: configured"
                } else {
                    "Developer OAuth client: optional / not configured"
                });
                if ui.button("OPEN DIAGNOSTICS").clicked() {
                    action = AccountPageAction::Diagnostics;
                }
            });
            ui.add_space(12.0);
            ui.label(
                RichText::new(
                    "Passwords, authenticator codes, recovery codes, and captcha responses are never stored by OpenSanctuary.",
                )
                .small()
                .color(Color32::from_rgb(105, 130, 150)),
            );
        });
    action
}

fn status_row(ui: &mut egui::Ui, label: &str, value: &str, good: bool) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label)
                .small()
                .strong()
                .color(Color32::from_rgb(107, 137, 160)),
        );
        ui.add_space((ui.available_width() - 220.0).max(0.0));
        ui.label(RichText::new("●").color(if good {
            Color32::from_rgb(73, 194, 140)
        } else {
            Color32::from_rgb(144, 162, 177)
        }));
        ui.label(RichText::new(value).strong());
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn account_page_never_enables_password_capture() {
        let source = include_str!("account_page.rs");
        let forbidden = [".password", "(true)"].concat();
        assert!(!source.contains(&forbidden));
    }
}
