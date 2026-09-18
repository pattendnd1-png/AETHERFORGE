use aetherforge_beacn_control::{
    hardware::{
        EqBandKind, EqBandState, HeadphoneEqChannel, HeadphonePower, NoiseStyle, ProcessorMode,
    },
    layout::ResponsiveLayout,
    pipewire, probe, profile, recorder, session,
    software_dsp::{DspBackendState, SoftwareDspState},
    usb,
};
use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke};
use std::env;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const APP_NAME: &str = "AetherForge BEACN Control";
const VERSION: &str = "0.1.12";
const REFRESH_INTERVAL: Duration = Duration::from_secs(2);

fn main() -> eframe::Result {
    if repair_output_profile_arg() {
        match pipewire::repair_beacn_output_profile_only(Duration::from_secs(3)) {
            Ok(Some((profile, _))) => {
                println!("AETHERFORGE_BEACN_OUTPUT_PROFILE_REPAIR=PASS:{profile}");
            }
            Ok(None) => {
                println!("AETHERFORGE_BEACN_OUTPUT_PROFILE_REPAIR=SKIP:NO_BEACN_OUTPUT_PROFILE");
            }
            Err(error) => {
                eprintln!("AETHERFORGE_BEACN_OUTPUT_PROFILE_REPAIR=FAIL:{error}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(path) = probe_arg() {
        if let Err(error) = probe::write_probe(&path) {
            eprintln!("probe failed: {error}");
            std::process::exit(1);
        }
        println!("AETHERFORGE_BEACN_PROBE={}", path.display());
        return Ok(());
    }

    if graphical_session_probe_arg() {
        let recovered = session::recover_graphical_environment();
        if session::graphical_environment_is_usable(&recovered) {
            let backend = if let Some(display) = recovered.get("WAYLAND_DISPLAY") {
                format!("WAYLAND:{display}")
            } else if let Some(display) = recovered.get("DISPLAY") {
                format!("X11:{display}")
            } else {
                "UNKNOWN".to_owned()
            };
            println!("AETHERFORGE_BEACN_GRAPHICAL_SESSION_RECOVERY=PASS:{backend}");
            return Ok(());
        }
        eprintln!("AETHERFORGE_BEACN_GRAPHICAL_SESSION_RECOVERY=FAIL:NO_DISPLAY_RECOVERED");
        std::process::exit(1);
    }

    match session::ensure_graphical_session() {
        Ok(session::GraphicalSessionAction::Ready) => {}
        Ok(session::GraphicalSessionAction::ReexecStarted) => return Ok(()),
        Err(error) => {
            eprintln!("AETHERFORGE_BEACN_GRAPHICAL_SESSION=FAIL:{error}");
            eprintln!(
                "Launch from the Plasma desktop session or import its Wayland/X11 environment."
            );
            std::process::exit(1);
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!("{APP_NAME} v{VERSION}"))
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(|cc| Ok(Box::new(BeacnApp::new(cc)))),
    )
}

fn graphical_session_probe_arg() -> bool {
    env::args_os()
        .skip(1)
        .any(|arg| arg.to_string_lossy() == "--graphical-session-probe")
}

fn repair_output_profile_arg() -> bool {
    env::args_os()
        .skip(1)
        .any(|arg| arg.to_string_lossy() == "--repair-output-profile")
}

fn probe_arg() -> Option<PathBuf> {
    let mut args = env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg.to_string_lossy() == "--probe" {
            return args.next().map(PathBuf::from);
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Mic,
    Mixer,
    Routing,
    Profiles,
    Device,
    Settings,
}

impl Page {
    const ALL: [Self; 6] = [
        Self::Mic,
        Self::Mixer,
        Self::Routing,
        Self::Profiles,
        Self::Device,
        Self::Settings,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Mic => "Mic Chain",
            Self::Mixer => "Mixing",
            Self::Routing => "Routing Table",
            Self::Profiles => "Profiles",
            Self::Device => "Device",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MicModule {
    Compressor,
    Expander,
    NoiseSuppression,
    DeEsser,
    Exciter,
    Headphones,
}

impl MicModule {
    const ALL: [Self; 6] = [
        Self::Compressor,
        Self::Expander,
        Self::NoiseSuppression,
        Self::DeEsser,
        Self::Exciter,
        Self::Headphones,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Compressor => "Compressor",
            Self::Expander => "Expander",
            Self::NoiseSuppression => "Noise Suppression",
            Self::DeEsser => "De-Esser",
            Self::Exciter => "Exciter",
            Self::Headphones => "Headphones",
        }
    }
}

struct BeacnApp {
    page: Page,
    mic_module: MicModule,
    graph: pipewire::AudioGraph,
    usb_devices: Vec<usb::UsbDevice>,
    selected_source: Option<String>,
    selected_sink: Option<String>,
    source_state: pipewire::VolumeState,
    sink_state: pipewire::VolumeState,
    profile_name: String,
    profiles: Vec<String>,
    status: String,
    last_refresh: Instant,
    auto_refresh: bool,
    device_column_visible: bool,
    hardware_status: String,
    dsp: SoftwareDspState,
    dsp_backend: DspBackendState,
    profile_dirty: bool,
    snapshots: Vec<String>,
    headphone_eq_channel: HeadphoneEqChannel,
    profile_panel_open: bool,
}

impl BeacnApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_dark_theme(&cc.egui_ctx);
        let mut app = Self {
            page: Page::Mic,
            mic_module: MicModule::Compressor,
            graph: pipewire::AudioGraph::default(),
            usb_devices: Vec::new(),
            selected_source: None,
            selected_sink: None,
            source_state: pipewire::VolumeState {
                volume: 0.0,
                muted: false,
            },
            sink_state: pipewire::VolumeState {
                volume: 0.0,
                muted: false,
            },
            profile_name: "Streaming".to_owned(),
            profiles: Vec::new(),
            status: "Starting…".to_owned(),
            last_refresh: Instant::now() - REFRESH_INTERVAL,
            auto_refresh: true,
            device_column_visible: true,
            hardware_status: "Protected pass-through active · Direct USB DSP control is blocked to preserve ALSA/PipeWire microphone availability".to_owned(),
            dsp: SoftwareDspState::default(),
            dsp_backend: DspBackendState::Unavailable,
            profile_dirty: false,
            snapshots: profile::list_snapshots().unwrap_or_default(),
            headphone_eq_channel: HeadphoneEqChannel::Left,
            profile_panel_open: false,
        };
        app.refresh_all();
        app.repair_output_profile_at_startup();
        app
    }

    fn repair_output_profile_at_startup(&mut self) {
        if self.usb_devices.is_empty()
            || !matches!(
                self.audio_health(),
                pipewire::BeacnAudioHealth::NodesMissing | pipewire::BeacnAudioHealth::SinkMissing
            )
        {
            return;
        }

        if let Ok(Some((profile, graph))) =
            pipewire::repair_beacn_output_profile_only(Duration::from_secs(2))
        {
            self.graph = graph;
            self.reselect_nodes();
            self.refresh_levels();
            self.status = format!("Restored BEACN output profile: {profile}");
            self.last_refresh = Instant::now();
        }
    }

    fn refresh_all(&mut self) {
        self.usb_devices = usb::discover_beacn_devices().unwrap_or_default();
        match pipewire::status() {
            Ok(graph) => {
                self.graph = graph;
                self.reselect_nodes();
                self.refresh_levels();
                self.status = self.audio_health_status();
            }
            Err(error) => {
                self.status = format!("PipeWire/WirePlumber unavailable: {error}");
            }
        }
        self.profiles = profile::list().unwrap_or_default();
        self.snapshots = profile::list_snapshots().unwrap_or_default();
        self.last_refresh = Instant::now();
    }

    fn reselect_nodes(&mut self) {
        let require_beacn = !self.usb_devices.is_empty();
        self.selected_source = pipewire::reselect_node(
            &self.graph.sources,
            self.selected_source.as_deref(),
            require_beacn,
        );
        self.selected_sink = pipewire::reselect_node(
            &self.graph.sinks,
            self.selected_sink.as_deref(),
            require_beacn,
        );
    }

    fn audio_health(&self) -> pipewire::BeacnAudioHealth {
        pipewire::beacn_audio_health(&self.graph, !self.usb_devices.is_empty())
    }

    fn audio_health_status(&self) -> String {
        let health = self.audio_health();
        if health.is_healthy() {
            "Audio graph ready · BEACN nodes healthy".to_owned()
        } else {
            health.to_string()
        }
    }

    fn recover_audio_nodes(&mut self) {
        if self.usb_devices.is_empty() {
            self.status = "Cannot recover audio nodes: BEACN USB device is not present".to_owned();
            return;
        }

        let selected_source = self.selected_source.clone();
        let selected_sink = self.selected_sink.clone();
        match pipewire::recover_beacn_nodes(Duration::from_secs(5)) {
            Ok(graph) => {
                self.graph = graph;
                self.selected_source = selected_source;
                self.selected_sink = selected_sink;
                self.reselect_nodes();
                self.refresh_levels();
                self.status =
                    "BEACN audio nodes recovered without changing configured defaults".to_owned();
            }
            Err(error) => {
                self.status = format!("BEACN audio-node recovery failed: {error}");
            }
        }
        self.last_refresh = Instant::now();
    }

    fn refresh_levels(&mut self) {
        if let Some(node) = selected(&self.graph.sources, self.selected_source.as_deref())
            && let Ok(state) = pipewire::get_volume(&node.id)
        {
            self.source_state = state;
        }
        if let Some(node) = selected(&self.graph.sinks, self.selected_sink.as_deref())
            && let Ok(state) = pipewire::get_volume(&node.id)
        {
            self.sink_state = state;
        }
    }

    fn apply_source(&mut self) {
        let Some(node) = selected(&self.graph.sources, self.selected_source.as_deref()) else {
            return;
        };
        let result = pipewire::set_volume(&node.id, self.source_state.volume)
            .and_then(|()| pipewire::set_mute(&node.id, self.source_state.muted));
        self.status = action_status("Microphone", result);
    }

    fn apply_sink(&mut self) {
        let Some(node) = selected(&self.graph.sinks, self.selected_sink.as_deref()) else {
            return;
        };
        let result = pipewire::set_volume(&node.id, self.sink_state.volume)
            .and_then(|()| pipewire::set_mute(&node.id, self.sink_state.muted));
        self.status = action_status("Headphones/output", result);
    }

    fn current_profile(&self) -> profile::Profile {
        let name = if self.profile_name.trim().is_empty() {
            "Live Profile".to_owned()
        } else {
            self.profile_name.trim().to_owned()
        };
        profile::Profile {
            name,
            source_name: self.selected_source.clone().unwrap_or_default(),
            source_volume: self.source_state.volume,
            source_muted: self.source_state.muted,
            sink_name: self.selected_sink.clone().unwrap_or_default(),
            sink_volume: self.sink_state.volume,
            sink_muted: self.sink_state.muted,
            dsp: self.dsp.clone(),
        }
    }

    fn save_profile(&mut self) {
        let item = self.current_profile();
        match profile::save(&item) {
            Ok(path) => {
                self.profile_name = item.name;
                self.profile_dirty = false;
                self.status = format!("Live Profile saved: {}", path.display());
                self.profiles = profile::list().unwrap_or_default();
            }
            Err(error) => self.status = format!("Profile save failed: {error}"),
        }
    }

    fn autosave_profile(&mut self) {
        self.profile_dirty = true;
        let item = self.current_profile();
        match profile::save(&item) {
            Ok(_) => {
                self.profile_name = item.name;
                self.profile_dirty = false;
                self.profiles = profile::list().unwrap_or_default();
            }
            Err(error) => self.status = format!("Live Profile autosave failed: {error}"),
        }
    }

    fn save_snapshot(&mut self) {
        let item = self.current_profile();
        match profile::save_snapshot(&item, "Manual") {
            Ok(path) => {
                self.status = format!("Snapshot saved: {}", path.display());
                self.snapshots = profile::list_snapshots().unwrap_or_default();
            }
            Err(error) => self.status = format!("Snapshot save failed: {error}"),
        }
    }

    fn apply_profile(&mut self, item: profile::Profile, label: &str) {
        self.selected_source = if item.source_name.is_empty() {
            None
        } else {
            Some(item.source_name.clone())
        };
        self.selected_sink = if item.sink_name.is_empty() {
            None
        } else {
            Some(item.sink_name.clone())
        };
        self.source_state = pipewire::VolumeState {
            volume: item.source_volume,
            muted: item.source_muted,
        };
        self.sink_state = pipewire::VolumeState {
            volume: item.sink_volume,
            muted: item.sink_muted,
        };
        self.profile_name = item.name;
        self.dsp = item.dsp;
        self.dsp.sanitize();
        self.apply_source();
        self.apply_sink();
        self.profile_dirty = false;
        self.status = format!("{label} restored");
    }

    fn load_profile(&mut self, name: &str) {
        match profile::load(name) {
            Ok(item) => self.apply_profile(item, &format!("Live Profile {name}")),
            Err(error) => self.status = format!("Profile load failed: {error}"),
        }
    }

    fn load_snapshot(&mut self, name: &str) {
        match profile::load_snapshot(name) {
            Ok(item) => {
                self.apply_profile(item, &format!("Snapshot {name}"));
                self.autosave_profile();
            }
            Err(error) => self.status = format!("Snapshot load failed: {error}"),
        }
    }

    fn dsp_edit_committed(&mut self) {
        self.dsp.sanitize();
        self.autosave_profile();
        if !self.dsp_backend.can_dispatch() {
            self.status = format!(
                "Live Profile autosaved · {} · ALSA/PipeWire microphone path preserved",
                self.dsp_backend.label()
            );
        }
    }

    fn beacn_header(&mut self, ui: &mut egui::Ui, layout: ResponsiveLayout) {
        egui::Panel::top("beacn-header")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(11, 9, 20, 248))
                    .stroke(Stroke::new(
                        1.0,
                        Color32::from_rgba_unmultiplied(132, 106, 225, 42),
                    ))
                    .inner_margin(egui::Margin::symmetric(16, 10)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("BEACN")
                            .size(22.0)
                            .strong()
                            .color(Color32::from_rgb(236, 233, 248)),
                    );
                    ui.label(
                        RichText::new("// AETHERFORGE")
                            .size(11.0)
                            .strong()
                            .color(accent()),
                    );
                    ui.add_space(10.0);
                    status_pill(
                        ui,
                        if self.usb_devices.is_empty() {
                            "MIC OFFLINE"
                        } else {
                            "MIC ONLINE"
                        },
                        !self.usb_devices.is_empty(),
                    );

                    if matches!(layout, ResponsiveLayout::Compact) {
                        egui::ComboBox::from_id_salt("compact-beacn-page")
                            .selected_text(self.page.label())
                            .show_ui(ui, |ui| {
                                for page in Page::ALL {
                                    ui.selectable_value(&mut self.page, page, page.label());
                                }
                            });
                    }

                    if !matches!(layout, ResponsiveLayout::Compact) {
                        ui.add_space(8.0);
                        ui.label(RichText::new(&self.status).small().color(dim_text()));
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("↻  REFRESH").clicked() {
                            self.refresh_all();
                        }
                        if !matches!(layout, ResponsiveLayout::Compact)
                            && ui
                                .button(if self.device_column_visible {
                                    "◀ DEVICES"
                                } else {
                                    "DEVICES ▶"
                                })
                                .clicked()
                        {
                            self.device_column_visible = !self.device_column_visible;
                        }
                        ui.label(
                            RichText::new(format!("v{VERSION}"))
                                .small()
                                .color(dim_text()),
                        );
                    });
                });
            });
    }

    fn live_profile_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("live-profile-bar")
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(18, 15, 31, 244))
                    .stroke(Stroke::new(
                        1.0,
                        Color32::from_rgba_unmultiplied(120, 96, 208, 38),
                    ))
                    .inner_margin(egui::Margin::symmetric(16, 8)),
            )
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("LIVE PROFILE")
                            .small()
                            .strong()
                            .color(dim_text()),
                    );
                    let mut selected_profile = self.profile_name.clone();
                    let profile_names = self.profiles.clone();
                    egui::ComboBox::from_id_salt("beacn-live-profile")
                        .width(180.0)
                        .selected_text(&selected_profile)
                        .show_ui(ui, |ui| {
                            for name in &profile_names {
                                if ui
                                    .selectable_value(&mut selected_profile, name.clone(), name)
                                    .changed()
                                {
                                    self.load_profile(name);
                                }
                            }
                        });

                    if ui.button("SNAPSHOT").clicked() {
                        self.save_snapshot();
                    }
                    if ui
                        .button(if self.profile_panel_open {
                            "▴ LIVE PROFILES"
                        } else {
                            "▾ LIVE PROFILES"
                        })
                        .clicked()
                    {
                        self.profile_panel_open = !self.profile_panel_open;
                    }
                    status_pill(
                        ui,
                        if self.profile_dirty {
                            "AUTOSAVING"
                        } else {
                            "AUTOSAVED"
                        },
                        !self.profile_dirty,
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        status_pill(
                            ui,
                            self.dsp_backend.label(),
                            self.dsp_backend.can_dispatch(),
                        );
                        status_pill(ui, "SYSTEM AUDIO PROTECTED", true);
                    });
                });

                if self.profile_panel_open {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("PROFILE NAME").small().color(dim_text()));
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.profile_name).desired_width(170.0),
                        );
                        if response.changed() {
                            self.autosave_profile();
                        }
                        if ui.button("SAVE NOW").clicked() {
                            self.save_profile();
                        }
                        if ui.button("TAKE SNAPSHOT").clicked() {
                            self.save_snapshot();
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        for name in self.profiles.clone() {
                            if dragon_tab(ui, &name, self.profile_name == name) {
                                self.load_profile(&name);
                            }
                        }
                    });
                    if !self.snapshots.is_empty() {
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new("SNAPSHOTS")
                                .small()
                                .strong()
                                .color(dim_text()),
                        );
                        ui.horizontal_wrapped(|ui| {
                            for name in self
                                .snapshots
                                .iter()
                                .rev()
                                .take(8)
                                .cloned()
                                .collect::<Vec<_>>()
                            {
                                if ui.button(format!("↶ {name}")).clicked() {
                                    self.load_snapshot(&name);
                                }
                            }
                        });
                    }
                }
            });
    }

    fn beacn_device_rail(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("beacn-device-rail")
            .resizable(true)
            .default_size(238.0)
            .min_size(190.0)
            .max_size(290.0)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(13, 11, 23, 248))
                    .stroke(Stroke::new(
                        1.0,
                        Color32::from_rgba_unmultiplied(132, 106, 225, 38),
                    ))
                    .inner_margin(12),
            )
            .show(ui, |ui| {
                ui.label(RichText::new("DEVICES").small().strong().color(dim_text()));
                ui.add_space(8.0);

                dragon_card(ui, "BEACN MIC", "USB · ONBOARD DSP", |ui| {
                    ui.horizontal(|ui| {
                        let connected = !self.usb_devices.is_empty();
                        ui.label(
                            RichText::new(if connected { "●" } else { "○" })
                                .size(26.0)
                                .color(if connected { accent() } else { dim_text() }),
                        );
                        ui.vertical(|ui| {
                            ui.label(RichText::new("BEACN MIC").strong());
                            ui.label(
                                RichText::new(if connected {
                                    "CONNECTED"
                                } else {
                                    "NOT DETECTED"
                                })
                                .small()
                                .color(dim_text()),
                            );
                        });
                    });
                    ui.add_space(8.0);
                    if dragon_tab(ui, "MIC SETUP", self.page == Page::Device) {
                        self.page = Page::Device;
                    }
                    if dragon_tab(
                        ui,
                        "MIC CHAIN",
                        self.page == Page::Mic && self.mic_module != MicModule::Headphones,
                    ) {
                        self.page = Page::Mic;
                        if self.mic_module == MicModule::Headphones {
                            self.mic_module = MicModule::Compressor;
                        }
                    }
                    if dragon_tab(
                        ui,
                        "HEADPHONES",
                        self.page == Page::Mic && self.mic_module == MicModule::Headphones,
                    ) {
                        self.page = Page::Mic;
                        self.mic_module = MicModule::Headphones;
                    }
                });

                ui.add_space(10.0);
                ui.label(
                    RichText::new("MIXING SUITE")
                        .small()
                        .strong()
                        .color(dim_text()),
                );
                ui.add_space(5.0);
                if dragon_tab(ui, "MIXING", self.page == Page::Mixer) {
                    self.page = Page::Mixer;
                }
                if dragon_tab(ui, "ROUTING", self.page == Page::Routing) {
                    self.page = Page::Routing;
                }

                ui.add_space(12.0);
                ui.label(RichText::new("APP").small().strong().color(dim_text()));
                ui.add_space(5.0);
                if dragon_tab(ui, "PROFILES", self.page == Page::Profiles) {
                    self.page = Page::Profiles;
                }
                if dragon_tab(ui, "DEVICE", self.page == Page::Device) {
                    self.page = Page::Device;
                }
                if dragon_tab(ui, "SETTINGS", self.page == Page::Settings) {
                    self.page = Page::Settings;
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(
                        RichText::new("AETHERFORGE DRAGONGLASS")
                            .small()
                            .strong()
                            .color(accent()),
                    );
                });
            });
    }

    fn mic_workspace(&mut self, ui: &mut egui::Ui, layout: ResponsiveLayout) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(7, 6, 13, 250))
                    .inner_margin(14),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.mic_chain_canvas(ui, layout);
                        ui.add_space(12.0);
                        self.plugin_tabs(ui, layout);
                        ui.add_space(12.0);
                        self.voice_recorder_strip(ui);
                        if !layout.uses_output_column() {
                            ui.add_space(12.0);
                            self.mic_output_meter(ui);
                        }
                    });
            });
    }

    fn mic_chain_canvas(&mut self, ui: &mut egui::Ui, layout: ResponsiveLayout) {
        dragon_card(
            ui,
            "EQUALIZER & ENHANCEMENT",
            "PRIMARY MICROPHONE PROCESSING",
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    status_pill(ui, "PROFILE DSP ACTIVE", true);
                    status_pill(
                        ui,
                        self.dsp_backend.label(),
                        self.dsp_backend.can_dispatch(),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_enabled(false, egui::Button::new("SYSTEM AUDIO PROTECTED"));
                    });
                });
                ui.label(
                RichText::new("Windows-style controls edit the Live Profile. Live audio DSP is dispatched only through a verified AetherStream backend; direct BEACN USB claims stay blocked.")
                    .small()
                    .color(dim_text()),
            );
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("MIC GAIN").small().strong().color(dim_text()));
                    let mut mic_gain = self.dsp.mic_gain_db;
                    if ui
                        .add(
                            egui::Slider::new(&mut mic_gain, -24.0..=24.0)
                                .show_value(true)
                                .suffix(" dB"),
                        )
                        .changed()
                    {
                        self.dsp.mic_gain_db = mic_gain;
                        self.dsp_edit_committed();
                    }
                    status_pill(ui, "AUTO-SAVE", true);
                });
                ui.add_space(10.0);
                self.microphone_eq_controls(ui, layout);
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    let de = if self.dsp.de_esser.enabled {
                        "DE-ESSER ON"
                    } else {
                        "DE-ESSER OFF"
                    };
                    let ex = if self.dsp.exciter.enabled {
                        "EXCITER ON"
                    } else {
                        "EXCITER OFF"
                    };
                    status_pill(ui, de, self.dsp.de_esser.enabled);
                    status_pill(ui, ex, self.dsp.exciter.enabled);
                });
            },
        );
    }

    fn plugin_tabs(&mut self, ui: &mut egui::Ui, layout: ResponsiveLayout) {
        dragon_card(
            ui,
            "SECONDARY PROCESSING",
            "WINDOWS BEACN 1.4 WORKFLOW",
            |ui| {
                ui.horizontal_wrapped(|ui| {
                    for module in MicModule::ALL {
                        if dragon_tab(ui, module.label(), self.mic_module == module) {
                            self.mic_module = module;
                        }
                    }
                });
                ui.add_space(10.0);
                match self.mic_module {
                    MicModule::Compressor => self.compressor_controls(ui),
                    MicModule::Expander => self.expander_controls(ui),
                    MicModule::NoiseSuppression => self.suppressor_controls(ui),
                    MicModule::DeEsser => self.de_esser_controls(ui),
                    MicModule::Exciter => self.exciter_controls(ui),
                    MicModule::Headphones => self.headphone_controls(ui, layout),
                }
            },
        );
    }

    fn voice_recorder_strip(&mut self, ui: &mut egui::Ui) {
        dragon_card(ui, "VOICE RECORDER", "10-SECOND MIC-CHAIN TEST", |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("●  RECORD 10s").clicked() {
                    self.start_test_recording();
                }
                if ui.button("▶  PLAY TEST").clicked() {
                    self.play_test_recording();
                }
                if let Ok(path) = recorder::recording_path() {
                    ui.label(
                        RichText::new(path.display().to_string())
                            .small()
                            .color(dim_text()),
                    );
                }
            });
        });
    }

    fn mic_output_meter(&mut self, ui: &mut egui::Ui) {
        dragon_card(ui, "MIC OUTPUT", "END OF MICROPHONE CHAIN", |ui| {
            let meter = self.source_state.volume.clamp(0.0, 1.0);
            level_meter(ui, meter);
            ui.label(
                RichText::new(format!("OUTPUT  {:>3}%", (meter * 100.0).round() as i32))
                    .small()
                    .strong()
                    .color(dim_text()),
            );
            ui.add_space(8.0);
            self.mic_output_contents(ui);
        });
    }

    fn auxiliary_workspace(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(7, 6, 13, 250))
                    .inner_margin(14),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        dragon_card(ui, self.page.label(), "BEACN WORKSPACE", |ui| {
                            match self.page {
                                Page::Mixer => self.mixer_page(ui),
                                Page::Routing => self.routing_page(ui),
                                Page::Profiles => self.profiles_page(ui),
                                Page::Device => self.device_page(ui),
                                Page::Settings => self.settings_page(ui),
                                Page::Mic => {}
                            }
                        });
                    });
            });
    }

    fn microphone_eq_controls(&mut self, ui: &mut egui::Ui, layout: ResponsiveLayout) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Microphone EQ").strong());
            status_pill(ui, "9-BAND PARAMETRIC", true);
        });

        let mut mode = self.dsp.mic_eq_mode;
        egui::ComboBox::from_label("EQ Mode")
            .selected_text(processor_mode_label(mode))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut mode, ProcessorMode::Simple, "Simple");
                ui.selectable_value(&mut mode, ProcessorMode::Advanced, "Advanced");
            });
        if mode != self.dsp.mic_eq_mode {
            self.dsp.mic_eq_mode = mode;
            self.dsp_edit_committed();
        }

        ui.add_space(8.0);
        eq_preview(ui, &self.dsp.mic_eq);
        ui.add_space(8.0);
        let card_width = match layout {
            ResponsiveLayout::Compact => ui.available_width().max(240.0),
            ResponsiveLayout::Standard => 220.0,
            ResponsiveLayout::Wide => 190.0,
        };
        ui.horizontal_wrapped(|ui| {
            for index in 0..self.dsp.mic_eq.len() {
                let original = self.dsp.mic_eq[index];
                let mut band = original;
                ui.group(|ui| {
                    ui.set_min_width(card_width);
                    ui.label(
                        RichText::new(format!("Band {}", index + 1))
                            .strong()
                            .color(accent()),
                    );
                    ui.checkbox(&mut band.enabled, "Enabled");
                    egui::ComboBox::from_id_salt(("mic-eq-kind", index))
                        .selected_text(eq_band_kind_label(band.kind))
                        .show_ui(ui, |ui| {
                            for candidate in eq_band_kinds() {
                                ui.selectable_value(
                                    &mut band.kind,
                                    candidate,
                                    eq_band_kind_label(candidate),
                                );
                            }
                        });
                    ui.add_enabled(
                        eq_kind_has_gain(band.kind),
                        egui::Slider::new(&mut band.gain_db, -12.0..=12.0)
                            .text("Gain")
                            .suffix(" dB"),
                    );
                    ui.add(
                        egui::Slider::new(&mut band.frequency_hz, 20.0..=20_000.0)
                            .logarithmic(true)
                            .text("Freq")
                            .suffix(" Hz"),
                    );
                    ui.add(
                        egui::Slider::new(&mut band.q, 0.1..=10.0)
                            .logarithmic(true)
                            .text("Q"),
                    );
                });
                if band != original {
                    self.dsp.mic_eq[index] = band;
                    self.dsp_edit_committed();
                }
            }
        });
    }

    fn headphone_controls(&mut self, ui: &mut egui::Ui, layout: ResponsiveLayout) {
        ui.horizontal_wrapped(|ui| {
            status_pill(ui, "10-BAND PER-EAR EQ", true);
            status_pill(ui, "MONITOR", true);
            ui.label(RichText::new("Enhanced Headphones · software profile model · system audio remains in ALSA/PipeWire").small().color(dim_text()));
        });

        let mut changed = false;
        changed |= ui
            .add(
                egui::Slider::new(&mut self.dsp.headphones.level_db, -70.0..=0.0)
                    .text("Headphone level")
                    .suffix(" dB"),
            )
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut self.dsp.headphones.mic_monitor_db, -100.0..=6.0)
                    .text("Mic monitor")
                    .suffix(" dB"),
            )
            .changed();
        let mut power = self.dsp.headphones.power;
        egui::ComboBox::from_label("Headphone type")
            .selected_text(headphone_power_label(power))
            .show_ui(ui, |ui| {
                for candidate in headphone_power_options() {
                    ui.selectable_value(&mut power, candidate, headphone_power_label(candidate));
                }
            });
        if power != self.dsp.headphones.power {
            self.dsp.headphones.power = power;
            changed = true;
        }
        changed |= ui
            .checkbox(
                &mut self.dsp.headphones.fx_enabled,
                "Headphone DSP / FX enabled",
            )
            .changed();
        changed |= ui
            .checkbox(&mut self.dsp.headphones.mono, "Mono Mode")
            .changed();
        changed |= ui
            .add(
                egui::Slider::new(&mut self.dsp.headphones.balance, -100..=100)
                    .text("Left / Right Balance"),
            )
            .changed();
        changed |= ui
            .checkbox(
                &mut self.dsp.headphones.binaural_personalization,
                "Binaural Personalization",
            )
            .changed();
        ui.horizontal_wrapped(|ui| {
            ui.label("Preset");
            if ui
                .text_edit_singleline(&mut self.dsp.headphones.preset_name)
                .changed()
            {
                changed = true;
            }
        });

        let mut linked = self.dsp.headphones.eq_linked;
        if ui
            .checkbox(&mut linked, "Link left/right headphone EQ")
            .changed()
        {
            self.dsp.set_headphones_linked(
                linked,
                self.headphone_eq_channel == HeadphoneEqChannel::Left,
            );
            changed = true;
        }
        if self.dsp.headphones.eq_linked {
            self.headphone_eq_channel = HeadphoneEqChannel::Left;
        } else {
            ui.horizontal(|ui| {
                ui.label("Editing ear:");
                ui.selectable_value(
                    &mut self.headphone_eq_channel,
                    HeadphoneEqChannel::Left,
                    "Left",
                );
                ui.selectable_value(
                    &mut self.headphone_eq_channel,
                    HeadphoneEqChannel::Right,
                    "Right",
                );
            });
        }
        if changed {
            self.dsp_edit_committed();
        }

        let channel = self.headphone_eq_channel;
        let bands = match channel {
            HeadphoneEqChannel::Left => self.dsp.headphones.eq_left,
            HeadphoneEqChannel::Right => self.dsp.headphones.eq_right,
        };
        eq_preview(ui, &bands);
        ui.add_space(8.0);
        let card_width = match layout {
            ResponsiveLayout::Compact => ui.available_width().max(240.0),
            ResponsiveLayout::Standard => 210.0,
            ResponsiveLayout::Wide => 180.0,
        };
        ui.horizontal_wrapped(|ui| {
            for (index, original) in bands.into_iter().enumerate() {
                let mut band = original;
                ui.group(|ui| {
                    ui.set_min_width(card_width);
                    ui.label(
                        RichText::new(format!(
                            "{} Band {}",
                            headphone_channel_label(channel),
                            index + 1
                        ))
                        .strong()
                        .color(accent()),
                    );
                    ui.checkbox(&mut band.enabled, "Enabled");
                    egui::ComboBox::from_id_salt(("hp-eq-kind", channel, index))
                        .selected_text(eq_band_kind_label(band.kind))
                        .show_ui(ui, |ui| {
                            for candidate in eq_band_kinds() {
                                ui.selectable_value(
                                    &mut band.kind,
                                    candidate,
                                    eq_band_kind_label(candidate),
                                );
                            }
                        });
                    ui.add_enabled(
                        eq_kind_has_gain(band.kind),
                        egui::Slider::new(&mut band.gain_db, -12.0..=12.0)
                            .text("Gain")
                            .suffix(" dB"),
                    );
                    ui.add(
                        egui::Slider::new(&mut band.frequency_hz, 20.0..=20_000.0)
                            .logarithmic(true)
                            .text("Freq")
                            .suffix(" Hz"),
                    );
                    ui.add(
                        egui::Slider::new(&mut band.q, 0.1..=10.0)
                            .logarithmic(true)
                            .text("Q"),
                    );
                });
                if band != original {
                    if let Err(error) = self.dsp.set_headphone_band(
                        channel == HeadphoneEqChannel::Left,
                        index,
                        band,
                    ) {
                        self.status = format!("Headphone EQ edit failed: {error}");
                    } else {
                        self.dsp_edit_committed();
                    }
                }
            }
        });
    }

    fn compressor_controls(&mut self, ui: &mut egui::Ui) {
        let original = self.dsp.compressor.clone();
        ui.checkbox(&mut self.dsp.compressor.enabled, "Enabled");
        let mut mode = self.dsp.compressor.mode;
        egui::ComboBox::from_label("Mode")
            .selected_text(processor_mode_label(mode))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut mode, ProcessorMode::Simple, "Simple");
                ui.selectable_value(&mut mode, ProcessorMode::Advanced, "Advanced");
            });
        self.dsp.compressor.mode = mode;
        ui.add(
            egui::Slider::new(&mut self.dsp.compressor.threshold_db, -40.0..=0.0)
                .text("Threshold")
                .suffix(" dB"),
        );
        ui.add(
            egui::Slider::new(&mut self.dsp.compressor.ratio, 1.0..=16.0)
                .text("Ratio")
                .suffix(":1"),
        );
        if self.dsp.compressor.mode == ProcessorMode::Advanced {
            ui.add(
                egui::Slider::new(&mut self.dsp.compressor.attack_ms, 1.0..=2000.0)
                    .logarithmic(true)
                    .text("Attack")
                    .suffix(" ms"),
            );
            ui.add(
                egui::Slider::new(&mut self.dsp.compressor.release_ms, 1.0..=2000.0)
                    .logarithmic(true)
                    .text("Release")
                    .suffix(" ms"),
            );
            ui.add(
                egui::Slider::new(&mut self.dsp.compressor.makeup_gain_db, 0.0..=12.0)
                    .text("Makeup gain")
                    .suffix(" dB"),
            );
        }
        if self.dsp.compressor != original {
            self.dsp_edit_committed();
        }
    }

    fn expander_controls(&mut self, ui: &mut egui::Ui) {
        let original = self.dsp.expander.clone();
        ui.checkbox(&mut self.dsp.expander.enabled, "Enabled");
        let mut mode = self.dsp.expander.mode;
        egui::ComboBox::from_label("Mode")
            .selected_text(processor_mode_label(mode))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut mode, ProcessorMode::Simple, "Simple");
                ui.selectable_value(&mut mode, ProcessorMode::Advanced, "Advanced");
            });
        self.dsp.expander.mode = mode;
        ui.add(
            egui::Slider::new(&mut self.dsp.expander.threshold_db, -90.0..=0.0)
                .text("Threshold")
                .suffix(" dB"),
        );
        ui.add(
            egui::Slider::new(&mut self.dsp.expander.ratio, 1.0..=10.0)
                .text("Expand amount / Ratio")
                .suffix(":1"),
        );
        if self.dsp.expander.mode == ProcessorMode::Advanced {
            ui.add(
                egui::Slider::new(&mut self.dsp.expander.attack_ms, 1.0..=2000.0)
                    .logarithmic(true)
                    .text("Attack")
                    .suffix(" ms"),
            );
            ui.add(
                egui::Slider::new(&mut self.dsp.expander.release_ms, 1.0..=2000.0)
                    .logarithmic(true)
                    .text("Release")
                    .suffix(" ms"),
            );
        }
        if self.dsp.expander != original {
            self.dsp_edit_committed();
        }
    }

    fn suppressor_controls(&mut self, ui: &mut egui::Ui) {
        let original = self.dsp.suppressor.clone();
        ui.checkbox(
            &mut self.dsp.suppressor.enabled,
            "Noise Suppression Enabled",
        );
        ui.add(
            egui::Slider::new(&mut self.dsp.suppressor.amount, 0.0..=100.0)
                .text("Amount")
                .suffix("%"),
        );
        let mut style = self.dsp.suppressor.style;
        egui::ComboBox::from_label("Style")
            .selected_text(noise_style_label(style))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut style, NoiseStyle::Instant, "Instant");
                ui.selectable_value(&mut style, NoiseStyle::Adaptive, "Adaptive");
                ui.selectable_value(&mut style, NoiseStyle::Snapshot, "Snapshot");
            });
        self.dsp.suppressor.style = style;
        ui.add(
            egui::Slider::new(&mut self.dsp.suppressor.sensitivity_db, -120.0..=-60.0)
                .text("Sensitivity")
                .suffix(" dB"),
        );
        if style == NoiseStyle::Adaptive {
            ui.add(
                egui::Slider::new(&mut self.dsp.suppressor.adapt_ms, 100.0..=5000.0)
                    .logarithmic(true)
                    .text("Adapt time")
                    .suffix(" ms"),
            );
        }
        if self.dsp.suppressor != original {
            self.dsp_edit_committed();
        }
    }

    fn de_esser_controls(&mut self, ui: &mut egui::Ui) {
        let original = self.dsp.de_esser.clone();
        ui.checkbox(&mut self.dsp.de_esser.enabled, "De-Esser Enabled");
        ui.add(
            egui::Slider::new(&mut self.dsp.de_esser.amount, 0.0..=100.0)
                .text("Amount")
                .suffix("%"),
        );
        ui.add(
            egui::Slider::new(&mut self.dsp.de_esser.frequency_hz, 2_000.0..=12_000.0)
                .logarithmic(true)
                .text("Sibilance frequency")
                .suffix(" Hz"),
        );
        ui.label(RichText::new("Reduces harsh sibilance in the same secondary-processing workflow as the Windows app.").small().color(dim_text()));
        if self.dsp.de_esser != original {
            self.dsp_edit_committed();
        }
    }

    fn exciter_controls(&mut self, ui: &mut egui::Ui) {
        let original = self.dsp.exciter.clone();
        ui.checkbox(&mut self.dsp.exciter.enabled, "Exciter Enabled");
        ui.add(
            egui::Slider::new(&mut self.dsp.exciter.amount, 0.0..=100.0)
                .text("Enhancement")
                .suffix("%"),
        );
        ui.add(egui::Slider::new(&mut self.dsp.exciter.tone, 0.0..=100.0).text("Tone"));
        ui.label(RichText::new("Adds the Windows-style enhancement/exciter control state without taking USB audio ownership.").small().color(dim_text()));
        if self.dsp.exciter != original {
            self.dsp_edit_committed();
        }
    }

    fn mic_output_contents(&mut self, ui: &mut egui::Ui) {
        let audio_health = self.audio_health();
        status_pill(ui, &audio_health.to_string(), audio_health.is_healthy());
        status_pill(
            ui,
            self.dsp_backend.label(),
            self.dsp_backend.can_dispatch(),
        );
        if !audio_health.is_healthy() && !self.usb_devices.is_empty() {
            ui.label(RichText::new("BEACN audio nodes missing: selections stay pinned and will not fall through to another microphone.").small().color(dim_text()));
            if ui.button("Recover BEACN audio nodes").clicked() {
                self.recover_audio_nodes();
            }
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new("END-OF-CHAIN DSP")
                .small()
                .strong()
                .color(dim_text()),
        );
        let mut output_gain = self.dsp.mic_output_gain_db;
        if ui
            .add(
                egui::Slider::new(&mut output_gain, -24.0..=12.0)
                    .text("Mic Output Gain")
                    .suffix(" dB"),
            )
            .changed()
        {
            self.dsp.mic_output_gain_db = output_gain;
            self.dsp_edit_committed();
        }
        ui.label(RichText::new("Profile DSP is applied live only when a verified AetherStream mutating backend is available.").small().color(dim_text()));
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);
        ui.label(
            RichText::new("SYSTEM CAPTURE")
                .small()
                .strong()
                .color(dim_text()),
        );
        if node_combo(ui, "Mic", &self.graph.sources, &mut self.selected_source) {
            self.refresh_levels();
            self.autosave_profile();
        }
        let source_available =
            selected(&self.graph.sources, self.selected_source.as_deref()).is_some();
        ui.add_space(6.0);
        ui.add_enabled(
            source_available,
            egui::ProgressBar::new((self.source_state.volume / 1.5).clamp(0.0, 1.0)).text(
                if source_available {
                    format!("{:.0}%", self.source_state.volume * 100.0)
                } else {
                    "offline".to_owned()
                },
            ),
        );
        let source_changed = ui
            .add_enabled(
                source_available,
                egui::Slider::new(&mut self.source_state.volume, 0.0..=1.5)
                    .text("PipeWire level")
                    .suffix(" ×"),
            )
            .changed();
        let source_mute_changed = ui
            .add_enabled(
                source_available,
                egui::Checkbox::new(&mut self.source_state.muted, "Mute mic"),
            )
            .changed();
        if source_changed || source_mute_changed {
            self.apply_source();
            self.autosave_profile();
        }
        ui.add_space(16.0);
        ui.separator();
        ui.add_space(10.0);
        ui.label(
            RichText::new("HEADPHONES / MONITOR")
                .small()
                .strong()
                .color(dim_text()),
        );
        if node_combo(ui, "Output", &self.graph.sinks, &mut self.selected_sink) {
            self.refresh_levels();
            self.autosave_profile();
        }
        let sink_available = selected(&self.graph.sinks, self.selected_sink.as_deref()).is_some();
        ui.add_space(6.0);
        ui.add_enabled(
            sink_available,
            egui::ProgressBar::new((self.sink_state.volume / 1.5).clamp(0.0, 1.0)).text(
                if sink_available {
                    format!("{:.0}%", self.sink_state.volume * 100.0)
                } else {
                    "offline".to_owned()
                },
            ),
        );
        let sink_changed = ui
            .add_enabled(
                sink_available,
                egui::Slider::new(&mut self.sink_state.volume, 0.0..=1.5)
                    .text("PipeWire monitor level")
                    .suffix(" ×"),
            )
            .changed();
        let sink_mute_changed = ui
            .add_enabled(
                sink_available,
                egui::Checkbox::new(&mut self.sink_state.muted, "Mute output"),
            )
            .changed();
        if sink_changed || sink_mute_changed {
            self.apply_sink();
            self.autosave_profile();
        }
    }

    fn start_test_recording(&mut self) {
        let Some(node) = selected(&self.graph.sources, self.selected_source.as_deref()) else {
            self.status = "Select a microphone source first".to_owned();
            return;
        };
        match recorder::start_10s(&node.id) {
            Ok(path) => self.status = format!("Recording 10-second mic test: {}", path.display()),
            Err(error) => self.status = format!("Mic test recording failed: {error}"),
        }
    }

    fn play_test_recording(&mut self) {
        let Some(node) = selected(&self.graph.sinks, self.selected_sink.as_deref()) else {
            self.status = "Select a headphone/output sink first".to_owned();
            return;
        };
        match recorder::play(&node.id) {
            Ok(path) => self.status = format!("Playing mic test: {}", path.display()),
            Err(error) => self.status = format!("Mic test playback failed: {error}"),
        }
    }

    fn mixer_page(&mut self, ui: &mut egui::Ui) {
        section(ui, "Mixing Window", |ui| {
            ui.label(RichText::new("Live PipeWire sources and outputs. Advanced BEACN submix endpoints will layer onto this graph.").color(dim_text()));
        });
        section(ui, "Available Sources", |ui| {
            for node in &self.graph.sources {
                let marker = if node.is_default { "default" } else { "source" };
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&node.name).strong());
                    ui.label(
                        RichText::new(format!("#{} · {marker}", node.id))
                            .small()
                            .color(dim_text()),
                    );
                });
            }
        });
        section(ui, "Available Outputs", |ui| {
            for node in &self.graph.sinks {
                let marker = if node.is_default { "default" } else { "sink" };
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(&node.name).strong());
                    ui.label(
                        RichText::new(format!("#{} · {marker}", node.id))
                            .small()
                            .color(dim_text()),
                    );
                });
            }
        });
    }

    fn routing_page(&mut self, ui: &mut egui::Ui) {
        section(ui, "Routing Table", |ui| {
            ui.label(RichText::new("Read-only PipeWire/WirePlumber graph in this pass. The BEACN routing table will become writable only through explicit, tested endpoints.").color(dim_text()));
            ui.add(
                egui::TextEdit::multiline(&mut self.graph.raw_status)
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(26)
                    .interactive(false),
            );
        });
    }

    fn profiles_page(&mut self, ui: &mut egui::Ui) {
        section(ui, "Live Profiles", |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Name");
                if ui.text_edit_singleline(&mut self.profile_name).changed() {
                    self.autosave_profile();
                }
                if ui.button("Save now").clicked() {
                    self.save_profile();
                }
                if ui.button("Snapshot").clicked() {
                    self.save_snapshot();
                }
                status_pill(ui, "AUTO-SAVE ON", true);
            });
            ui.label(RichText::new("Live Profiles continuously preserve PipeWire selections/levels plus the complete microphone/headphone processing state. Snapshots are fixed restore points.").small().color(dim_text()));
            ui.add_space(12.0);
            if self.profiles.is_empty() {
                ui.label(RichText::new("No profiles saved yet.").color(dim_text()));
            }
            for name in self.profiles.clone() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(&name);
                    if ui.button("Load").clicked() {
                        self.load_profile(&name);
                    }
                });
            }
            if !self.snapshots.is_empty() {
                ui.add_space(14.0);
                ui.separator();
                ui.heading(RichText::new("Snapshots").color(accent()));
                for name in self.snapshots.iter().rev().cloned().collect::<Vec<_>>() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(&name);
                        if ui.button("Restore").clicked() {
                            self.load_snapshot(&name);
                        }
                    });
                }
            }
        });
    }

    fn device_page(&mut self, ui: &mut egui::Ui) {
        section(ui, "BEACN USB Hardware", |ui| {
            if self.usb_devices.is_empty() {
                ui.label(
                    RichText::new("No BEACN-labelled USB device found in sysfs.").color(dim_text()),
                );
            }
            for device in &self.usb_devices {
                ui.heading(if device.product.is_empty() {
                    "BEACN device"
                } else {
                    device.product.as_str()
                });
                grid_row(ui, "Manufacturer", &device.manufacturer);
                grid_row(
                    ui,
                    "VID:PID",
                    &format!("{}:{}", device.vendor_id, device.product_id),
                );
                grid_row(
                    ui,
                    "Bus / Device",
                    &format!("{} / {}", device.bus_number, device.device_number),
                );
                grid_row(ui, "Sysfs", &device.sysfs_path.display().to_string());
                ui.add_space(10.0);
            }
        });
        section(ui, "Audio Node Health", |ui| {
            let health = self.audio_health();
            status_pill(ui, &health.to_string(), health.is_healthy());
            ui.label(RichText::new("Selections stay pinned by stable PipeWire node name; missing BEACN nodes never fall through to a webcam or unrelated source.").color(dim_text()));
            if !health.is_healthy()
                && !self.usb_devices.is_empty()
                && ui.button("Recover BEACN audio nodes").clicked()
            {
                self.recover_audio_nodes();
            }
        });
        section(ui, "Control Ownership", |ui| {
            status_pill(ui, "SYSTEM AUDIO PROTECTED", true);
            status_pill(
                ui,
                self.dsp_backend.label(),
                self.dsp_backend.can_dispatch(),
            );
            ui.label(RichText::new(&self.hardware_status).color(dim_text()));
            ui.add_enabled(false, egui::Button::new("DIRECT USB CONTROL BLOCKED"));
            ui.label(RichText::new("v0.1.12 keeps snd_usb_audio / ALSA / PipeWire authoritative. Windows-style DSP controls are profile-backed and may become audible only through a separately verified AetherStream typed mutation backend.").small().color(dim_text()));
            ui.add_space(8.0);
            grid_row(ui, "Mic EQ model", "9-band parametric");
            grid_row(ui, "Headphone EQ model", "10-band per ear");
            grid_row(
                ui,
                "Secondary processors",
                "Compressor · Expander · Noise Suppression · De-Esser · Exciter",
            );
            grid_row(ui, "Live Profiles", "Auto-save + snapshots");
        });
    }

    fn settings_page(&mut self, ui: &mut egui::Ui) {
        section(ui, "AetherForge", |ui| {
            ui.checkbox(
                &mut self.auto_refresh,
                "Auto-refresh audio/device state every 2 seconds",
            );
            ui.label(
                RichText::new("Dark Mode / DragonGlass is the canonical appearance.")
                    .color(dim_text()),
            );
            ui.label(RichText::new("Windows BEACN 1.4 clean-room workflow parity: Live Profiles, snapshots, anchored EQ/enhancement, secondary processing, Enhanced Headphones, recorder, and Mic Output.").color(dim_text()));
            ui.label(RichText::new("No Electron. No browser runtime. ALSA/PipeWire remain authoritative; direct USB DSP claims are blocked. AetherStream DSP mutation is capability-gated and never guessed.").color(dim_text()));
        });
        section(ui, "DSP Backend", |ui| {
            status_pill(
                ui,
                self.dsp_backend.label(),
                self.dsp_backend.can_dispatch(),
            );
            ui.label(RichText::new("Profile controls remain editable and persistent when the mutating DSP backend is unavailable. The app never reports an audible DSP change unless a verified backend accepted it.").small().color(dim_text()));
        });
        section(ui, "Protocol Probe", |ui| {
            ui.label("Generate a safe, read-only device/audio handoff probe.");
            if ui.button("Write probe to Downloads").clicked() {
                self.write_default_probe();
            }
            ui.monospace("aetherforge-beacn-control --probe ~/Downloads/AetherForge-BEACN-Control-v0.1.12-PROBE.txt");
        });
    }

    fn write_default_probe(&mut self) {
        let Some(home) = env::var_os("HOME") else {
            self.status = "Cannot write probe: HOME is not set".to_owned();
            return;
        };
        let path =
            PathBuf::from(home).join("Downloads/AetherForge-BEACN-Control-v0.1.12-PROBE.txt");
        match probe::write_probe(&path) {
            Ok(()) => self.status = format!("Probe written: {}", path.display()),
            Err(error) => self.status = format!("Probe failed: {error}"),
        }
    }
}

