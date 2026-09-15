#![forbid(unsafe_code)]

use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassCheckpoint {
    pub version: Option<String>,
    pub pass_endpoint: String,
    pub platform: Option<String>,
    pub referenced_checksums: Vec<String>,
}

pub fn parse_pass_checkpoint(path: &Path, text: &str) -> Option<PassCheckpoint> {
    if !eligible_filename(path) {
        return None;
    }

    let mut in_fence = false;
    let mut version = None;
    let mut platform = None;
    let mut pass_endpoint = None;
    let mut referenced_checksums = Vec::new();

    for raw_line in text.lines() {
        let line = raw_line.trim();

        if line.starts_with("```") || line.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || line.is_empty() {
            continue;
        }

        let Some((key, value)) = strict_key_value(line) else {
            continue;
        };

        match key {
            "AETHERAI_VERSION" => version = Some(value.to_string()),
            "AETHERAI_PLATFORM" => platform = Some(value.to_string()),
            _ => {}
        }

        if is_final_verify_key(key) && value == "PASS" {
            pass_endpoint = Some(format!("{key}=PASS"));
        }

        if key.contains("SHA256") && is_sha256(value) {
            referenced_checksums.push(value.to_ascii_lowercase());
        }
    }

    let version = version?;
    let pass_endpoint = pass_endpoint?;
    referenced_checksums.sort();
    referenced_checksums.dedup();

    Some(PassCheckpoint {
        version: Some(version),
        pass_endpoint,
        platform,
        referenced_checksums,
    })
}

pub fn checkpoint_pass_line(text: &str, endpoint: &str) -> Option<u32> {
    let mut in_fence = false;
    for (index, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.starts_with("```") || line.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence && line == endpoint {
            return u32::try_from(index + 1).ok();
        }
    }
    None
}

fn eligible_filename(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let upper = name.to_ascii_uppercase();

    if !upper.ends_with(".TXT") {
        return false;
    }

    upper.ends_with("-VERIFY.TXT") || upper.contains("-PACKAGE-VERIFY")
}

fn strict_key_value(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once('=')?;
    if key.is_empty() || value.is_empty() {
        return None;
    }
    if !key
        .bytes()
        .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return None;
    }
    if value.chars().any(char::is_whitespace) {
        return None;
    }
    Some((key, value))
}

fn is_final_verify_key(key: &str) -> bool {
    key.starts_with("AETHERAI_")
        && (key.ends_with("_VERIFY") || key.ends_with("_PACKAGE_VERIFY"))
        && key.contains("_V")
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
