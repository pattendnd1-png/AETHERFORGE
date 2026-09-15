pub mod git_context;
pub use git_context::{
    GitCommitSummary, GitContextError, GitContextSnapshot, GitContextState,
    PermissionedGitContextProvider, git_context_snapshot,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectKind {
    RustCargo,
    Node,
    Python,
    CMake,
    Meson,
    Git,
    SourceTree,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProvenanceClass {
    Confirmed,
    Likely,
    Uncertain,
    NormalProject,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryEvidence {
    pub score: u8,
    pub reason: String,
    pub explicit: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectCandidate {
    pub root: PathBuf,
    pub name: String,
    pub kind: ProjectKind,
    pub provenance: ProvenanceClass,
    pub evidence: Vec<DiscoveryEvidence>,
}
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    pub max_depth: usize,
    pub follow_symlinks: bool,
    pub exclusions: Vec<String>,
}
impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            max_depth: 8,
            follow_symlinks: false,
            exclusions: vec![
                "target".into(),
                "node_modules".into(),
                ".git/objects".into(),
                ".venv".into(),
                "venv".into(),
                "__pycache__".into(),
                ".cache".into(),
                ".ssh".into(),
                ".gnupg".into(),
                "password-store".into(),
            ],
        }
    }
}
#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
pub fn scan_root(
    root: &Path,
    config: &DiscoveryConfig,
) -> Result<Vec<ProjectCandidate>, WorkspaceError> {
    let mut out = Vec::new();
    walk(root, root, 0, config, &mut out)?;
    out.sort_by(|a, b| a.root.cmp(&b.root));
    out.dedup_by(|a, b| a.root == b.root);
    Ok(out)
}
fn walk(
    scan_root: &Path,
    dir: &Path,
    depth: usize,
    cfg: &DiscoveryConfig,
    out: &mut Vec<ProjectCandidate>,
) -> Result<(), WorkspaceError> {
    if depth > cfg.max_depth {
        return Ok(());
    }
    let rel = dir
        .strip_prefix(scan_root)
        .unwrap_or(dir)
        .to_string_lossy()
        .replace('\\', "/");
    if excluded(&rel, cfg) {
        return Ok(());
    }
    let meta = fs::symlink_metadata(dir)?;
    if meta.file_type().is_symlink() && !cfg.follow_symlinks {
        return Ok(());
    }
    if let Some(kind) = detect_kind(dir) {
        let evidence = collect_evidence(dir);
        let score: i32 = evidence.iter().map(|e| e.score as i32).sum();
        let explicit = evidence.iter().any(|e| e.explicit);
        let provenance = if score >= 8 && explicit {
            ProvenanceClass::Confirmed
        } else if score >= 5 {
            ProvenanceClass::Likely
        } else if score >= 1 {
            ProvenanceClass::Uncertain
        } else {
            ProvenanceClass::NormalProject
        };
        out.push(ProjectCandidate {
            root: dir.to_path_buf(),
            name: dir
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("Project")
                .to_string(),
            kind,
            provenance,
            evidence,
        });
    }
    for entry in match fs::read_dir(dir) {
        Ok(v) => v,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return Ok(()),
        Err(e) => return Err(e.into()),
    } {
        let entry = entry?;
        let p = entry.path();
        if entry.file_type()?.is_dir() {
            walk(scan_root, &p, depth + 1, cfg, out)?;
        }
    }
    Ok(())
}
fn excluded(rel: &str, cfg: &DiscoveryConfig) -> bool {
    let rel = rel.trim_matches('/');
    cfg.exclusions
        .iter()
        .any(|x| rel == x || rel.starts_with(&format!("{x}/")) || rel.split('/').any(|p| p == x))
}
fn detect_kind(d: &Path) -> Option<ProjectKind> {
    if d.join("Cargo.toml").is_file() {
        Some(ProjectKind::RustCargo)
    } else if d.join("package.json").is_file() {
        Some(ProjectKind::Node)
    } else if d.join("pyproject.toml").is_file() || d.join("requirements.txt").is_file() {
        Some(ProjectKind::Python)
    } else if d.join("CMakeLists.txt").is_file() {
        Some(ProjectKind::CMake)
    } else if d.join("meson.build").is_file() {
        Some(ProjectKind::Meson)
    } else if d.join(".git").is_dir() {
        Some(ProjectKind::Git)
    } else {
        let count = fs::read_dir(d)
            .ok()?
            .filter_map(Result::ok)
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|v| v.to_str())
                    .is_some_and(|x| matches!(x, "rs" | "py" | "js" | "ts" | "c" | "cpp" | "go"))
            })
            .take(4)
            .count();
        (count >= 3).then_some(ProjectKind::SourceTree)
    }
}
fn collect_evidence(d: &Path) -> Vec<DiscoveryEvidence> {
    let mut out = vec![];
    let entries = match fs::read_dir(d) {
        Ok(v) => v,
        Err(_) => return out,
    };
    for e in entries.filter_map(Result::ok) {
        let p = e.path();
        if !p.is_file() {
            continue;
        }
        let n = p
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if n.contains("verify") && (n.contains("aetherai") || n.contains("aetherforge")) {
            out.push(DiscoveryEvidence {
                score: 4,
                reason: format!("versioned Aether verification artifact: {}", p.display()),
                explicit: false,
            });
        }
        if n.contains("chatgpt") || n.contains("openai") || n.contains("aetherai") {
            out.push(DiscoveryEvidence {
                score: 5,
                reason: format!("explicit AI project metadata: {}", p.display()),
                explicit: true,
            });
        }
        if matches!(
            n.as_str(),
            "readme.md" | "changelog.md" | "notes.md" | "start-here.txt"
        ) {
            if let Ok(mut text) = fs::read_to_string(&p) {
                text.truncate(text.len().min(131072));
                let low = text.to_ascii_lowercase();
                if low.contains("chatgpt") || low.contains("openai") || low.contains("aetherai") {
                    out.push(DiscoveryEvidence {
                        score: 2,
                        reason: format!(
                            "project documentation references AI assistance: {}",
                            p.display()
                        ),
                        explicit: true,
                    });
                }
                if low.contains("hit it")
                    || low.contains("continue from")
                    || low.contains("assistant-generated")
                {
                    out.push(DiscoveryEvidence {
                        score: 3,
                        reason: format!("assistant continuation/build notes: {}", p.display()),
                        explicit: false,
                    });
                }
            }
        }
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    fn td() -> PathBuf {
        let p = std::env::temp_dir().join(format!("aether-workspace-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn cargo_with_verify_is_ai_assisted() {
        let d = td();
        fs::write(d.join("Cargo.toml"), "[workspace]\nmembers=[]\n").unwrap();
        fs::write(
            d.join("AetherAI-v0.1.9-VERIFY.txt"),
            "AETHERAI_V0_1_9_VERIFY=PASS",
        )
        .unwrap();
        let f = scan_root(&d, &DiscoveryConfig::default()).unwrap();
        assert_eq!(f[0].kind, ProjectKind::RustCargo);
        assert!(matches!(
            f[0].provenance,
            ProvenanceClass::Likely | ProvenanceClass::Confirmed
        ));
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn excludes_target_and_git_objects() {
        let d = td();
        fs::create_dir_all(d.join("target/noise")).unwrap();
        fs::write(d.join("target/noise/Cargo.toml"), "[workspace]").unwrap();
        fs::create_dir_all(d.join("real")).unwrap();
        fs::write(d.join("real/Cargo.toml"), "[workspace]").unwrap();
        let f = scan_root(&d, &DiscoveryConfig::default()).unwrap();
        assert_eq!(f.len(), 1);
        assert!(f[0].root.ends_with("real"));
        let _ = fs::remove_dir_all(d);
    }
    #[test]
    fn no_symlink_follow_by_default() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let d = td();
            let out = td();
            fs::write(out.join("Cargo.toml"), "[workspace]").unwrap();
            symlink(&out, d.join("linked")).unwrap();
            assert!(
                scan_root(&d, &DiscoveryConfig::default())
                    .unwrap()
                    .is_empty()
            );
            let _ = fs::remove_dir_all(d);
            let _ = fs::remove_dir_all(out);
        }
    }
}
