use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectIndex {
    names: Vec<String>,
}

impl ProjectIndex {
    pub fn discover(downloads: &Path) -> Self {
        let mut by_key = BTreeMap::<String, String>::new();

        if let Ok(entries) = fs::read_dir(downloads) {
            for entry in entries.flatten() {
                let path = entry.path();
                let Ok(meta) = fs::symlink_metadata(&path) else {
                    continue;
                };
                if meta.file_type().is_symlink() || !meta.is_dir() {
                    continue;
                }
                if has_project_marker(&path)
                    && let Some(name) = path.file_name().and_then(|value| value.to_str())
                {
                    insert_name(
                        &mut by_key,
                        derive_family_name(name).unwrap_or_else(|| name.to_owned()),
                    );
                }
            }
        }

        for canonical in [
            downloads.join("Project Files"),
            downloads.join("ForgeClean/Projects"),
        ] {
            if let Ok(entries) = fs::read_dir(&canonical) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let Ok(meta) = fs::symlink_metadata(&path) else {
                        continue;
                    };
                    if meta.file_type().is_symlink()
                        || !meta.is_dir()
                        || !project_root_is_trusted(&path)
                    {
                        continue;
                    }
                    if let Some(name) = path.file_name().and_then(|value| value.to_str()) {
                        insert_name(
                            &mut by_key,
                            derive_family_name(name).unwrap_or_else(|| name.to_owned()),
                        );
                    }
                }
            }
        }

        let mut names = by_key.into_values().collect::<Vec<_>>();
        names.sort_by(|a, b| {
            tokenise(b)
                .len()
                .cmp(&tokenise(a).len())
                .then_with(|| b.len().cmp(&a.len()))
                .then_with(|| a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()))
        });
        Self { names }
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }

    pub fn family_for_artifact_name(&self, name: &str) -> Option<String> {
        let stripped = strip_action_prefixes(name);
        let artifact_tokens = tokenise(stripped);
        if artifact_tokens.is_empty() {
            return None;
        }
        self.names
            .iter()
            .find(|family| {
                let family_tokens = tokenise(family);
                !family_tokens.is_empty()
                    && artifact_tokens.len() >= family_tokens.len()
                    && artifact_tokens[..family_tokens.len()] == family_tokens[..]
            })
            .cloned()
    }

    pub fn family_for_top_level(&self, path: &Path) -> Option<String> {
        let meta = fs::symlink_metadata(path).ok()?;
        if meta.file_type().is_symlink() {
            return None;
        }
        let name = path.file_name()?.to_str()?;
        if meta.is_dir() && has_project_marker(path) {
            return derive_family_name(name).or_else(|| Some(name.to_owned()));
        }
        self.family_for_artifact_name(name)
    }
}

pub fn has_project_marker(dir: &Path) -> bool {
    const MARKERS: &[&str] = &[
        "Cargo.toml",
        "CMakeLists.txt",
        "meson.build",
        "Makefile",
        "package.json",
        "pyproject.toml",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "settings.gradle",
        ".git",
    ];
    MARKERS.iter().any(|marker| {
        fs::symlink_metadata(dir.join(marker))
            .map(|meta| !meta.file_type().is_symlink() && (meta.is_file() || meta.is_dir()))
            .unwrap_or(false)
    })
}

pub fn derive_family_name(name: &str) -> Option<String> {
    let trimmed = strip_known_extensions(name)
        .trim_matches(|ch: char| ch == '-' || ch == '_' || ch.is_whitespace());
    if trimmed.is_empty() {
        return None;
    }
    let mut cut = trimmed.len();
    let lower = trimmed.to_ascii_lowercase();

    for (index, _) in lower.char_indices() {
        let rest = &lower[index..];
        let sep = index == 0
            || lower
                .as_bytes()
                .get(index.wrapping_sub(1))
                .is_some_and(|b| matches!(*b, b'-' | b'_'));
        if !sep {
            continue;
        }
        let token = rest.trim_start_matches(['-', '_']);
        if looks_like_version_token(token)
            || looks_like_milestone_token(token)
            || token.starts_with("milestone")
            || token.starts_with("task")
                && token[4..]
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_ascii_digit())
        {
            cut = cut.min(index);
        }
    }

    for marker in [
        "-verify",
        "_verify",
        "-sha256sums",
        "_sha256sums",
        "-rollback",
        "_rollback",
    ] {
        if let Some(pos) = lower.find(marker) {
            cut = cut.min(pos);
        }
    }

    let candidate =
        trimmed[..cut].trim_matches(|ch: char| ch == '-' || ch == '_' || ch.is_whitespace());
    if candidate.len() < 2 || generic_name(candidate) {
        None
    } else {
        Some(candidate.to_owned())
    }
}

