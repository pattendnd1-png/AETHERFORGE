use eframe::egui::{self, Color32, CornerRadius, FontId, Pos2, Rect, Stroke, Vec2};
use sanctuary_engine::{EngineRuntimeInfo, run_descriptor};
use std::{path::PathBuf, time::Duration};

fn main() -> eframe::Result {
    let descriptor = descriptor_path().unwrap_or_else(|message| {
        eprintln!("OpenSanctuary engine: {message}");
        std::process::exit(2);
    });
    let runtime = run_descriptor(&descriptor).unwrap_or_else(|error| {
        eprintln!("OpenSanctuary engine: {error}");
        std::process::exit(3);
    });

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OpenSanctuary Engine • Native Runtime")
            .with_inner_size([1200.0, 760.0])
            .with_min_inner_size([900.0, 600.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "org.opensanctuary.Engine",
        options,
        Box::new(move |_cc| Ok(Box::new(EngineApp::new(runtime)))),
    )
}

fn descriptor_path() -> Result<PathBuf, String> {
    let mut args = std::env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == std::ffi::OsStr::new("--descriptor") {
            return args
                .next()
                .map(PathBuf::from)
                .ok_or_else(|| "--descriptor requires a path".into());
        }
    }
    Err("usage: opensanctuary-engine --descriptor <launch.json>".into())
}

struct EngineApp {
    runtime: EngineRuntimeInfo,
}

impl EngineApp {
    fn new(runtime: EngineRuntimeInfo) -> Self {
        Self { runtime }
    }

    fn paint_hero(&self, ui: &mut egui::Ui, rect: Rect) {
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_rgb(8, 11, 18));
        let t = ui.input(|i| i.time) as f32;
        let center = rect.center();

        for ring in 0..8 {
            let radius = 80.0 + ring as f32 * 55.0 + (t * 10.0 + ring as f32 * 7.0).sin() * 4.0;
            painter.circle_stroke(
                center,
                radius,
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(36, 103, 170, 55)),
            );
        }
        for i in 0..72 {
            let phase = i as f32 * 0.618 + t * (0.08 + (i % 7) as f32 * 0.01);
            let x = rect.left() + ((phase.sin() * 0.5 + 0.5) * rect.width());
            let y = rect.top() + (((phase * 1.73).cos() * 0.5 + 0.5) * rect.height());
            let amber = i % 9 == 0;
            let color = if amber {
                Color32::from_rgba_unmultiplied(225, 143, 62, 110)
            } else {
                Color32::from_rgba_unmultiplied(78, 154, 224, 90)
            };
            painter.circle_filled(Pos2::new(x, y), 1.0 + (i % 3) as f32 * 0.45, color);
        }
    }
}

impl eframe::App for EngineApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx().request_repaint_after(Duration::from_millis(16));
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let rect = ui.max_rect();
        self.paint_hero(ui, rect);

        let card = Rect::from_min_size(
            Pos2::new(rect.left() + 56.0, rect.bottom() - 260.0),
            Vec2::new((rect.width() - 112.0).min(820.0), 190.0),
        );
        ui.painter().rect_filled(
            card,
            CornerRadius::same(10),
            Color32::from_rgba_unmultiplied(15, 21, 31, 235),
        );
        ui.painter().rect_stroke(
            card,
            CornerRadius::same(10),
            Stroke::new(1.0, Color32::from_rgb(52, 78, 108)),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            card.left_top() + Vec2::new(24.0, 22.0),
            egui::Align2::LEFT_TOP,
            "OPEN SANCTUARY • NATIVE ENGINE ONLINE",
            FontId::proportional(22.0),
            Color32::from_rgb(217, 232, 245),
        );
        ui.painter().text(
            card.left_top() + Vec2::new(24.0, 66.0),
            egui::Align2::LEFT_TOP,
            format!("Install: {}", self.runtime.install_path.display()),
            FontId::monospace(14.0),
            Color32::from_rgb(159, 180, 201),
        );
        ui.painter().text(
            card.left_top() + Vec2::new(24.0, 96.0),
            egui::Align2::LEFT_TOP,
            format!(
                "Content: {} index files • {} data files",
                self.runtime.index_files, self.runtime.data_files
            ),
            FontId::monospace(14.0),
            Color32::from_rgb(159, 180, 201),
        );
        ui.painter().text(
            card.left_top() + Vec2::new(24.0, 126.0),
            egui::Align2::LEFT_TOP,
            format!(
                "Renderer: wgpu/Vulkan {} • PipeWire {}",
                if self.runtime.vulkan_available {
                    "ready"
                } else {
                    "unavailable"
                },
                if self.runtime.pipewire_available {
                    "ready"
                } else {
                    "unavailable"
                }
            ),
            FontId::monospace(14.0),
            Color32::from_rgb(91, 190, 242),
        );
    }
}
