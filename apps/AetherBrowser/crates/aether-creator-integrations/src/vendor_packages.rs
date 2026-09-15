//! Typed identities for user-owned vendor installers used as AetherBrowser compatibility references.
//!
//! These packages are data/reference inputs only. AetherBrowser never executes the vendor installers
//! or their platform-specific binaries; runtime execution stays on native Linux/Rust/browser bridges.

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VendorPackageFormat {
    MacXarPkg,
    WindowsNsisExe,
    WindowsMsi,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VendorExecutionPolicy {
    MetadataReferenceOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserIntegrationTarget {
    ObsWorkspace,
    StreamElementsStudio,
    StreamlabsStudio,
    StreamElementsGroundControl,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VendorPackageDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub canonical_basename: &'static str,
    pub expected_version: Option<&'static str>,
    pub expected_sha256: &'static str,
    pub format: VendorPackageFormat,
    pub execution_policy: VendorExecutionPolicy,
    pub browser_target: BrowserIntegrationTarget,
}

pub const OBS_STUDIO_3222: VendorPackageDescriptor = VendorPackageDescriptor {
    id: "obs-studio-32.2.2",
    display_name: "OBS Studio 32.2.2",
    canonical_basename: "OBS-Studio-32.2.2-Windows-x64-Installer.exe",
    expected_version: Some("32.2.2"),
    expected_sha256: "c3a0b880adbe64dc4bcb68f93016916ab5b55ae43fd227115287bf80257d92dc",
    format: VendorPackageFormat::WindowsNsisExe,
    execution_policy: VendorExecutionPolicy::MetadataReferenceOnly,
    browser_target: BrowserIntegrationTarget::ObsWorkspace,
};

pub const OBS_STREAMELEMENTS_LATEST: VendorPackageDescriptor = VendorPackageDescriptor {
    id: "obs-streamelements-latest",
    display_name: "OBS + StreamElements",
    canonical_basename: "obs-streamelements-setup-latest.exe",
    expected_version: None,
    expected_sha256: "79be650ee593f79727e94c098a7d9a60fbe16ef73db2c7c54472aa7753a29ad3",
    format: VendorPackageFormat::WindowsNsisExe,
    execution_policy: VendorExecutionPolicy::MetadataReferenceOnly,
    browser_target: BrowserIntegrationTarget::StreamElementsStudio,
};

pub const STREAMLABS_DESKTOP_1219: VendorPackageDescriptor = VendorPackageDescriptor {
    id: "streamlabs-desktop-1.21.9",
    display_name: "Streamlabs Desktop 1.21.9",
    canonical_basename: "Streamlabs+Desktop+Setup+1.21.9-5qLbAShV5RGxPpP.exe",
    expected_version: Some("1.21.9"),
    expected_sha256: "57e0c280bc4a85e66411a1ed0f874d56b8f2a33590d42146c6d07bab96ca9ceb",
    format: VendorPackageFormat::WindowsNsisExe,
    execution_policy: VendorExecutionPolicy::MetadataReferenceOnly,
    browser_target: BrowserIntegrationTarget::StreamlabsStudio,
};

pub const GROUND_CONTROL_2120: VendorPackageDescriptor = VendorPackageDescriptor {
    id: "streamelements-ground-control-2.1.20",
    display_name: "Ground Control 2.1.20",
    canonical_basename: "Ground Control_x64_en-US.msi",
    expected_version: Some("2.1.20"),
    expected_sha256: "b374dbf545d00626fe134b4e23e6e542eba64ebb8b9f66f4b056c11233da0ed5",
    format: VendorPackageFormat::WindowsMsi,
    execution_policy: VendorExecutionPolicy::MetadataReferenceOnly,
    browser_target: BrowserIntegrationTarget::StreamElementsGroundControl,
};

pub const AUTHORITATIVE_VENDOR_PACKAGES: &[VendorPackageDescriptor] = &[
    OBS_STUDIO_3222,
    OBS_STREAMELEMENTS_LATEST,
    STREAMLABS_DESKTOP_1219,
    GROUND_CONTROL_2120,
];

pub const GROUND_CONTROL_ALERT_ACTIONS: &[&str] = &[
    "Mute Alerts",
    "UnMute Alerts",
    "Pause Alerts",
    "Resume Alerts",
    "Skip Alert",
    "Toggle Alerts",
];

#[must_use]
pub const fn authoritative_vendor_packages() -> &'static [VendorPackageDescriptor] {
    AUTHORITATIVE_VENDOR_PACKAGES
}

#[must_use]
pub const fn ground_control_alert_actions() -> &'static [&'static str] {
    GROUND_CONTROL_ALERT_ACTIONS
}

#[must_use]
pub fn default_downloads_root(home: &Path) -> PathBuf {
    home.join("Downloads")
}

/// Discover the exact canonical package first, then browser/file-manager duplicate aliases like `(1)`.
#[must_use]
pub fn discover_authoritative_package(
    home: &Path,
    descriptor: &VendorPackageDescriptor,
) -> Option<PathBuf> {
    let downloads = default_downloads_root(home);
    let preferred = downloads.join(descriptor.canonical_basename);
    if preferred.is_file() {
        return Some(preferred);
    }
    let (stem, extension) = split_basename(descriptor.canonical_basename);
    let mut candidates = fs::read_dir(downloads)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                return false;
            };
            duplicate_alias_matches(name, stem, extension)
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|path| path.metadata().and_then(|meta| meta.modified()).ok());
    candidates.pop()
}

#[must_use]
pub fn package_reference_status(home: &Path, descriptor: &VendorPackageDescriptor) -> &'static str {
    if discover_authoritative_package(home, descriptor).is_some() {
        "FOUND_REFERENCE_ONLY"
    } else {
        "MISSING"
    }
}

fn split_basename(basename: &str) -> (&str, &str) {
    match basename.rsplit_once('.') {
        Some((stem, extension)) => (stem, extension),
        None => (basename, ""),
    }
}

fn duplicate_alias_matches(name: &str, stem: &str, extension: &str) -> bool {
    let suffix = if extension.is_empty() {
        String::new()
    } else {
        format!(".{extension}")
    };
    if name == format!("{stem}{suffix}") {
        return true;
    }
    let Some(middle) = name
        .strip_prefix(stem)
        .and_then(|value| value.strip_suffix(&suffix))
    else {
        return false;
    };
    let Some(number) = middle
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
    else {
        return false;
    };
    !number.is_empty() && number.chars().all(|character| character.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_alias_is_strict() {
        assert!(duplicate_alias_matches(
            "OBS-Studio-32.2.2-Windows-x64-Installer(1).exe",
            "OBS-Studio-32.2.2-Windows-x64-Installer",
            "exe"
        ));
        assert!(!duplicate_alias_matches(
            "OBS-Studio-32.2.2-Windows-x64-Installer-copy.exe",
            "OBS-Studio-32.2.2-Windows-x64-Installer",
            "exe"
        ));
    }
}
