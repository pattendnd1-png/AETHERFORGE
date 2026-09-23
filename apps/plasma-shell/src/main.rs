//! AetherForge Plasma Shell Layer
//! Offline KWin configuration generator (no D-Bus required)

mod config;

use config::{kwin, theme, performance};

fn main() {
    println!("========================================");
    println!("  AetherForge Plasma Shell v1.0.0");
    println!("  Config Generator (Offline Mode)");
    println!("========================================");
    println!();
    
    // Detect performance mode
    let perf = performance::PerformanceMode::detect();
    println!("📊 Detected: {} RAM, Mode: {}", 
             if perf.ram_total_gb >= 8 { "8GB+" } 
             else if perf.ram_total_gb >= 4 { "4GB+" }
             else { "<4GB" },
             perf.mode);
    
    // Choose config based on mode
    let kwin_cfg = if perf.should_disable_effects() {
        println!("⚡ Performance mode: disabling effects");
        kwin::KWinConfig::light_mode()
    } else {
        println!("✨ Full mode: enabling visual effects");
        kwin::KWinConfig::default()
    };
    
    // Determine output path
    let home = dirs::home_dir().expect("Cannot determine home directory");
    let config_dir = home.join(".config");
    
    // Write KWin config
    let kwinrc_path = config_dir.join("kwinrc");
    kwin_cfg.save(&kwinrc_path).expect("Failed to write kwinrc");
    println!("✅ KWin config written to: {}", kwinrc_path.display());
    
    // Apply theme
    let current_theme = if perf.should_disable_effects() {
        theme::AetherTheme::void_light()
    } else {
        theme::AetherTheme::dragon_glass()
    };
    
    let theme_dir = home.join(".local/share/plasma/desktoptheme");
    current_theme.write_metadata(&theme_dir.join("aetherforge").join("metadata.desktop"))
        .expect("Failed to write theme metadata");
    println!("✅ Theme metadata written to: {}", theme_dir.display());
    
    println!();
    println!("✅ Plasma shell configuration generated");
    println!("💡 To apply: Log out and log back in, or restart KDE session");
}