impl eframe::App for BeacnApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.auto_refresh && self.last_refresh.elapsed() >= REFRESH_INTERVAL {
            self.refresh_all();
        }

        let layout = ResponsiveLayout::from_width(ui.available_width());
        self.beacn_header(ui, layout);
        self.live_profile_bar(ui);
        if layout.uses_device_column() && self.device_column_visible {
            self.beacn_device_rail(ui);
        }
        if self.page == Page::Mic && layout.uses_output_column() {
            egui::Panel::right("beacn-mic-output")
                .resizable(true)
                .default_size(285.0)
                .min_size(240.0)
                .max_size(360.0)
                .frame(
                    egui::Frame::new()
                        .fill(Color32::from_rgba_unmultiplied(12, 10, 22, 248))
                        .stroke(Stroke::new(
                            1.0,
                            Color32::from_rgba_unmultiplied(132, 106, 225, 38),
                        ))
                        .inner_margin(12),
                )
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| self.mic_output_meter(ui));
                });
        }

        if self.page == Page::Mic {
            self.mic_workspace(ui, layout);
        } else {
            self.auxiliary_workspace(ui);
        }
        ui.ctx().request_repaint_after(Duration::from_millis(500));
    }
}

fn selected<'a>(
    nodes: &'a [pipewire::AudioNode],
    name: Option<&str>,
) -> Option<&'a pipewire::AudioNode> {
    pipewire::selected_by_name(nodes, name)
}

