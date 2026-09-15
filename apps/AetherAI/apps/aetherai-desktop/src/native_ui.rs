use crate::app::{AetherApp, DrawCommand};
use aether_ui::{DesktopPage, Rgba};
use std::time::Duration;
const EFRAME_VERSION: &str = "0.31.1";
const EGUI_VERSION: &str = "0.31.1";
const WGPU_VERSION: &str = "24.0.5";
const WINIT_VERSION: &str = "0.30.13";

pub fn diagnostic() {
    let c = aether_ui::current_visual_contract();
    let t = aether_terminal_ui::DragonGlassTheme::canonical();
    println!("AETHERAI_PRIMARY_RENDERER=EFRAME_EGUI_WGPU_WINIT");
    println!("AETHERAI_RENDERER_BACKEND=WGPU");
    println!("AETHERAI_WINDOW_BACKEND=WINIT");
    println!("AETHERAI_THEME_AUTHORITY=AETHERFORGE_TERMINAL_10.2.21_RUNTIME");
    println!("AETHERAI_THEME_DRIFT=ZERO");
    println!("AETHERAI_RENDERER_EFRAME={EFRAME_VERSION}");
    println!("AETHERAI_RENDERER_EGUI={EGUI_VERSION}");
    println!("AETHERAI_RENDERER_WGPU={WGPU_VERSION}");
    println!("AETHERAI_RENDERER_WINIT={WINIT_VERSION}");
    println!("AETHERAI_TITLEBAR_HEIGHT={}", t.decoration.title_height);
    println!("AETHERAI_WINDOW_CONTROL_SIZE={}", t.decoration.button_width);
    println!(
        "AETHERAI_WINDOW_CONTROL_SPACING={}",
        t.decoration.button_spacing
    );
    println!(
        "AETHERAI_WINDOW_CONTROL_LEFT_INSET={}",
        t.decoration.title_border_left
    );
    println!("AETHERAI_SURFACE_ALPHA={}", t.background.a);
    println!("AETHERAI_CONTRACT_SCHEMA={}", c.schema_id);
    println!("AETHERAI_WHOLE_WINDOW_OPACITY=DISABLED");
    println!("AETHERAI_TRANSPARENT_FRAMEBUFFER_CLEAR=YES");
    println!("AETHERAI_NORMAL_USER_TERMINAL_REQUIRED=NO");
}

pub fn run(app: AetherApp) -> Result<(), Box<dyn std::error::Error>> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("AetherAI — DragonGlass")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([960.0, 640.0])
            .with_decorations(false)
            .with_transparent(true),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "AetherAI — DragonGlass",
        options,
        Box::new(move |cc| {
            let mut v = egui::Visuals::dark();
            v.panel_fill = egui::Color32::TRANSPARENT;
            v.window_fill = egui::Color32::TRANSPARENT;
            v.extreme_bg_color = egui::Color32::TRANSPARENT;
            v.faint_bg_color = egui::Color32::TRANSPARENT;
            cc.egui_ctx.set_visuals(v);
            Ok(Box::new(NativeAetherApp {
                app,
                maximized: false,
                drop_hover: false,
            }))
        }),
    )?;
    Ok(())
}

