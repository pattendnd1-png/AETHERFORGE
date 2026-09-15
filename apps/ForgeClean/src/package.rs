use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const PACKAGE_SUFFIXES: &[&str] = &[".pkg.tar.zst", ".pkg.tar.xz", ".pkg.tar.gz", ".pkg.tar.lz4"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageArchive {
    pub path: PathBuf,
    pub name: String,
    /// Arch package version including pkgrel, e.g. `6.12.10.arch1-1`.
    pub version: String,
    pub arch: String,
}

impl PackageArchive {
    pub fn for_test(name: &str, version: &str, arch: &str) -> Self {
        Self {
            path: PathBuf::from(format!("{name}-{version}-{arch}.pkg.tar.zst")),
            name: name.to_owned(),
            version: version.to_owned(),
            arch: arch.to_owned(),
        }
    }
}

pub trait VersionComparator {
    fn compare(&self, a: &str, b: &str) -> Result<Ordering, String>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ArchVersionComparator;

impl VersionComparator for ArchVersionComparator {
    fn compare(&self, a: &str, b: &str) -> Result<Ordering, String> {
        let output = Command::new("vercmp")
            .arg(a)
            .arg(b)
            .output()
            .map_err(|e| format!("cannot run Arch vercmp: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "vercmp failed with status {}",
                output.status.code().unwrap_or(-1)
            ));
        }
        let text = String::from_utf8(output.stdout)
            .map_err(|e| format!("vercmp returned invalid UTF-8: {e}"))?;
        let value: i32 = text
            .trim()
            .parse()
            .map_err(|e| format!("invalid vercmp result {:?}: {e}", text.trim()))?;
        Ok(value.cmp(&0))
    }
}

pub fn installed_versions_from_pacman() -> Result<BTreeMap<String, String>, String> {
    let output = Command::new("pacman")
        .arg("-Q")
        .output()
        .map_err(|e| format!("cannot run pacman -Q: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "pacman -Q failed with status {}",
            output.status.code().unwrap_or(-1)
        ));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|e| format!("pacman -Q returned invalid UTF-8: {e}"))?;
    let mut installed = BTreeMap::new();
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let Some(name) = fields.next() else { continue };
        let Some(version) = fields.next() else {
            continue;
        };
        installed.insert(name.to_owned(), version.to_owned());
    }
    Ok(installed)
}

pub fn parse_package_filename(path: &Path) -> Option<PackageArchive> {
    let file_name = path.file_name()?.to_str()?;
    let stem = PACKAGE_SUFFIXES
        .iter()
        .find_map(|suffix| file_name.strip_suffix(suffix))?;

    // Pacman archive format is pkgname-pkgver-pkgrel-arch.pkg.tar.*.
    // Package names may contain hyphens, so parse the three right-most fields.
    let mut parts = stem.rsplitn(4, '-');
    let arch = parts.next()?;
    let pkgrel = parts.next()?;
    let pkgver = parts.next()?;
    let name = parts.next()?;
    if name.is_empty() || pkgver.is_empty() || pkgrel.is_empty() || arch.is_empty() {
        return None;
    }

    Some(PackageArchive {
        path: path.to_path_buf(),
        name: name.to_owned(),
        version: format!("{pkgver}-{pkgrel}"),
        arch: arch.to_owned(),
    })
}

pub fn package_signature_archive_path(path: &Path) -> Option<PathBuf> {
    let file_name = path.file_name()?.to_str()?;
    let archive_name = file_name.strip_suffix(".sig")?;
    let archive_path = path.with_file_name(archive_name);
    parse_package_filename(&archive_path)?;
    Some(archive_path)
}

pub fn is_package_signature(path: &Path) -> bool {
    package_signature_archive_path(path).is_some()
}

pub fn signature_path_for_archive(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(".sig");
    PathBuf::from(value)
}

pub fn is_package_partial(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
        return false;
    };
    let Some(base) = name
        .strip_suffix(".part")
        .or_else(|| name.strip_suffix(".partial"))
    else {
        return false;
    };
    PACKAGE_SUFFIXES.iter().any(|suffix| base.ends_with(suffix))
}

pub fn select_superseded(
    archives: Vec<PackageArchive>,
    keep: usize,
    comparator: &dyn VersionComparator,
) -> Result<Vec<PackageArchive>, String> {
    if keep == 0 {
        return Err("keep must be at least 1".to_owned());
    }

    let mut groups: BTreeMap<(String, String), Vec<PackageArchive>> = BTreeMap::new();
    for archive in archives {
        groups
            .entry((archive.name.clone(), archive.arch.clone()))
            .or_default()
            .push(archive);
    }

    let mut superseded = Vec::new();
    for ((_name, _arch), mut group) in groups {
        // Insertion sort lets the comparator fail closed instead of hiding errors
        // inside an infallible sort closure.
        for i in 1..group.len() {
            let mut j = i;
            while j > 0 {
                if comparator.compare(&group[j - 1].version, &group[j].version)?
                    == Ordering::Greater
                {
                    group.swap(j - 1, j);
                    j -= 1;
                } else {
                    break;
                }
            }
        }

        if group.len() > keep {
            let remove_count = group.len() - keep;
            superseded.extend(group.drain(..remove_count));
        }
    }

    superseded.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(superseded)
}