fn node_combo(
    ui: &mut egui::Ui,
    label: &str,
    nodes: &[pipewire::AudioNode],
    selected_name: &mut Option<String>,
) -> bool {
    let selected_text = match selected(nodes, selected_name.as_deref()) {
        Some(node) => node.name.clone(),
        None => selected_name
            .as_deref()
            .map_or_else(|| "No device".to_owned(), |name| format!("Missing: {name}")),
    };
    let mut changed = false;
    egui::ComboBox::from_label(label)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            for node in nodes {
                changed |= ui
                    .selectable_value(selected_name, Some(node.name.clone()), &node.name)
                    .changed();
            }
        });
    changed
}

fn dragon_card(ui: &mut egui::Ui, title: &str, subtitle: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(21, 18, 36, 232))
        .stroke(Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(126, 100, 219, 52),
        ))
        .corner_radius(CornerRadius::same(13))
        .inner_margin(14)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(title)
                            .size(15.0)
                            .strong()
                            .color(Color32::from_rgb(238, 235, 250)),
                    );
                    ui.label(RichText::new(subtitle).size(9.5).strong().color(dim_text()));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    ui.label(
                        RichText::new("◆")
                            .color(Color32::from_rgba_unmultiplied(174, 151, 255, 125)),
                    );
                });
            });
            ui.add_space(10.0);
            add(ui);
        });
}

