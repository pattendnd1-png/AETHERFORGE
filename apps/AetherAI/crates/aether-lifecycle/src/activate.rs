use crate::{
    LifecycleError, ReleaseInfo, StagedRelease, VerificationReport,
    verify::{staged_from_root, verify_release_dir},
};
use semver::Version;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct FsLifecycleManager {
    root: PathBuf,
    expected_arch: String,
}

impl FsLifecycleManager {
    pub fn new(root: PathBuf, expected_arch: impl Into<String>) -> Self {
        Self {
            root,
            expected_arch: expected_arch.into(),
        }
    }

    pub fn for_user() -> Result<Self, LifecycleError> {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .ok_or(LifecycleError::HomeUnavailable)?;

        Ok(Self::new(
            PathBuf::from(home).join(".local/lib/aetherforge/aetherai"),
            std::env::consts::ARCH,
        ))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn expected_arch(&self) -> &str {
        &self.expected_arch
    }

    pub fn incoming_dir(&self) -> PathBuf {
        self.root.join("incoming")
    }

    pub fn staging_dir(&self) -> PathBuf {
        self.root.join("staging")
    }

    pub fn releases_dir(&self) -> PathBuf {
        self.root.join("releases")
    }

    pub fn current_link(&self) -> PathBuf {
        self.root.join("current")
    }

    pub fn previous_link(&self) -> PathBuf {
        self.root.join("previous")
    }

    pub fn ensure_layout(&self) -> Result<(), LifecycleError> {
        std::fs::create_dir_all(self.incoming_dir())?;
        std::fs::create_dir_all(self.staging_dir())?;
        std::fs::create_dir_all(self.releases_dir())?;
        Ok(())
    }

    pub fn current_version(&self) -> Result<Option<String>, LifecycleError> {
        self.link_version(&self.current_link())
    }

    pub fn previous_version(&self) -> Result<Option<String>, LifecycleError> {
        self.link_version(&self.previous_link())
    }

    fn link_version(&self, link: &Path) -> Result<Option<String>, LifecycleError> {
        if !link.is_symlink() {
            return Ok(None);
        }
        let target = std::fs::read_link(link)?;
        Ok(target
            .file_name()
            .map(|name| name.to_string_lossy().into_owned()))
    }

    pub(crate) fn activate_existing(&self, release: &StagedRelease) -> Result<(), LifecycleError> {
        verify_release_dir(&release.root, &self.expected_arch)?;
        self.ensure_layout()?;

        let new_target = release.root.canonicalize()?;
        let current = self.current_link();

        if current.is_symlink() {
            let old_current = std::fs::read_link(&current)?;
            replace_link_atomic(&self.previous_link(), &old_current)?;
        }

        replace_link_atomic(&current, &new_target)?;
        Ok(())
    }

    pub(crate) fn install_staged(
        &self,
        staged: &StagedRelease,
    ) -> Result<StagedRelease, LifecycleError> {
        self.ensure_layout()?;

        let destination = self.releases_dir().join(&staged.manifest.version);
        if destination.exists() {
            return Err(LifecycleError::AlreadyInstalled(
                staged.manifest.version.clone(),
            ));
        }

        let temp = self.releases_dir().join(format!(
            ".{}.next.{}",
            staged.manifest.version,
            std::process::id()
        ));

        if temp.exists() {
            std::fs::remove_dir_all(&temp)?;
        }

        copy_tree(&staged.root, &temp)?;
        std::fs::rename(&temp, &destination)?;
        staged_from_root(destination)
    }

    pub(crate) fn discover_incoming(&self) -> Result<Option<ReleaseInfo>, LifecycleError> {
        let incoming = self.incoming_dir();
        if !incoming.is_dir() {
            return Ok(None);
        }

        let current = self
            .current_version()?
            .and_then(|version| Version::parse(&version).ok());

        let mut candidates = Vec::new();

        for entry in std::fs::read_dir(incoming)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }

            let root = entry.path();
            let Ok(manifest) = crate::verify::read_manifest(&root) else {
                continue;
            };

            if manifest.architecture != self.expected_arch {
                continue;
            }

            let Ok(version) = Version::parse(&manifest.version) else {
                continue;
            };

            if current
                .as_ref()
                .is_none_or(|installed| version > *installed)
            {
                candidates.push((version, manifest, root));
            }
        }

        candidates.sort_by(|a, b| b.0.cmp(&a.0));

        Ok(candidates.first().map(|(_, manifest, root)| ReleaseInfo {
            version: manifest.version.clone(),
            architecture: manifest.architecture.clone(),
            source: root.clone(),
        }))
    }

    pub(crate) fn verify_release(
        &self,
        staged: &StagedRelease,
    ) -> Result<VerificationReport, LifecycleError> {
        verify_release_dir(&staged.root, &self.expected_arch)
    }
}

pub(crate) fn copy_tree(source: &Path, destination: &Path) -> Result<(), LifecycleError> {
    std::fs::create_dir_all(destination)?;

    for entry in WalkDir::new(source).follow_links(false) {
        let entry = entry.map_err(|error| LifecycleError::Io(error.to_string()))?;
        let relative = entry
            .path()
            .strip_prefix(source)
            .map_err(|error| LifecycleError::Io(error.to_string()))?;

        if relative.as_os_str().is_empty() {
            continue;
        }

        if entry.file_type().is_symlink() {
            return Err(LifecycleError::Integrity(format!(
                "package symlink rejected: {}",
                relative.display()
            )));
        }

        let target = destination.join(relative);

        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &target)?;
            std::fs::set_permissions(&target, std::fs::metadata(entry.path())?.permissions())?;
        }
    }

    Ok(())
}

#[cfg(unix)]
fn create_link(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn create_link(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
}

fn replace_link_atomic(link: &Path, target: &Path) -> Result<(), LifecycleError> {
    let parent = link
        .parent()
        .ok_or_else(|| LifecycleError::Io("link has no parent".into()))?;
    std::fs::create_dir_all(parent)?;

    let temp = parent.join(format!(
        ".{}.next.{}",
        link.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));

    if temp.is_symlink() || temp.exists() {
        let _ = std::fs::remove_file(&temp);
    }

    create_link(target, &temp)?;

    #[cfg(unix)]
    {
        std::fs::rename(&temp, link)?;
    }

    #[cfg(windows)]
    {
        if link.is_symlink() || link.exists() {
            std::fs::remove_file(link)?;
        }
        std::fs::rename(&temp, link)?;
    }

    Ok(())
}
