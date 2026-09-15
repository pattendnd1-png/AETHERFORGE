use forgeclean::gui::{DRAGONGLASS_WINDOW_ALPHA, ForgeCleanGui};

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args
        .iter()
        .any(|arg| arg == "--version" || arg == "version")
    {
        println!("ForgeClean GUI v1.0.1");
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--self-test") {
        match ForgeCleanGui::self_test() {
            Ok(()) => {
                println!("FORGECLEAN_GUI_SELF_TEST=PASS");
                println!("FORGECLEAN_GUI_VERSION=1.0.1");
                println!("FORGECLEAN_DRAGONGLASS_ALPHA={DRAGONGLASS_WINDOW_ALPHA}");
                return Ok(());
            }
            Err(error) => {
                eprintln!("FORGECLEAN_GUI_SELF_TEST=FAIL");
                eprintln!("ERROR={error}");
                std::process::exit(1);
            }
        }
    }

    let native_options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("ForgeClean · AetherForge")
            .with_app_id("org.aetherforge.forgeclean")
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([960.0, 640.0])
            .with_transparent(true)
            .with_decorations(false),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "org.aetherforge.forgeclean",
        native_options,
        Box::new(|cc| Ok(Box::new(ForgeCleanGui::new(cc)))),
    )
}