fn dragon_tab(ui: &mut egui::Ui, label: &str, selected: bool) -> bool {
    let fill = if selected {
        Color32::from_rgba_unmultiplied(92, 66, 176, 210)
    } else {
        Color32::from_rgba_unmultiplied(30, 25, 49, 190)
    };
    let stroke = if selected {
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(190, 173, 255, 145))
    } else {
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(126, 100, 219, 35))
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(9))
        .inner_margin(egui::Margin::symmetric(9, 6))
        .show(ui, |ui| {
            ui.add(
                egui::Button::new(RichText::new(label).size(10.5).strong().color(if selected {
                    Color32::WHITE
                } else {
                    dim_text()
                }))
                .frame(false),
            )
            .clicked()
        })
        .inner
}

fn section(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(8.0);
    egui::Frame::new()
        .fill(Color32::from_rgba_unmultiplied(24, 20, 40, 220))
        .stroke(Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(105, 83, 190, 55),
        ))
        .corner_radius(CornerRadius::same(16))
        .inner_margin(16)
        .show(ui, |ui| {
            ui.heading(RichText::new(title).color(accent()));
            ui.add_space(6.0);
            add(ui);
        });
}

fn status_pill(ui: &mut egui::Ui, text: &str, positive: bool) {
    egui::Frame::new()
        .fill(if positive {
            Color32::from_rgba_unmultiplied(61, 47, 110, 190)
        } else {
            Color32::from_rgba_unmultiplied(39, 34, 54, 190)
        })
        .corner_radius(CornerRadius::same(10))
        .inner_margin(6)
        .show(ui, |ui| {
            ui.label(RichText::new(text).small().color(if positive {
                accent()
            } else {
                dim_text()
            }));
        });
}

