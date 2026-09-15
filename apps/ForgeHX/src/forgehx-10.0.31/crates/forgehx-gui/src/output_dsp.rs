use crate::{theme, widgets};
use eframe::egui::{self, RichText};
use forgehx_core::{
    OutputDeviceClass, OutputDspProfile, OutputFilter, OutputFilterKind, PRESERVED_AUDIO_ENDPOINT,
};

#[derive(Debug, Clone)]
pub enum OutputDspAction {
    LiveUpdate(OutputDspProfile),
    ResetFactory(OutputDeviceClass),
}

pub fn show(
    ui: &mut egui::Ui,
    profile: &mut OutputDspProfile,
    live: bool,
    generation: u64,
) -> Option<OutputDspAction> {
    widgets::page_heading(
        ui,
        "Output DSP",
        "Full-range AetherStream processing • live edits • routing preserved",
    );
    ui.label(
        RichText::new(format!("Input / output lock: {PRESERVED_AUDIO_ENDPOINT}"))
            .strong()
            .color(theme::accent()),
    );
    ui.small("ForgeHX controls DSP only. It never changes the selected PipeWire source/sink or default device.");
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.label(if live {
            "AetherStream DSP: LIVE"
        } else {
            "AetherStream DSP: OFFLINE"
        });
        ui.separator();
        ui.label(format!("Generation {generation}"));
    });
    ui.add_space(14.0);

    let mut changed = false;
    let mut reset_factory = false;
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.heading("Device profile");
        egui::ComboBox::from_id_salt("output_device_class")
            .selected_text(profile.device_class.label())
            .show_ui(ui, |ui| {
                for class in OutputDeviceClass::ALL {
                    if ui
                        .selectable_value(&mut profile.device_class, class, class.label())
                        .changed()
                    {
                        changed = true;
                    }
                }
            });
        if ui.button("Load factory profile").clicked() {
            reset_factory = true;
        }
        changed |= ui
            .checkbox(&mut profile.bypassed, "Bypass output DSP")
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut profile.preamp_db, -24.0..=12.0)
                    .text("Preamp / headroom dB"),
            )
            .changed();
    });
    ui.add_space(10.0);

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.heading("Filters / EQ");
        let mut remove = None;
        for (i, f) in profile.filters.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                changed |= ui.checkbox(&mut f.enabled, "").changed();
                egui::ComboBox::from_id_salt(("filter_kind", i))
                    .selected_text(f.kind.label())
                    .show_ui(ui, |ui| {
                        for kind in OutputFilterKind::ALL {
                            changed |= ui
                                .selectable_value(&mut f.kind, kind, kind.label())
                                .changed();
                        }
                    });
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut f.frequency_hz)
                            .range(20.0..=22000.0)
                            .suffix(" Hz"),
                    )
                    .changed();
                if matches!(
                    f.kind,
                    OutputFilterKind::Peak
                        | OutputFilterKind::LowShelf
                        | OutputFilterKind::HighShelf
                ) {
                    changed |= ui
                        .add(
                            egui::DragValue::new(&mut f.gain_db)
                                .range(-24.0..=24.0)
                                .suffix(" dB"),
                        )
                        .changed();
                }
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut f.q)
                            .range(0.1..=18.0)
                            .speed(0.05)
                            .prefix("Q "),
                    )
                    .changed();
                if ui.small_button("×").clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            profile.filters.remove(i);
            changed = true;
        }
        ui.horizontal(|ui| {
            if ui.button("+ High-pass").clicked() {
                profile.filters.push(OutputFilter::high_pass(60.0, 24));
                changed = true;
            }
            if ui.button("+ Mid / band-pass").clicked() {
                profile
                    .filters
                    .push(OutputFilter::new(OutputFilterKind::BandPass, 1000.0));
                changed = true;
            }
            if ui.button("+ Low-pass").clicked() {
                profile.filters.push(OutputFilter::low_pass(18000.0, 12));
                changed = true;
            }
            if ui.button("+ Parametric").clicked() {
                profile
                    .filters
                    .push(OutputFilter::new(OutputFilterKind::Peak, 3000.0));
                changed = true;
            }
        });
    });
    ui.add_space(10.0);

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.heading("Bass boost");
        changed |= ui.checkbox(&mut profile.bass.enabled, "Enabled").changed();
        changed |= ui
            .add(egui::Slider::new(&mut profile.bass.amount_db, 0.0..=12.0).text("Boost dB"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut profile.bass.frequency_hz, 40.0..=250.0).text("Center Hz"))
            .changed();
        ui.separator();
        ui.heading("Clarity boost");
        changed |= ui
            .checkbox(&mut profile.clarity.enabled, "Enabled")
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut profile.clarity.amount_db, 0.0..=12.0).text("Boost dB"))
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut profile.clarity.frequency_hz, 1000.0..=8000.0)
                    .text("Center Hz"),
            )
            .changed();
    });
    ui.add_space(10.0);

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.heading("Gaming impact / detail");
        changed |= ui
            .checkbox(&mut profile.gaming.enabled, "Gaming enhancement")
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut profile.gaming.impact_db, -6.0..=12.0)
                    .text("Gaming impact dB"),
            )
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut profile.gaming.mud_cut_db, -12.0..=0.0).text("Mud cut dB"))
            .changed();
        changed |= ui
            .add(egui::Slider::new(&mut profile.gaming.detail_db, -6.0..=12.0).text("Detail dB"))
            .changed();
        ui.separator();
        ui.heading("Limiter / output");
        changed |= ui
            .add(
                egui::Slider::new(&mut profile.output_gain_db, -24.0..=12.0).text("Output gain dB"),
            )
            .changed();
        changed |= ui
            .checkbox(&mut profile.limiter.enabled, "Limiter")
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut profile.limiter.ceiling_dbfs, -12.0..=0.0)
                    .text("Limiter ceiling dBFS"),
            )
            .changed();
    });

    if reset_factory {
        Some(OutputDspAction::ResetFactory(profile.device_class))
    } else {
        changed.then(|| OutputDspAction::LiveUpdate(profile.clone()))
    }
}