struct NativeAetherApp {
    app: AetherApp,
    maximized: bool,
    drop_hover: bool,
}
impl NativeAetherApp {
    fn input(&mut self, ctx: &egui::Context) {
        let (events, clicked, pos, size, scroll, dropped_paths, hovered_files) = ctx.input(|i| {
            (
                i.events.clone(),
                i.pointer.primary_clicked(),
                i.pointer.interact_pos(),
                i.screen_rect().size(),
                i.smooth_scroll_delta,
                i.raw
                    .dropped_files
                    .iter()
                    .filter_map(|file| file.path.clone())
                    .collect::<Vec<_>>(),
                !i.raw.hovered_files.is_empty(),
            )
        });
        self.drop_hover = hovered_files;
        if !dropped_paths.is_empty() {
            self.app.native_attach_dropped_paths(dropped_paths);
        }
        for e in events {
            match e {
                egui::Event::Text(s) | egui::Event::Paste(s) => self.app.handle_text(&s),
                egui::Event::Key {
                    key: egui::Key::Backspace,
                    pressed: true,
                    ..
                } => self.app.backspace(),
                egui::Event::Key {
                    key: egui::Key::ArrowLeft,
                    pressed: true,
                    modifiers,
                    ..
                } => self.app.cursor_left(modifiers.ctrl),
                egui::Event::Key {
                    key: egui::Key::ArrowRight,
                    pressed: true,
                    modifiers,
                    ..
                } => self.app.cursor_right(modifiers.ctrl),
                egui::Event::Key {
                    key: egui::Key::Home,
                    pressed: true,
                    ..
                } => self.app.cursor_home(),
                egui::Event::Key {
                    key: egui::Key::End,
                    pressed: true,
                    ..
                } => self.app.cursor_end(),
                egui::Event::Key {
                    key: egui::Key::Delete,
                    pressed: true,
                    ..
                } => self.app.delete_forward(),
                egui::Event::Key {
                    key: egui::Key::Enter,
                    pressed: true,
                    modifiers,
                    ..
                } => self.app.enter(modifiers.shift),
                egui::Event::Key {
                    key: egui::Key::C,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.ctrl && self.app.native_active_page() == DesktopPage::Terminal => {
                    self.app.native_ctrl_c()
                }
                _ => {}
            }
        }
        if self.app.native_active_page() == DesktopPage::Terminal && scroll.y.abs() >= 0.5 {
            self.app
                .native_terminal_scroll((scroll.y / 18.0).round() as i32);
        }
        if clicked && let Some(p) = pos {
            let t = aether_terminal_ui::DragonGlassTheme::canonical();
            if p.y >= t.decoration.title_height {
                self.app.native_pointer_click(
                    p.x.round() as i32,
                    p.y.round() as i32,
                    size.x.round().max(1.0) as i32,
                    size.y.round().max(1.0) as i32,
                );
            }
        }
    }
    fn commands(&self, p: &egui::Painter, o: egui::Pos2, cmds: &[DrawCommand]) {
        let mono = self.app.native_active_page() == DesktopPage::Terminal;
        for c in cmds {
            match c {
                DrawCommand::Rect {
                    rect,
                    color,
                    radius,
                } => {
                    let r = egui::Rect::from_min_size(
                        egui::pos2(o.x + rect.x as f32, o.y + rect.y as f32),
                        egui::vec2(rect.w.max(0) as f32, rect.h.max(0) as f32),
                    );
                    p.rect_filled(
                        r,
                        egui::CornerRadius::same((*radius).clamp(0, 255) as u8),
                        shared(*color),
                    );
                }
                DrawCommand::Text { x, y, text, color } => {
                    p.text(
                        egui::pos2(o.x + *x as f32, o.y + *y as f32),
                        egui::Align2::LEFT_BOTTOM,
                        text,
                        if mono {
                            egui::FontId::monospace(14.0)
                        } else {
                            egui::FontId::proportional(14.0)
                        },
                        shared(*color),
                    );
                }
            }
        }
    }
    fn chrome(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let t = aether_terminal_ui::DragonGlassTheme::canonical();
        let d = t.decoration;
        let controls = [
            (
                "aetherai-close",
                "×",
                d.close.active,
                egui::ViewportCommand::Close,
            ),
            (
                "aetherai-maximize",
                "□",
                d.maximize.active,
                egui::ViewportCommand::Maximized(!self.maximized),
            ),
            (
                "aetherai-minimize",
                "—",
                d.minimize.active,
                egui::ViewportCommand::Minimized(true),
            ),
        ];
        for (index, (id, glyph, fill, command)) in controls.into_iter().enumerate() {
            let x = d.title_border_left + index as f32 * (d.button_width + d.button_spacing);
            let r = egui::Rect::from_min_size(
                egui::pos2(x, d.button_margin_top),
                egui::vec2(d.button_width, d.button_height),
            );
            let resp = ui.interact(r, egui::Id::new(id), egui::Sense::click());
            ui.painter().rect_filled(
                r,
                egui::CornerRadius::same((d.button_width * 0.5).round() as u8),
                term(fill),
            );
            ui.painter().text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                glyph,
                egui::FontId::monospace(11.0),
                term(d.active_title_text),
            );
            if resp.clicked() {
                if id == "aetherai-maximize" {
                    self.maximized = !self.maximized;
                }
                ctx.send_viewport_cmd(command);
            }
        }
        let cw = 3.0 * d.button_width + 2.0 * d.button_spacing;
        let nr = egui::Rect::from_min_size(
            egui::pos2(d.title_border_left + cw + 7.0, 5.0),
            egui::vec2(18.0, 18.0),
        );
        let n = ui.interact(
            nr,
            egui::Id::new("aetherai-nav-toggle"),
            egui::Sense::click(),
        );
        ui.painter().text(
            nr.center(),
            egui::Align2::CENTER_CENTER,
            "◆",
            egui::FontId::monospace(10.0),
            term(t.label_text),
        );
        if n.clicked() {
            self.app.native_toggle_navigation();
        }
        let drag = egui::Rect::from_min_max(
            egui::pos2(d.title_border_left + cw + 36.0, 0.0),
            egui::pos2(ui.max_rect().right() - 8.0, d.title_height),
        );
        let dr = ui.interact(
            drag,
            egui::Id::new("aetherai-title-drag"),
            egui::Sense::click_and_drag(),
        );
        if dr.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        if dr.double_clicked() {
            self.maximized = !self.maximized;
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(self.maximized));
        }
    }
}
impl eframe::App for NativeAetherApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.app.native_first_frame_startup_health();

        if self.app.tick() {
            ctx.request_repaint();
        }
        self.input(ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let v = ui.max_rect();
                let w = v.width().round().max(1.0) as i32;
                let h = v.height().round().max(1.0) as i32;
                let cmds = self.app.render(w, h);
                let p = ui.painter_at(v);
                self.commands(&p, v.min, &cmds);
                self.chrome(ui, ctx);
                if self.drop_hover {
                    let theme = aether_terminal_ui::DragonGlassTheme::canonical();
                    let overlay = v.shrink(34.0);
                    p.rect_filled(overlay, egui::CornerRadius::same(18), term(theme.panel));
                    p.rect_stroke(
                        overlay,
                        egui::CornerRadius::same(18),
                        egui::Stroke::new(2.0_f32, term(theme.interaction_cyan)),
                        egui::StrokeKind::Inside,
                    );
                    p.text(
                        overlay.center(),
                        egui::Align2::CENTER_CENTER,
                        "DROP FILES OR FOLDERS TO ATTACH",
                        egui::FontId::proportional(20.0),
                        term(theme.value_text),
                    );
                }
            });
        if self.app.native_should_quit() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(16));
    }
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }
}
fn shared(c: Rgba) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
}
fn term(c: aether_terminal_ui::Rgba) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c.r, c.g, c.b, c.a)
}