fn grid_row(ui: &mut egui::Ui, key: &str, value: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(format!("{key}:")).strong());
        ui.label(if value.is_empty() { "—" } else { value });
    });
}

fn processor_mode_label(mode: ProcessorMode) -> &'static str {
    match mode {
        ProcessorMode::Simple => "Simple",
        ProcessorMode::Advanced => "Advanced",
    }
}

fn noise_style_label(style: NoiseStyle) -> &'static str {
    match style {
        NoiseStyle::Instant => "Instant",
        NoiseStyle::Adaptive => "Adaptive",
        NoiseStyle::Snapshot => "Snapshot",
    }
}

fn level_meter(ui: &mut egui::Ui, level: f32) {
    let level = level.clamp(0.0, 1.0);
    let width = ui.available_width().max(180.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 18.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let segments = 18;
    let gap = 2.0;
    let segment_width = (rect.width() - gap * (segments as f32 - 1.0)) / segments as f32;
    let active = (level * segments as f32).round() as usize;
    for index in 0..segments {
        let left = rect.left() + index as f32 * (segment_width + gap);
        let segment = egui::Rect::from_min_size(
            egui::pos2(left, rect.top()),
            egui::vec2(segment_width, rect.height()),
        );
        let fill = if index < active {
            if index > 14 {
                Color32::from_rgb(229, 127, 255)
            } else if index > 11 {
                Color32::from_rgb(139, 108, 244)
            } else {
                Color32::from_rgb(96, 77, 207)
            }
        } else {
            Color32::from_rgba_unmultiplied(78, 67, 102, 90)
        };
        painter.rect_filled(segment, CornerRadius::same(2), fill);
    }
}

fn eq_preview(ui: &mut egui::Ui, bands: &[EqBandState]) {
    let width = ui.available_width().max(260.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 170.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(
        rect,
        CornerRadius::same(12),
        Color32::from_rgba_unmultiplied(8, 7, 15, 235),
    );

    let zero_y = rect.center().y;
    painter.line_segment(
        [
            egui::pos2(rect.left(), zero_y),
            egui::pos2(rect.right(), zero_y),
        ],
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(174, 151, 255, 65)),
    );
    for fraction in [0.25_f32, 0.5, 0.75] {
        let x = rect.left() + rect.width() * fraction;
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(174, 151, 255, 24)),
        );
    }

    let min_log = 20.0_f32.ln();
    let max_log = 20_000.0_f32.ln();
    let mut points = Vec::new();
    for (index, band) in bands.iter().enumerate() {
        if !band.enabled {
            continue;
        }
        let frequency = band.frequency_hz.clamp(20.0, 20_000.0);
        let x_t = (frequency.ln() - min_log) / (max_log - min_log);
        let y_t = (band.gain_db.clamp(-12.0, 12.0) + 12.0) / 24.0;
        let point = egui::pos2(
            rect.left() + rect.width() * x_t,
            rect.bottom() - rect.height() * y_t,
        );
        points.push(point);
        painter.circle_filled(point, 5.0, accent());
        painter.text(
            point + egui::vec2(7.0, -7.0),
            egui::Align2::LEFT_BOTTOM,
            (index + 1).to_string(),
            egui::FontId::proportional(11.0),
            dim_text(),
        );
    }
    points.sort_by(|a, b| a.x.total_cmp(&b.x));
    for pair in points.windows(2) {
        painter.line_segment(
            [pair[0], pair[1]],
            Stroke::new(2.0, Color32::from_rgba_unmultiplied(174, 151, 255, 180)),
        );
    }

    painter.text(
        rect.left_top() + egui::vec2(8.0, 8.0),
        egui::Align2::LEFT_TOP,
        "+12 dB",
        egui::FontId::proportional(10.0),
        dim_text(),
    );
    painter.text(
        rect.left_bottom() + egui::vec2(8.0, -8.0),
        egui::Align2::LEFT_BOTTOM,
        "-12 dB · 20 Hz",
        egui::FontId::proportional(10.0),
        dim_text(),
    );
    painter.text(
        rect.right_bottom() + egui::vec2(-8.0, -8.0),
        egui::Align2::RIGHT_BOTTOM,
        "20 kHz",
        egui::FontId::proportional(10.0),
        dim_text(),
    );
}

