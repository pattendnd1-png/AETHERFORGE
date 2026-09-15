use aether_control_core::{action_spec, actions_for_page, ActionId, ConfirmationPolicy, Page, Privilege, RestartPolicy, SnapshotId};
use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke};
use std::process::Command;
use std::sync::mpsc::{self, Receiver};

const WORKER: &str = "/usr/lib/aetherforge/aether-control-worker";

struct RunResult {
    ok: bool,
    output: String,
    restart: RestartPolicy,
}

struct ControlCenter {
    page: Page,
    output: String,
    pending: Option<ActionId>,
    snapshot_id: String,
    restart_required: bool,
    receiver: Option<Receiver<RunResult>>,
}

impl Default for ControlCenter {
    fn default() -> Self {
        Self {
            page: Page::Dashboard,
            output: "Aether Control Center v0.5.0 — Rust Toolbox Reforge\nReady.".to_owned(),
            pending: None,
            snapshot_id: String::new(),
            restart_required: false,
            receiver: None,
        }
    }
}

impl ControlCenter {
    fn dispatch(&mut self, id: ActionId, confirmed: bool) {
        let spec = action_spec(id);
        if spec.confirmation == ConfirmationPolicy::DestructiveOnly && !confirmed {
            self.pending = Some(id);
            return;
        }
        let snapshot = if id == ActionId::SnapshotRollback {
            match SnapshotId::parse(self.snapshot_id.trim()) {
                Ok(value) => Some(value),
                Err(error) => {
                    self.output = format!("Snapshot id error: {error}");
                    return;
                }
            }
        } else {
            None
        };
        self.pending = None;
        self.output = format!("Running: {}…", spec.label);
        let (tx, rx) = mpsc::channel();
        self.receiver = Some(rx);
        std::thread::spawn(move || {
            let mut args = vec!["--action".to_owned(), id.as_str().to_owned()];
            if let Some(snapshot) = snapshot {
                args.push("--snapshot-id".to_owned());
                args.push(snapshot.get().to_string());
            }
            if confirmed {
                args.push("--confirm-destructive".to_owned());
            }
            let result = if spec.privilege == Privilege::Root {
                Command::new("/usr/bin/pkexec").arg(WORKER).args(&args).output()
            } else {
                Command::new(WORKER).args(&args).output()
            };
            let run = match result {
                Ok(output) => {
                    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
                    if !output.stderr.is_empty() {
                        if !text.is_empty() { text.push('\n'); }
                        text.push_str(&String::from_utf8_lossy(&output.stderr));
                    }
                    RunResult { ok: output.status.success(), output: text, restart: spec.restart }
                }
                Err(error) => RunResult { ok: false, output: format!("Unable to launch typed backend: {error}"), restart: RestartPolicy::None },
            };
            let _ = tx.send(run);
        });
    }

    fn poll_result(&mut self) {
        let Some(receiver) = &self.receiver else { return; };
        if let Ok(result) = receiver.try_recv() {
            self.output = if result.output.trim().is_empty() {
                if result.ok { "Action completed successfully.".to_owned() } else { "Action failed without output.".to_owned() }
            } else {
                result.output
            };
            if result.ok && matches!(result.restart, RestartPolicy::MayRequireManual | RestartPolicy::RequiredManual) {
                self.restart_required = true;
            }
            self.receiver = None;
        }
    }
}

impl eframe::App for ControlCenter {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_result();
        ctx.request_repaint_after(std::time::Duration::from_millis(150));

        egui::SidePanel::left("aether-sidebar")
            .resizable(false)
            .exact_width(220.0)
            .frame(egui::Frame::default().fill(Color32::from_rgba_unmultiplied(12, 7, 30, 242)))
            .show(ctx, |ui| {
                ui.add_space(18.0);
                ui.heading(RichText::new("AETHER CONTROL").color(Color32::from_rgb(225, 180, 255)));
                ui.label(RichText::new("CENTER  v0.5.0").color(Color32::from_rgb(87, 218, 255)));
                ui.add_space(16.0);
                for page in Page::ALL {
                    let selected = self.page == page;
                    if ui.selectable_label(selected, page.label()).clicked() {
                        self.page = page;
                    }
                }
            });

        egui::TopBottomPanel::top("status-banner").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.strong("AetherForge system control plane");
                if self.receiver.is_some() {
                    ui.spinner();
                    ui.label("Action running");
                }
                if self.restart_required {
                    ui.separator();
                    ui.colored_label(Color32::from_rgb(255, 183, 77), "Restart Required — use your normal session controls when ready");
                }
            });
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(Color32::from_rgb(8, 5, 20)).inner_margin(egui::Margin::same(24)))
            .show(ctx, |ui| {
                ui.heading(self.page.label());
                ui.separator();
                match self.page {
                    Page::Devices => ui.label("Existing Aether device registry/provider ownership remains authoritative; vendor-specific advanced controls stay in ForgeHX/ReForge/OpenDeck."),
                    Page::Profiles => ui.label("Aether profiles remain a first-class Control Center page."),
                    Page::Gaming => ui.label("Gaming controls remain in the Aether profile/capability integration lane."),
                    Page::Streaming => ui.label("Streaming controls remain in the ForgeStream/OpenDeck integration lane."),
                    Page::AudioCreator => ui.label("Audio and creator controls remain capability-driven and do not expose raw device writes."),
                    Page::ForgeSuite => ui.label("Forge Suite surfaces Aether-owned applications without surrendering device ownership to the system worker."),
                    _ => {}
                }

                if self.page == Page::SnapshotsRecovery {
                    ui.horizontal(|ui| {
                        ui.label("Snapshot ID for rollback:");
                        ui.text_edit_singleline(&mut self.snapshot_id);
                    });
                }

                ui.add_space(8.0);
                for spec in actions_for_page(self.page) {
                    egui::Frame::default()
                        .fill(Color32::from_rgba_unmultiplied(28, 12, 50, 220))
                        .stroke(Stroke::new(1.0, Color32::from_rgba_unmultiplied(166, 76, 255, 110)))
                        .corner_radius(12.0)
                        .inner_margin(egui::Margin::same(12))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.strong(spec.label);
                                    ui.label(spec.description);
                                    ui.small(format!("{} • {:?}", spec.id.as_str(), spec.privilege));
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let label = if spec.confirmation == ConfirmationPolicy::DestructiveOnly { "Review" } else { "Run" };
                                    if ui.add_enabled(self.receiver.is_none(), egui::Button::new(label)).clicked() {
                                        self.dispatch(spec.id, false);
                                    }
                                });
                            });
                        });
                    ui.add_space(8.0);
                }

                ui.add_space(14.0);
                ui.heading("Activity / Output");
                ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    ui.monospace(&self.output);
                });
            });

        if let Some(id) = self.pending {
            let spec = action_spec(id);
            egui::Window::new("Confirm destructive operation")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(spec.label);
                    ui.label("This typed action changes or removes persistent system state.");
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() { self.pending = None; }
                        if ui.button("Confirm").clicked() { self.dispatch(id, true); }
                    });
                });
        }
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "org.aetherforge.ControlCenter",
        options,
        Box::new(|_cc| Ok(Box::new(ControlCenter::default()))),
    )
}
