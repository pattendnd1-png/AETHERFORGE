#![forbid(unsafe_code)]

use std::path::Path;

const DEFAULT_EXCLUDED_DIRS: &[&str] = &[
    "target",
    "node_modules",
    ".git",
    ".cache",
    "__pycache__",
    ".venv",
    "venv",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
];

const HIGH_RISK_COMPONENTS: &[&str] = &[
    ".ssh",
    ".gnupg",
    ".gpg",
    ".aws",
    ".azure",
    ".kube",
    ".password-store",
];

const HIGH_RISK_FILES: &[&str] = &[
    ".env",
    ".env.local",
    ".env.production",
    "id_rsa",
    "id_ed25519",
    "credentials",
    "credentials.json",
    "secrets.json",
    "secrets.toml",
];

pub(crate) fn should_exclude(path: &Path, root: &Path, policy: &crate::IndexPolicy) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let components = relative
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();

    if components.iter().any(|component| {
        DEFAULT_EXCLUDED_DIRS
            .iter()
            .any(|excluded| component.eq_ignore_ascii_case(excluded))
    }) {
        return true;
    }

    if !policy.allow_high_risk_names {
        if components.iter().any(|component| {
            HIGH_RISK_COMPONENTS
                .iter()
                .any(|excluded| component.eq_ignore_ascii_case(excluded))
        }) {
            return true;
        }

        if let Some(file_name) = path.file_name().and_then(|value| value.to_str()) {
            if HIGH_RISK_FILES
                .iter()
                .any(|excluded| file_name.eq_ignore_ascii_case(excluded))
            {
                return true;
            }
        }
    }

    policy.user_ignores.iter().any(|rule| {
        let trimmed = rule.trim_matches(&['/', '\\'][..]);
        !trimmed.is_empty()
            && components
                .iter()
                .any(|component| component.eq_ignore_ascii_case(trimmed))
    })
}
