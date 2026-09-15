use crate::{theme, widgets};
use eframe::egui::{self, RichText};
use forgehx_core::{DeviceInfo, EqConfig, EqFilterKind, ParametricEqBand};

#[derive(Debug, Clone)]
pub enum EqControl {
    Load(String),
    Save(EqConfig),
    Apply(String),
    Delete(String),
    Bypass,
}

pub fn show(
    ui: &mut egui::Ui,
    device: &DeviceInfo,
    config: &mut EqConfig,
    profiles: &[String],
    selected_profile: &mut String,
) -> Option<EqControl> {
    let mut action = None;

    ui.heading("Parametric EQ");
    ui.label(RichText::new("ForgeHX uses the Linux PipeWire DSP path for system EQ. Profiles are stored separately from vendor firmware.").color(theme::text_secondary()));
    if let Some(owner) = device.selected_owner(forgehx_core::Capability::Eq) {
        widgets::info_row(ui, "EQ backend", owner.backend);
    }
    if config.target_node_id.is_none() {
        config.target_node_id = device
            .interfaces
            .iter()
            .find_map(|interface| interface.audio_node_id);
    }
    if let Some(node) = config.target_node_id {
        widgets::info_row(ui, "Target PipeWire node", node);
    }

    ui.separator();
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("forgehx_eq_profiles")
            .selected_text(if selected_profile.is_empty() {
                "Saved profiles"
            } else {
                selected_profile.as_str()
            })
            .show_ui(ui, |ui| {
                for name in profiles {
                    ui.selectable_value(selected_profile, name.clone(), name);
                }
            });
        if ui
            .add_enabled(!selected_profile.is_empty(), egui::Button::new("Load"))
            .clicked()
        {
            action = Some(EqControl::Load(selected_profile.clone()));
        }
        if ui
            .add_enabled(
                !selected_profile.is_empty(),
                egui::Button::new("Apply").fill(theme::accent()),
            )
            .clicked()
        {
            action = Some(EqControl::Apply(selected_profile.clone()));
        }
        if ui
            .add_enabled(!selected_profile.is_empty(), egui::Button::new("Delete"))
            .clicked()
        {
            action = Some(EqControl::Delete(selected_profile.clone()));
        }
        if ui.button("Bypass").clicked() {
            action = Some(EqControl::Bypass);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Profile name");
        ui.text_edit_singleline(&mut config.name);
        ui.add(egui::Slider::new(&mut config.preamp_db, -30.0..=12.0).text("Preamp dB"));
    });

    let mut remove = None;
    for (index, band) in config.filters.iter_mut().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut band.enabled, format!("Band {}", index + 1));
                egui::ComboBox::from_id_salt(("eq_kind", index))
                    .selected_text(match band.kind {
                        EqFilterKind::Peaking => "Peaking",
                        EqFilterKind::LowShelf => "Low shelf",
                        EqFilterKind::HighShelf => "High shelf",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut band.kind, EqFilterKind::Peaking, "Peaking");
                        ui.selectable_value(&mut band.kind, EqFilterKind::LowShelf, "Low shelf");
                        ui.selectable_value(&mut band.kind, EqFilterKind::HighShelf, "High shelf");
                    });
                if ui.button("Remove").clicked() {
                    remove = Some(index);
                }
            });
            ui.add(
                egui::Slider::new(&mut band.frequency_hz, 10.0..=24000.0)
                    .logarithmic(true)
                    .text("Frequency Hz"),
            );
            ui.add(egui::Slider::new(&mut band.gain_db, -24.0..=24.0).text("Gain dB"));
            ui.add(
                egui::Slider::new(&mut band.q, 0.05..=20.0)
                    .logarithmic(true)
                    .text("Q"),
            );
        });
    }
    if let Some(index) = remove {
        config.filters.remove(index);
    }
    ui.horizontal(|ui| {
        if ui
            .add_enabled(config.filters.len() < 32, egui::Button::new("Add band"))
            .clicked()
        {
            config.filters.push(ParametricEqBand {
                enabled: true,
                kind: EqFilterKind::Peaking,
                frequency_hz: 1000.0,
                gain_db: 0.0,
                q: 1.0,
            });
        }
        if ui
            .add_enabled(!config.name.trim().is_empty(), egui::Button::new("Save EQ"))
            .clicked()
        {
            action = Some(EqControl::Save(config.clone()));
        }
    });
    action
}
