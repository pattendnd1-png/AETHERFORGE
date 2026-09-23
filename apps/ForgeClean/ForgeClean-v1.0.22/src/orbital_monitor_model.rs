use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NetworkTotals {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

pub fn parse_proc_net_dev(text: &str) -> NetworkTotals {
    let mut totals = NetworkTotals::default();
    for line in text.lines() {
        let Some((name, counters)) = line.split_once(':') else {
            continue;
        };
        if name.trim() == "lo" {
            continue;
        }
        let values: Vec<&str> = counters.split_whitespace().collect();
        if values.len() < 9 {
            continue;
        }
        totals.rx_bytes = totals
            .rx_bytes
            .saturating_add(values[0].parse::<u64>().unwrap_or(0));
        totals.tx_bytes = totals
            .tx_bytes
            .saturating_add(values[8].parse::<u64>().unwrap_or(0));
    }
    totals
}

pub fn read_network_totals() -> NetworkTotals {
    fs::read_to_string("/proc/net/dev")
        .map(|text| parse_proc_net_dev(&text))
        .unwrap_or_default()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncProgress {
    pub phase: String,
    pub project: String,
    pub current_file: String,
    pub completed: usize,
    pub total: usize,
    pub active: bool,
}

pub fn parse_diagnostic(text: &str, file_is_live: bool) -> SyncProgress {
    let mut out = SyncProgress {
        active: file_is_live,
        ..SyncProgress::default()
    };
    for raw in text.lines() {
        let line = raw.trim();
        if let Some(v) = line.strip_prefix("PHASE=") {
            out.phase = v.trim().to_string();
            continue;
        }
        if let Some(v) = line.strip_prefix("SOURCE_IMPORT_START=") {
            out.project = v.trim().to_string();
            continue;
        }
        if let Some(v) = line.strip_prefix("ARTIFACT_UPLOAD_QUEUE=") {
            out.total = v.trim().parse().unwrap_or(out.total);
            continue;
        }
        if let Some(v) = line.strip_prefix("ARTIFACT_UPLOAD_PROGRESS=") {
            if let Some((counts, file)) = v.split_once(':') {
                if let Some((done, total)) = counts.split_once('/') {
                    out.completed = done.parse().unwrap_or(out.completed);
                    out.total = total.parse().unwrap_or(out.total);
                }
                out.current_file = file.trim().to_string();
            }
            continue;
        }
        if line == "AETHERFORGE_GITHUB_MIGRATION_SWEEP=PASS" {
            out.phase = "COMPLETE".to_string();
            out.active = false;
        }
    }
    out
}

fn tail_text(path: &Path, max_bytes: u64) -> std::io::Result<String> {
    let mut file = fs::File::open(path)?;
    let len = file.metadata()?.len();
    let start = len.saturating_sub(max_bytes);
    file.seek(SeekFrom::Start(start))?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    Ok(text)
}

pub fn diagnostic_paths(home: &Path) -> (PathBuf, PathBuf) {
    let d = home.join("Downloads");
    (
        d.join("AETHERFORGE-MIGRATION-SWEEP-DIAG.txt.tmp"),
        d.join("AETHERFORGE-MIGRATION-SWEEP-DIAG.txt"),
    )
}

pub fn read_sync_progress(home: &Path) -> SyncProgress {
    let (live, done) = diagnostic_paths(home);
    if live.is_file() {
        return tail_text(&live, 131_072)
            .map(|text| parse_diagnostic(&text, true))
            .unwrap_or_default();
    }
    tail_text(&done, 131_072)
        .map(|text| parse_diagnostic(&text, false))
        .unwrap_or_default()
}
