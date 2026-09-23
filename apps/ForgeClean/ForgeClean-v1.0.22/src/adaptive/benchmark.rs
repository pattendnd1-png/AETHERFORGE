use crate::adaptive::audit::AuditEngine;
use crate::adaptive::health::{HealthSample, SystemHealth, unix_ms};
use crate::adaptive::workload_guard::{ProtectedReason, ProtectionState};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BenchmarkKind {
    Quick,
    Full,
    Cpu,
    Memory,
    Storage,
    Network,
    Audio,
    Beacn,
    Mixed,
}

impl BenchmarkKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "quick" => Some(Self::Quick),
            "full" => Some(Self::Full),
            "cpu" => Some(Self::Cpu),
            "memory" => Some(Self::Memory),
            "storage" => Some(Self::Storage),
            "network" => Some(Self::Network),
            "audio" => Some(Self::Audio),
            "beacn" => Some(Self::Beacn),
            "mixed" | "mixed-load" => Some(Self::Mixed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkMetric {
    pub name: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkReport {
    pub schema_version: u32,
    pub started_at_unix_ms: u64,
    pub ended_at_unix_ms: u64,
    pub kind: BenchmarkKind,
    pub metrics: Vec<BenchmarkMetric>,
    pub health: SystemHealth,
}

#[derive(Debug, Clone)]
pub struct BenchmarkRequest {
    pub kind: BenchmarkKind,
    pub allow_network_saturation: bool,
    pub max_seconds: u64,
}

impl BenchmarkRequest {
    pub fn quick() -> Self {
        Self {
            kind: BenchmarkKind::Quick,
            allow_network_saturation: false,
            max_seconds: 5,
        }
    }

    pub fn for_kind(kind: BenchmarkKind) -> Self {
        Self {
            kind,
            allow_network_saturation: false,
            max_seconds: 30,
        }
    }
}

#[derive(Debug)]
pub enum BenchmarkError {
    ProtectedWorkload(ProtectedReason),
    Io(io::Error),
    UnsafeTarget(String),
}

impl fmt::Display for BenchmarkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProtectedWorkload(r) => write!(f, "protected workload active: {r:?}"),
            Self::Io(e) => write!(f, "{e}"),
            Self::UnsafeTarget(p) => write!(f, "unsafe benchmark target: {p}"),
        }
    }
}

impl std::error::Error for BenchmarkError {}

impl From<io::Error> for BenchmarkError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

#[derive(Debug, Clone)]
pub struct BenchmarkEngine {
    home: PathBuf,
    audit: AuditEngine,
}

impl BenchmarkEngine {
    pub fn new(home: &Path) -> Self {
        Self {
            home: home.to_path_buf(),
            audit: AuditEngine,
        }
    }

    pub fn run<G: ProtectionState>(
        &self,
        request: &BenchmarkRequest,
        guard: &G,
    ) -> Result<BenchmarkReport, BenchmarkError> {
        if let Some(reason) = guard.protected_reason() {
            return Err(BenchmarkError::ProtectedWorkload(reason));
        }

        let started = unix_ms();
        let mut metrics = Vec::new();
        match request.kind {
            BenchmarkKind::Quick => {
                metrics.extend(cpu_benchmark());
                metrics.extend(memory_benchmark());
                metrics.extend(self.storage_benchmark()?);
            }
            BenchmarkKind::Full | BenchmarkKind::Mixed => {
                metrics.extend(cpu_benchmark());
                metrics.extend(memory_benchmark());
                metrics.extend(self.storage_benchmark()?);
                metrics.extend(network_observation());
                metrics.extend(audio_observation());
            }
            BenchmarkKind::Cpu => metrics.extend(cpu_benchmark()),
            BenchmarkKind::Memory => metrics.extend(memory_benchmark()),
            BenchmarkKind::Storage => metrics.extend(self.storage_benchmark()?),
            BenchmarkKind::Network => metrics.extend(network_observation()),
            BenchmarkKind::Audio => metrics.extend(audio_observation()),
            BenchmarkKind::Beacn => {}
        }

        let health = self.audit.quick()?;
        if matches!(
            request.kind,
            BenchmarkKind::Beacn | BenchmarkKind::Full | BenchmarkKind::Mixed
        ) {
            let connected = health.samples.iter().find_map(|s| {
                if let HealthSample::BeacnConnected(v) = s {
                    Some(*v)
                } else {
                    None
                }
            });
            metrics.push(BenchmarkMetric {
                name: "beacn_connected".into(),
                value: if connected.unwrap_or(false) { 1.0 } else { 0.0 },
                unit: "bool".into(),
            });
        }

        Ok(BenchmarkReport {
            schema_version: 1,
            started_at_unix_ms: started,
            ended_at_unix_ms: unix_ms(),
            kind: request.kind,
            metrics,
            health,
        })
    }