fn eq_kind_has_gain(kind: EqBandKind) -> bool {
    matches!(
        kind,
        EqBandKind::Bell | EqBandKind::LowShelf | EqBandKind::HighShelf
    )
}

fn eq_band_kind_label(kind: EqBandKind) -> &'static str {
    match kind {
        EqBandKind::NotSet => "Not set",
        EqBandKind::LowPass => "Low pass",
        EqBandKind::HighPass => "High pass",
        EqBandKind::Notch => "Notch",
        EqBandKind::Bell => "Bell",
        EqBandKind::LowShelf => "Low shelf",
        EqBandKind::HighShelf => "High shelf",
    }
}

fn eq_band_kinds() -> [EqBandKind; 7] {
    [
        EqBandKind::NotSet,
        EqBandKind::LowPass,
        EqBandKind::HighPass,
        EqBandKind::Notch,
        EqBandKind::Bell,
        EqBandKind::LowShelf,
        EqBandKind::HighShelf,
    ]
}

fn headphone_power_label(power: HeadphonePower) -> &'static str {
    match power {
        HeadphonePower::LineLevel => "Line level",
        HeadphonePower::Normal => "Normal power",
        HeadphonePower::HighImpedance => "High impedance",
        HeadphonePower::InEarMonitors => "In-ear monitors",
    }
}

