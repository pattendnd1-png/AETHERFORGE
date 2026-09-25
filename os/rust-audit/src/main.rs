use std::fs;
use std::path::Path;

const VERSION: &str = "10.2.96";

const APP_SCOPE_PATHS: &[&str] = &[
    "/usr/lib/aetherforge/aetherstream/",
    "/usr/bin/aether-browser",
    "/usr/bin/forgehx-launch",
    "/usr/bin/aetherterminal",
    "/usr/bin/aetherai",
    "/usr/bin/aethersong",
    "/usr/bin/opensanctuary",
    "/usr/bin/opendeck",
    "/usr/bin/control-center",
    "/usr/bin/behringer-control",
    "/usr/bin/stellar",
    "/usr/bin/darkstone",
    "/usr/bin/reforge-logitech",
    "/usr/lib/aetherforge/",
    "/opt/aetherforge/",
];

const OS_OWNED_PATHS: &[&str] = &[
    "/usr/bin/garuda-",
    "/usr/bin/aetherforge-",
    "/usr/sbin/garuda-",
    "/usr/sbin/aetherforge-",
    "/etc/systemd/system/aetherforge-",
    "/lib/systemd/system/aetherforge-",
    "/etc/systemd/system/garuda-",
    "/lib/systemd/system/garuda-",
    "/etc/profile.d/aetherforge-",
    "/etc/profile.d/garuda-",
    "/usr/share/aur/aurto/",
    "/etc/xdg/autostart/aetherforge-",
    "/usr/share/polkit-1/rules.d/aetherforge-",
];

const SCAN_SURFACES: &[&str] = &[
    "/usr/bin",
    "/usr/sbin",
    "/usr/lib",
    "/usr/libexec",
    "/etc/systemd/system",
    "/lib/systemd/system",
    "/etc/profile.d",
    "/usr/share/aur/aurto",
    "/etc/xdg/autostart",
    "/usr/share/polkit-1/rules.d",
];

#[derive(Debug, Clone)]
pub struct AuditRecord {
    pub path: String,
    pub classification: &'static str,
    pub is_blocker: bool,
    pub os_owned: bool,
    pub file_size: u64,
}

fn is_app_scope(path: &str) -> bool {
    APP_SCOPE_PATHS.iter().any(|p| path.starts_with(p))
}

fn is_os_owned(path: &str) -> bool {
    OS_OWNED_PATHS.iter().any(|p| path.starts_with(p))
}

fn classify_script_bytes(prefix: &[u8]) -> Option<&'static str> {
    if prefix.starts_with(b"#!") {
        let shebang = std::str::from_utf8(prefix).unwrap_or("");
        return Some(match shebang {
            s if s.contains("python3") => "PYTHON",
            s if s.contains("python") => "PYTHON",
            s if s.contains("bash") => "BASH",
            s if s.contains("sh ") || s.ends_with("/sh") => "SHELL",
            s if s.contains("perl") => "PERL",
            s if s.contains("ruby") => "RUBY",
            s if s.contains("node") => "NODE",
            _ => "UNKNOWN_SCRIPT",
        });
    }

    if prefix.starts_with(b"\x7fELF") {
        if has_rust_evidence(prefix) {
            return Some("RUST_BINARY");
        }

        return Some("ELF_BINARY");
    }

    None
}

fn has_rust_evidence(prefix: &[u8]) -> bool {
    const SIGS: &[&[u8]] = &[
        b"rust_begin_unwind",
        b"__rust_alloc",
        b"libstd-",
        b"libcore-",
    ];
    SIGS.iter()
        .any(|s| prefix.windows(s.len()).any(|w| w == *s))
}

fn scan_directory(dir: &str) -> Vec<AuditRecord> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .filter_map(|entry| {
            let path = entry.path();
            let Ok(metadata) = fs::metadata(&path) else {
                return None;
            };
            if !metadata.is_file() {
                return None;
            }
            let path_str = path.to_string_lossy().to_string();
            if is_app_scope(&path_str) {
                return None;
            }
            let Ok(content) = fs::read(&path) else {
                return None;
            };
            let prefix = &content[..content.len().min(512)];
            let classification = classify_script_bytes(prefix)?;
            let is_blocker = matches!(
                classification,
                "PYTHON" | "BASH" | "SHELL" | "PERL" | "RUBY" | "NODE" | "UNKNOWN_SCRIPT"
            );
            Some(AuditRecord {
                path: path_str.clone(),
                classification,
                is_blocker,
                os_owned: is_os_owned(&path_str),
                file_size: metadata.len(),
            })
        })
        .collect()
}

fn run_audit() -> Vec<AuditRecord> {
    SCAN_SURFACES
        .iter()
        .flat_map(|surface| {
            if !Path::new(surface).exists() {
                return Vec::new();
            }
            scan_directory(surface)
        })
        .collect()
}

fn generate_report(records: &[AuditRecord]) -> String {
    use std::collections::HashMap;
    let mut stats = HashMap::new();
    let mut blocker_count = 0;
    for r in records {
        *stats.entry(r.classification).or_insert(0) += 1;
        if r.is_blocker {
            blocker_count += 1;
        }
    }
    let mut report = format!(
        "=== AETHERFORGE OS RUST AUDIT v{} ===\nTimestamp: {:?}\nItems: {}\nBlockers: {}\n\n",
        VERSION,
        std::time::SystemTime::now(),
        records.len(),
        blocker_count
    );
    report.push_str("--- BREAKDOWN ---\n");
    for (k, v) in &stats {
        report.push_str(&format!("  {}: {}\n", k, v));
    }
    report.push_str("\n--- BLOCKERS ---\n");
    for r in records.iter().filter(|x| x.is_blocker) {
        report.push_str(&format!("  [{}] {}\n", r.classification, r.path));
    }
    report
}

fn main() {
    eprintln!("🔍 AETHERFORGE_OS_RUST_AUDIT_VERSION={}", VERSION);
    let records = run_audit();
    let report = generate_report(&records);
    print!("{}", report);

    if let Some(home) = dirs::home_dir() {
        let path = home
            .join("Downloads")
            .join(format!("aetherforge-audit-{}.txt", VERSION));
        let _ = fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new("")));
        let _ = fs::write(&path, &report);
        eprintln!("📄 Report saved: {}", path.display());
    }

    let blocker_count = records.iter().filter(|r| r.is_blocker).count();
    std::process::exit(if blocker_count > 0 { 1 } else { 0 });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_is_app_scope() {
        assert!(is_app_scope("/usr/lib/aetherforge/aetherstream/test"));
    }
    #[test]
    fn test_is_os_owned() {
        assert!(is_os_owned("/usr/bin/garuda-update"));
        assert!(!is_os_owned("/usr/bin/random"));
    }
    #[test]
    fn test_classify() {
        assert_eq!(
            classify_script_bytes(b"#!/usr/bin/python3\n"),
            Some("PYTHON")
        );
    }
}