    fn storage_benchmark(&self) -> Result<Vec<BenchmarkMetric>, BenchmarkError> {
        let root = self.home.join("Downloads/ForgeClean/Temporary");
        fs::create_dir_all(&root)?;
        let root = fs::canonicalize(&root)?;
        let path = root.join(format!("adaptive-bench-{}.bin", std::process::id()));
        if !path.starts_with(&root) || path.starts_with("/dev") || path.starts_with("/sys") {
            return Err(BenchmarkError::UnsafeTarget(path.display().to_string()));
        }

        let block = vec![0xA5u8; 1024 * 1024];
        let blocks = 16usize;
        let start = Instant::now();
        {
            let mut f = File::create(&path)?;
            for _ in 0..blocks {
                f.write_all(&block)?;
            }
            f.sync_all()?;
        }
        let write_secs = start.elapsed().as_secs_f64().max(0.000_001);

        let start = Instant::now();
        let mut f = File::open(&path)?;
        let mut buf = vec![0u8; 1024 * 1024];
        let mut bytes = 0u64;
        loop {
            let n = f.read(&mut buf)?;
            if n == 0 {
                break;
            }
            bytes = bytes.saturating_add(n as u64);
        }
        let read_secs = start.elapsed().as_secs_f64().max(0.000_001);
        fs::remove_file(&path)?;

        let mib = bytes as f64 / (1024.0 * 1024.0);
        Ok(vec![
            BenchmarkMetric {
                name: "storage_write_mib_s".into(),
                value: blocks as f64 / write_secs,
                unit: "MiB/s".into(),
            },
            BenchmarkMetric {
                name: "storage_read_mib_s".into(),
                value: mib / read_secs,
                unit: "MiB/s".into(),
            },
        ])
    }
}

fn cpu_benchmark() -> Vec<BenchmarkMetric> {
    let start = Instant::now();
    let target = Duration::from_millis(300);
    let mut iterations = 0u64;
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    while start.elapsed() < target {
        for _ in 0..4096 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            iterations = iterations.wrapping_add(1);
        }
    }
    std::hint::black_box(x);
    let secs = start.elapsed().as_secs_f64().max(0.000_001);
    vec![BenchmarkMetric {
        name: "cpu_integer_ops_s".into(),
        value: iterations as f64 / secs,
        unit: "ops/s".into(),
    }]
}

fn memory_benchmark() -> Vec<BenchmarkMetric> {
    let size = 32 * 1024 * 1024usize;
    let src = vec![0x5Au8; size];
    let mut dst = vec![0u8; size];
    let passes = 4usize;
    let start = Instant::now();
    for _ in 0..passes {
        dst.copy_from_slice(&src);
        std::hint::black_box(&dst);
    }
    let secs = start.elapsed().as_secs_f64().max(0.000_001);
    let mib = (size * passes) as f64 / (1024.0 * 1024.0);
    vec![BenchmarkMetric {
        name: "memory_copy_mib_s".into(),
        value: mib / secs,
        unit: "MiB/s".into(),
    }]
}

fn network_observation() -> Vec<BenchmarkMetric> {
    let output = Command::new("ping")
        .args(["-c", "4", "-W", "1", "1.1.1.1"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let Some(line) = text.lines().find(|line| line.contains("min/avg/max")) else {
        return Vec::new();
    };
    let Some(values) = line.split('=').nth(1) else {
        return Vec::new();
    };
    let parts: Vec<&str> = values.trim().split('/').collect();
    let Some(avg) = parts.get(1).and_then(|v| v.parse::<f64>().ok()) else {
        return Vec::new();
    };
    vec![BenchmarkMetric {
        name: "network_rtt_avg_ms".into(),
        value: avg,
        unit: "ms".into(),
    }]
}

fn audio_observation() -> Vec<BenchmarkMetric> {
    let output = Command::new("journalctl")
        .args([
            "--user",
            "-u",
            "pipewire.service",
            "--since",
            "-2 minutes",
            "--no-pager",
        ])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
    let count = text.lines().filter(|line| line.contains("xrun")).count();
    vec![BenchmarkMetric {
        name: "pipewire_xruns".into(),
        value: count as f64,
        unit: "count".into(),
    }]
}
