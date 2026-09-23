//! KWin compositor configuration for AetherForge

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct KWinConfig {
    pub effects: Effects,
    pub opengl: OpenGL,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Effects {
    pub blur: bool,
    pub blur_radius: u8,
    pub transparency: u8,
    pub animations: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenGL {
    pub force_compositioning: bool,
    pub fps_limit: u32,
}

impl Default for KWinConfig {
    fn default() -> Self {
        Self {
            effects: Effects {
                blur: true,
                blur_radius: 12,
                transparency: 90,
                animations: true,
            },
            opengl: OpenGL {
                force_compositioning: true,
                fps_limit: 60,
            },
        }
    }
}

impl KWinConfig {
    pub fn light_mode() -> Self {
        Self {
            effects: Effects {
                blur: false,
                blur_radius: 0,
                transparency: 100,
                animations: false,
            },
            ..Default::default()
        }
    }
    
    pub fn save(&self, path: &PathBuf) -> Result<(), std::io::Error> {
        let mut file = fs::File::create(path)?;
        
        writeln!(file, "[Effects]")?;
        writeln!(file, "Enabled={}", self.effects.blur)?;
        writeln!(file, "BlurRadius={}", self.effects.blur_radius)?;
        writeln!(file, "Transparency={}", self.effects.transparency)?;
        writeln!(file, "Animations={}", self.effects.animations)?;
        writeln!(file)?;
        writeln!(file, "[OpenGL]")?;
        writeln!(file, "ForceCompositionPipeline={}", self.opengl.force_compositioning)?;
        writeln!(file, "FPSLimit={}", self.opengl.fps_limit)?;
        
        Ok(())
    }
}
