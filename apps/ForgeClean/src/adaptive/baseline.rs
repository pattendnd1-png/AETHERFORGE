use crate::adaptive::benchmark::BenchmarkReport;
use crate::adaptive::store::AuditStore;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct BaselineStore {
    store: AuditStore,
}

impl BaselineStore {
    pub fn for_home(home: &Path) -> Self {
        Self {
            store: AuditStore::for_home(home),
        }
    }

    pub fn create(&self, name: &str, report: &BenchmarkReport) -> io::Result<PathBuf> {
        validate_name(name)?;
        let dir = self.store.baselines_dir();
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{name}.json"));
        let data = serde_json::to_vec_pretty(report).map_err(io::Error::other)?;
        fs::write(&path, data)?;
        Ok(path)
    }

    pub fn load(&self, name: &str) -> io::Result<BenchmarkReport> {
        validate_name(name)?;
        let path = self.store.baselines_dir().join(format!("{name}.json"));
        let data = fs::read(path)?;
        serde_json::from_slice(&data).map_err(io::Error::other)
    }
}

fn validate_name(name: &str) -> io::Result<()> {
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "baseline name may contain only letters, digits, dash and underscore",
        ));
    }
    Ok(())
}
