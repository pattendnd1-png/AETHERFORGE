use crate::{
    FileDigest, LifecycleError, ReleaseManifest, StagedRelease, VerificationReport,
    health::run_health_diagnostic,
};
use semver::Version;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path};

pub fn sha256_file(path: &Path) -> Result<String, LifecycleError> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];

    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }

    Ok(hex::encode(hasher.finalize()))
}

pub(crate) fn safe_relative(path: &str) -> Result<&Path, LifecycleError> {
    let path = Path::new(path);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(LifecycleError::InvalidManifest(format!(
            "unsafe package path: {}",
            path.display()
        )));
    }
    Ok(path)
}

pub(crate) fn read_manifest(root: &Path) -> Result<ReleaseManifest, LifecycleError> {
    let manifest_path = root.join("release.json");
    let sidecar_path = root.join("release.json.sha256");

    let expected = std::fs::read_to_string(&sidecar_path)
        .map_err(|error| LifecycleError::InvalidManifest(error.to_string()))?
        .trim()
        .to_string();
    let actual = sha256_file(&manifest_path)?;

    if expected != actual {
        return Err(LifecycleError::Integrity(
            "release.json SHA-256 sidecar mismatch".into(),
        ));
    }

    let manifest: ReleaseManifest = serde_json::from_slice(&std::fs::read(&manifest_path)?)
        .map_err(|error| LifecycleError::InvalidManifest(error.to_string()))?;

    if manifest.schema != 1 {
        return Err(LifecycleError::InvalidManifest(format!(
            "unsupported manifest schema {}",
            manifest.schema
        )));
    }

    Version::parse(&manifest.version)
        .map_err(|error| LifecycleError::InvalidManifest(error.to_string()))?;
    safe_relative(&manifest.binary)?;

    if manifest.files.is_empty() {
        return Err(LifecycleError::InvalidManifest(
            "manifest contains no hashed files".into(),
        ));
    }

    Ok(manifest)
}

fn verify_file(root: &Path, digest: &FileDigest) -> Result<(), LifecycleError> {
    let relative = safe_relative(&digest.path)?;
    let path = root.join(relative);
    let metadata = std::fs::symlink_metadata(&path)?;

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LifecycleError::Integrity(format!(
            "hashed package entry is not a regular file: {}",
            digest.path
        )));
    }

    let actual = sha256_file(&path)?;
    if actual != digest.sha256 {
        return Err(LifecycleError::Integrity(format!(
            "SHA-256 mismatch: {}",
            digest.path
        )));
    }

    Ok(())
}

pub(crate) fn verify_release_dir(
    root: &Path,
    expected_arch: &str,
) -> Result<VerificationReport, LifecycleError> {
    let manifest = read_manifest(root)?;

    if manifest.architecture != expected_arch {
        return Err(LifecycleError::Architecture {
            expected: expected_arch.into(),
            actual: manifest.architecture,
        });
    }

    if manifest.browser_webview || manifest.openai_api_required || manifest.paid_service_required {
        return Err(LifecycleError::Contract(
            "release violates native zero-cost runtime contract".into(),
        ));
    }

    let binary = root.join(safe_relative(&manifest.binary)?);
    let metadata = std::fs::symlink_metadata(&binary)?;

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LifecycleError::Integrity(
            "desktop binary missing or not a regular file".into(),
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(LifecycleError::Integrity(
                "desktop binary is not executable".into(),
            ));
        }
    }

    for digest in &manifest.files {
        verify_file(root, digest)?;
    }

    run_health_diagnostic(root, &manifest.binary)?;

    Ok(VerificationReport {
        version: manifest.version,
        architecture: expected_arch.into(),
        checks: vec![
            "manifest".into(),
            "manifest-sha256".into(),
            "version".into(),
            "architecture".into(),
            "package-structure".into(),
            "binary".into(),
            "file-sha256".into(),
            "native-zero-cost-contract".into(),
            "health-diagnostic".into(),
        ],
    })
}

pub(crate) fn staged_from_root(root: std::path::PathBuf) -> Result<StagedRelease, LifecycleError> {
    let manifest = read_manifest(&root)?;
    Ok(StagedRelease { root, manifest })
}
