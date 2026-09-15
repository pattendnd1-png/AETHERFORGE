use crate::adaptive::health::{SystemHealth, unix_ms};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AuditStore {
    root: PathBuf,
}

impl AuditStore {
    pub fn for_home(home: &Path) -> Self {
        Self {
            root: home.join("Downloads/ForgeClean/Audits"),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn current_dir(&self) -> PathBuf {
        self.root.join("Current")
    }

    pub fn baselines_dir(&self) -> PathBuf {
        self.root.join("Baselines")
    }

    pub fn write_current_health(&self, health: &SystemHealth) -> io::Result<PathBuf> {
        self.write_current("system-health.json", health)
    }

    pub fn write_history_health(&self, health: &SystemHealth) -> io::Result<PathBuf> {
        self.write_history("audit.json", health)
    }

    pub fn write_current<T: Serialize>(&self, name: &str, value: &T) -> io::Result<PathBuf> {
        validate_leaf(name)?;
        let path = self.current_dir().join(name);
        self.write_json_atomic(&path, value)?;
        Ok(path)
    }

    pub fn write_history<T: Serialize>(&self, name: &str, value: &T) -> io::Result<PathBuf> {
        validate_leaf(name)?;
        let dir = self.root.join("History").join(unix_ms().to_string());
        fs::create_dir_all(&dir)?;
        let path = dir.join(name);
        self.write_json_atomic(&path, value)?;
        Ok(path)
    }

    pub fn read_current<T: DeserializeOwned>(&self, name: &str) -> io::Result<T> {
        validate_leaf(name)?;
        let bytes = fs::read(self.current_dir().join(name))?;
        serde_json::from_slice(&bytes).map_err(io::Error::other)
    }

    fn write_json_atomic<T: Serialize>(&self, path: &Path, value: &T) -> io::Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("output has no parent"))?;
        fs::create_dir_all(parent)?;
        let tmp = parent.join(format!(".{}.{}.tmp", std::process::id(), unix_ms()));
        let data = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
        fs::write(&tmp, data)?;
        fs::rename(tmp, path)
    }
}

fn validate_leaf(name: &str) -> io::Result<()> {
    let p = Path::new(name);
    if name.is_empty() || p.components().count() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "output name must be one path component",
        ));
    }
    Ok(())
}