fn headphone_power_options() -> [HeadphonePower; 4] {
    [
        HeadphonePower::LineLevel,
        HeadphonePower::Normal,
        HeadphonePower::HighImpedance,
        HeadphonePower::InEarMonitors,
    ]
}

fn headphone_channel_label(channel: HeadphoneEqChannel) -> &'static str {
    match channel {
        HeadphoneEqChannel::Left => "Left",
        HeadphoneEqChannel::Right => "Right",
    }
}

fn action_status(label: &str, result: io::Result<()>) -> String {
    match result {
        Ok(()) => format!("{label} updated"),
        Err(error) => format!("{label} update failed: {error}"),
    }
}

fn apply_dark_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(7, 6, 13);
    visuals.window_fill = Color32::from_rgb(13, 11, 23);
    visuals.extreme_bg_color = Color32::from_rgb(7, 6, 13);
    visuals.selection.bg_fill = Color32::from_rgb(95, 68, 190);
    visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(224, 218, 255));
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(31, 25, 52);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(57, 43, 96);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(77, 56, 142);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(10);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(10);
    visuals.widgets.active.corner_radius = CornerRadius::same(10);
    visuals.override_text_color = Some(Color32::from_rgb(232, 228, 244));
    ctx.set_visuals(visuals);
    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.spacing.item_spacing = egui::vec2(8.0, 7.0);
        style.spacing.button_padding = egui::vec2(10.0, 7.0);
        style.spacing.interact_size.y = 30.0;
    });
}

fn accent() -> Color32 {
    Color32::from_rgb(174, 151, 255)
}

fn dim_text() -> Color32 {
    Color32::from_rgb(184, 178, 207)
}
