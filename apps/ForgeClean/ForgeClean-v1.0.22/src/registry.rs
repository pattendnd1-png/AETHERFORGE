use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRecord {
    pub name: String,
    pub active_path: PathBuf,
    pub legacy_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectRegistry {
    pub records: Vec<ProjectRecord>,
}

impl ProjectRegistry {
    pub fn read(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        let mut records = Vec::new();
        for (line_no, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let mut fields = line.splitn(3, '\t');
            let name = fields.next().unwrap_or("");
            let active = fields
                .next()
                .ok_or_else(|| format!("registry line {} missing active path", line_no + 1))?;
            let legacy = fields.next().unwrap_or("");
            if name.is_empty() || active.is_empty() {
                return Err(format!("invalid registry line {}", line_no + 1));
            }
            let legacy_paths = if legacy.is_empty() {
                Vec::new()
            } else {
                legacy.split('|').map(PathBuf::from).collect()
            };
            records.push(ProjectRecord {
                name: name.to_owned(),
                active_path: PathBuf::from(active),
                legacy_paths,
            });
        }
        Ok(Self { records })
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = String::new();
        for record in &self.records {
            validate_field(&record.name)?;
            let active = record.active_path.to_string_lossy();
            validate_field(&active)?;
            let legacy: Vec<String> = record
                .legacy_paths
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            for value in &legacy {
                validate_field(value)?;
            }
            out.push_str(&format!(
                "{}\t{}\t{}\n",
                record.name,
                active,
                legacy.join("|")
            ));
        }
        fs::write(path, out).map_err(|e| e.to_string())
    }

    pub fn set_active(&mut self, name: &str, active: PathBuf, legacy: Vec<PathBuf>) {
        if let Some(record) = self
            .records
            .iter_mut()
            .find(|record| record.name.eq_ignore_ascii_case(name))
        {
            record.name = name.to_owned();
            record.active_path = active;
            for path in legacy {
                if !record.legacy_paths.contains(&path) {
                    record.legacy_paths.push(path);
                }
            }
            record.legacy_paths.sort();
        } else {
            self.records.push(ProjectRecord {
                name: name.to_owned(),
                active_path: active,
                legacy_paths: legacy,
            });
        }
        self.records.sort_by(|a, b| {
            a.name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase())
        });
    }

    pub fn resolve(&self, name: &str) -> Option<&ProjectRecord> {
        self.records
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(name))
    }
}

fn validate_field(value: &str) -> Result<(), String> {
    if value.contains(['\n', '\r', '\t', '|']) {
        Err("registry field contains reserved separator".to_owned())
    } else {
        Ok(())
    }
}

pub fn create_legacy_alias(downloads: &Path, legacy: &Path, active: &Path) -> Result<(), String> {
    let downloads = fs::canonicalize(downloads).map_err(|e| e.to_string())?;
    let active = fs::canonicalize(active).map_err(|e| e.to_string())?;
    let legacy_parent = legacy
        .parent()
        .ok_or_else(|| "legacy path has no parent".to_owned())?;
    let legacy_parent = fs::canonicalize(legacy_parent).map_err(|e| e.to_string())?;
    if !legacy_parent.starts_with(&downloads) {
        return Err("legacy alias must stay under Downloads".to_owned());
    }
    if !active.starts_with(downloads.join("ForgeClean")) {
        return Err("active project path is outside ForgeClean root".to_owned());
    }
    match fs::symlink_metadata(legacy) {
        Ok(meta) if meta.file_type().is_symlink() => {
            let current = fs::read_link(legacy).map_err(|e| e.to_string())?;
            if current == active {
                return Ok(());
            }
            Err(format!(
                "legacy symlink already points elsewhere: {}",
                legacy.display()
            ))
        }
        Ok(_) => Err(format!(
            "refusing to overwrite real legacy path: {}",
            legacy.display()
        )),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            symlink(&active, legacy).map_err(|e| e.to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}