pub fn project_class_for_name(name: &str, source_tree: bool) -> &'static str {
    if source_tree {
        return "SOURCE";
    }
    let lower = name.to_ascii_lowercase();
    if lower.contains("verify")
        || lower.contains("sha256")
        || lower.contains("checksum")
        || lower.ends_with(".log")
    {
        return "EVIDENCE";
    }
    if lower.ends_with(".zip")
        || lower.ends_with(".7z")
        || lower.ends_with(".rar")
        || lower.ends_with(".tar")
        || lower.ends_with(".tar.gz")
        || lower.ends_with(".tar.xz")
        || lower.ends_with(".tar.zst")
        || lower.ends_with(".exe")
        || lower.ends_with(".msi")
        || lower.ends_with(".appimage")
        || lower.ends_with(".pkg.tar.zst")
    {
        return "RELEASES";
    }
    "FILES"
}

pub fn canonical_project_root(downloads: &Path, family: &str) -> PathBuf {
    downloads.join("Project Files").join(family)
}

fn insert_name(map: &mut BTreeMap<String, String>, candidate: String) {
    let Some(name) = derive_family_name(&candidate).or_else(|| {
        let trimmed = candidate.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    }) else {
        return;
    };
    if generic_name(&name) {
        return;
    }
    let key = tokenise(&name).join("\u{1f}");
    if key.is_empty() {
        return;
    }
    map.entry(key).or_insert(name);
}

pub fn project_root_is_trusted(path: &Path) -> bool {
    if has_project_marker(path) || has_project_marker(&path.join("Active")) {
        return true;
    }
    let active = path.join("Active");
    let Ok(entries) = fs::read_dir(active) else {
        return false;
    };
    entries.flatten().any(|entry| {
        fs::symlink_metadata(entry.path())
            .map(|meta| {
                meta.is_dir() && !meta.file_type().is_symlink() && has_project_marker(&entry.path())
            })
            .unwrap_or(false)
    })
}

fn strip_action_prefixes(mut name: &str) -> &str {
    const PREFIXES: &[&str] = &[
        "JOIN-AND-HIT-IT-",
        "BUILD-AND-RUN-",
        "HIT-IT-",
        "UPLOAD-ME-",
        "DIAGNOSE-",
        "FINALIZE-",
        "QUALIFY-",
        "RECOVER-",
        "REPLACE-",
        "ROLLBACK-",
        "INSTALL-",
        "UPDATE-",
        "APPLY-",
        "STATUS-",
        "VERIFY-",
        "TRACE-",
        "COLLECT-",
        "FIX-",
        "RUN-",
    ];
    loop {
        let mut changed = false;
        for prefix in PREFIXES {
            if name.len() >= prefix.len() && name[..prefix.len()].eq_ignore_ascii_case(prefix) {
                name = &name[prefix.len()..];
                changed = true;
                break;
            }
        }
        if !changed {
            return name;
        }
    }
}

fn strip_known_extensions(name: &str) -> &str {
    let lower = name.to_ascii_lowercase();
    for suffix in [
        ".tar.gz.sha256",
        ".tar.xz.sha256",
        ".tar.zst.sha256",
        ".zip.sha256",
        ".sha256",
        ".tar.gz",
        ".tar.xz",
        ".tar.zst",
        ".appimage",
        ".pkg.tar.zst",
        ".json",
        ".yaml",
        ".yml",
        ".toml",
        ".txt",
        ".log",
        ".md",
        ".zip",
        ".7z",
        ".rar",
        ".exe",
        ".msi",
        ".run",
        ".sh",
        ".rs",
        ".crt",
        ".cer",
    ] {
        if lower.ends_with(suffix) {
            return &name[..name.len() - suffix.len()];
        }
    }
    name
}

fn looks_like_version_token(token: &str) -> bool {
    let token = token.strip_prefix('v').unwrap_or(token);
    let mut parts = token.split('.');
    let Some(first) = parts.next() else {
        return false;
    };
    let Some(second) = parts.next() else {
        return false;
    };
    !first.is_empty()
        && !second.is_empty()
        && first.chars().all(|ch| ch.is_ascii_digit())
        && second.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

fn looks_like_milestone_token(token: &str) -> bool {
    let Some(rest) = token.strip_prefix('m') else {
        return false;
    };
    rest.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

fn generic_name(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "readme"
            | "build"
            | "source"
            | "verify"
            | "sha256sums"
            | "downloads"
            | "project"
            | "projects"
            | "rust"
            | "src"
            | "target"
            | "release"
            | "releases"
            | "installer"
            | "install"
    )
}

fn tokenise(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}
