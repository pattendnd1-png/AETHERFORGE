//! AetherForge Plasma theme configuration

use std::fs;
use std::io::Write;
use std::path::Path;

pub struct AetherTheme {
    pub name: String,
    pub primary_color: String,
    pub secondary_color: String,
    #[allow(dead_code)]
    pub accent_color: String,
    pub neon_glow: bool,
}

impl AetherTheme {
    pub fn dragon_glass() -> Self {
        Self {
            name: "DragonGlass".to_string(),
            primary_color: "#6d4aff".to_string(),
            secondary_color: "#00d4ff".to_string(),
            accent_color: "#ff006e".to_string(),
            neon_glow: true,
        }
    }
    
    pub fn void_light() -> Self {
        Self {
            name: "VoidLight".to_string(),
            primary_color: "#2c3e50".to_string(),
            secondary_color: "#34495e".to_string(),
            accent_color: "#3498db".to_string(),
            neon_glow: false,
        }
    }
    
    pub fn write_metadata(&self, path: &Path) -> Result<(), std::io::Error> {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        
        let content = format!(
            "[Desktop Entry]\n\
             Name={}\n\
             Comment=AetherForge {} theme\n\
             Type=Theme\n\n\
             [Plasma Desktop Theme]\n\
             name={}\n\
             primaryColor={}\n\
             secondaryColor={}\n",
            self.name,
            if self.neon_glow { "neon" } else { "minimal" },
            self.name,
            self.primary_color,
            self.secondary_color
        );
        
        fs::write(path, content)?;
        Ok(())
    }
}
