#![forbid(unsafe_code)]
//! Native Linux system and streaming telemetry for Aether Browser.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const PROC_STAT: &str = "/proc/stat";
const PROC_MEMINFO: &str = "/proc/meminfo";
const PROC_NET_DEV: &str = "/proc/net/dev";
const DRM_ROOT: &str = "/sys/class/drm";
const THERMAL_ROOT: &str = "/sys/class/thermal";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CpuTimes {
    pub total: u64,
    pub idle: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MemoryCounters {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_free_bytes: u64,
}

impl MemoryCounters {
    #[must_use]
    pub fn used_bytes(self) -> u64 {
        self.total_bytes.saturating_sub(self.available_bytes)
    }

    #[must_use]
    pub fn swap_used_bytes(self) -> u64 {
        self.swap_total_bytes.saturating_sub(self.swap_free_bytes)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NetworkCounters {
    pub received_bytes: u64,
    pub transmitted_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StreamTelemetrySnapshot {
    pub active: bool,
    pub recording: bool,
    pub target_fps_milli: Option<u32>,
    pub render_fps_milli: Option<u32>,
    pub render_skipped_frames: u64,
    pub output_skipped_frames: u64,
    pub network_dropped_frames: u64,
    pub total_frames: u64,
}

impl StreamTelemetrySnapshot {
    #[must_use]
    pub fn total_lost_frames(self) -> u64 {
        self.render_skipped_frames
            .saturating_add(self.output_skipped_frames)
            .saturating_add(self.network_dropped_frames)
    }

    #[must_use]
    pub fn render_loss_basis_points(self) -> Option<u32> {
        loss_basis_points(self.render_skipped_frames, self.total_frames)
    }

    #[must_use]
    pub fn output_loss_basis_points(self) -> Option<u32> {
        loss_basis_points(self.output_skipped_frames, self.total_frames)
    }

    #[must_use]
    pub fn network_loss_basis_points(self) -> Option<u32> {
        loss_basis_points(self.network_dropped_frames, self.total_frames)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SystemTelemetrySnapshot {
    pub cpu_percent: Option<u8>,
    pub ram_used_bytes: Option<u64>,
    pub ram_total_bytes: Option<u64>,
    pub swap_used_bytes: Option<u64>,
    pub swap_total_bytes: Option<u64>,
    pub network_rx_bytes_per_sec: Option<u64>,
    pub network_tx_bytes_per_sec: Option<u64>,
    pub gpu_percent: Option<u8>,
    pub vram_used_bytes: Option<u64>,
    pub vram_total_bytes: Option<u64>,
    pub gpu_temp_millic: Option<i64>,
    pub cpu_temp_millic: Option<i64>,
    pub browser_fps_milli: Option<u32>,
    pub stream: StreamTelemetrySnapshot,
}

#[derive(Debug)]
pub struct FrameRateTracker {
    window_start: Instant,
    frames: u32,
    minimum_window: Duration,
}

impl FrameRateTracker {
    #[must_use]
    pub fn new(now: Instant) -> Self {
        Self {
            window_start: now,
            frames: 0,
            minimum_window: Duration::from_millis(500),
        }
    }

    #[must_use]
    pub fn record_frame(&mut self, now: Instant) -> Option<u32> {
        self.frames = self.frames.saturating_add(1);
        let elapsed = now.saturating_duration_since(self.window_start);
        if elapsed < self.minimum_window {
            return None;
        }
        let nanos = elapsed.as_nanos();
        let numerator = u128::from(self.frames) * 1_000_000_000_000_u128;
        let fps_milli = numerator
            .checked_div(nanos)
            .unwrap_or(0)
            .min(u128::from(u32::MAX)) as u32;
        self.frames = 0;
        self.window_start = now;
        Some(fps_milli)
    }
}

#[derive(Debug, Default)]
pub struct LinuxTelemetrySampler {
    previous_cpu: Option<CpuTimes>,
    previous_network: Option<(Instant, NetworkCounters)>,
    gpu_device_dirs: Vec<PathBuf>,
    cpu_temp_paths: Vec<PathBuf>,
    paths_discovered: bool,
}

impl LinuxTelemetrySampler {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn sample(&mut self) -> SystemTelemetrySnapshot {
        self.sample_at(Instant::now())
    }

    #[must_use]
    pub fn sample_at(&mut self, now: Instant) -> SystemTelemetrySnapshot {
        if !self.paths_discovered {
            self.gpu_device_dirs = drm_device_dirs(Path::new(DRM_ROOT));
            self.cpu_temp_paths = cpu_temperature_paths(Path::new(THERMAL_ROOT));
            self.paths_discovered = true;
        }

        let current_cpu = fs::read_to_string(PROC_STAT)
            .ok()
            .and_then(|text| parse_proc_stat(&text));
        let cpu_percent = match (self.previous_cpu, current_cpu) {
            (Some(previous), Some(current)) => cpu_percent_between(previous, current),
            _ => None,
        };
        self.previous_cpu = current_cpu;

        let memory = fs::read_to_string(PROC_MEMINFO)
            .ok()
            .and_then(|text| parse_meminfo(&text));

        let current_network = fs::read_to_string(PROC_NET_DEV)
            .ok()
            .and_then(|text| parse_net_dev(&text));
        let (network_rx_bytes_per_sec, network_tx_bytes_per_sec) =
            match (self.previous_network, current_network) {
                (Some((previous_at, previous)), Some(current)) => {
                    let elapsed = now.saturating_duration_since(previous_at);
                    (
                        bytes_per_second(previous.received_bytes, current.received_bytes, elapsed),
                        bytes_per_second(
                            previous.transmitted_bytes,
                            current.transmitted_bytes,
                            elapsed,
                        ),
                    )
                }
                _ => (None, None),
            };
        self.previous_network = current_network.map(|counters| (now, counters));

        let gpu = read_gpu_snapshot_devices(&self.gpu_device_dirs);
        let cpu_temp_millic = read_cpu_temperature_paths(&self.cpu_temp_paths);

        SystemTelemetrySnapshot {
            cpu_percent,
            ram_used_bytes: memory.map(MemoryCounters::used_bytes),
            ram_total_bytes: memory.map(|value| value.total_bytes),
            swap_used_bytes: memory.map(MemoryCounters::swap_used_bytes),
            swap_total_bytes: memory.map(|value| value.swap_total_bytes),
            network_rx_bytes_per_sec,
            network_tx_bytes_per_sec,
            gpu_percent: gpu.busy_percent,
            vram_used_bytes: gpu.vram_used_bytes,
            vram_total_bytes: gpu.vram_total_bytes,
            gpu_temp_millic: gpu.temp_millic,
            cpu_temp_millic,
            browser_fps_milli: None,
            stream: StreamTelemetrySnapshot::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct GpuSnapshot {
    busy_percent: Option<u8>,
    vram_used_bytes: Option<u64>,
    vram_total_bytes: Option<u64>,
    temp_millic: Option<i64>,
}

#[must_use]
pub fn parse_proc_stat(text: &str) -> Option<CpuTimes> {
    let line = text.lines().find(|line| line.starts_with("cpu "))?;
    let mut values = line
        .split_whitespace()
        .skip(1)
        .take(8)
        .map(str::parse::<u64>);
    let user = values.next()?.ok()?;
    let nice = values.next()?.ok()?;
    let system = values.next()?.ok()?;
    let idle = values.next()?.ok()?;
    let iowait = values.next().and_then(Result::ok).unwrap_or(0);
    let irq = values.next().and_then(Result::ok).unwrap_or(0);
    let softirq = values.next().and_then(Result::ok).unwrap_or(0);
    let steal = values.next().and_then(Result::ok).unwrap_or(0);
    Some(CpuTimes {
        total: user
            .saturating_add(nice)
            .saturating_add(system)
            .saturating_add(idle)
            .saturating_add(iowait)
            .saturating_add(irq)
            .saturating_add(softirq)
            .saturating_add(steal),
        idle: idle.saturating_add(iowait),
    })
}

#[must_use]
pub fn cpu_percent_between(previous: CpuTimes, current: CpuTimes) -> Option<u8> {
    let total = current.total.saturating_sub(previous.total);
    if total == 0 {
        return None;
    }
    let idle = current.idle.saturating_sub(previous.idle).min(total);
    let busy = total.saturating_sub(idle);
    Some(((busy.saturating_mul(100) / total).min(100)) as u8)
}

#[must_use]
pub fn parse_meminfo(text: &str) -> Option<MemoryCounters> {
    fn value(text: &str, name: &str) -> Option<u64> {
        text.lines().find_map(|line| {
            let (key, rest) = line.split_once(':')?;
            if key != name {
                return None;
            }
            rest.split_whitespace()
                .next()?
                .parse::<u64>()
                .ok()
                .map(|kib| kib.saturating_mul(1024))
        })
    }

    Some(MemoryCounters {
        total_bytes: value(text, "MemTotal")?,
        available_bytes: value(text, "MemAvailable")?,
        swap_total_bytes: value(text, "SwapTotal").unwrap_or(0),
        swap_free_bytes: value(text, "SwapFree").unwrap_or(0),
    })
}

#[must_use]
pub fn parse_net_dev(text: &str) -> Option<NetworkCounters> {
    let mut counters = NetworkCounters::default();
    let mut found = false;
    for line in text.lines().skip(2) {
        let Some((interface, values)) = line.split_once(':') else {
            continue;
        };
        if interface.trim() == "lo" {
            continue;
        }
        let fields = values.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 16 {
            continue;
        }
        let Ok(received) = fields[0].parse::<u64>() else {
            continue;
        };
        let Ok(transmitted) = fields[8].parse::<u64>() else {
            continue;
        };
        counters.received_bytes = counters.received_bytes.saturating_add(received);
        counters.transmitted_bytes = counters.transmitted_bytes.saturating_add(transmitted);
        found = true;
    }
    found.then_some(counters)
}

#[must_use]
pub fn loss_basis_points(lost_frames: u64, total_frames: u64) -> Option<u32> {
    if total_frames == 0 {
        return None;
    }
    Some(
        ((u128::from(lost_frames) * 10_000_u128) / u128::from(total_frames))
            .min(u128::from(u32::MAX)) as u32,
    )
}

fn bytes_per_second(previous: u64, current: u64, elapsed: Duration) -> Option<u64> {
    let nanos = elapsed.as_nanos();
    if nanos == 0 {
        return None;
    }
    Some(
        ((u128::from(current.saturating_sub(previous)) * 1_000_000_000_u128) / nanos)
            .min(u128::from(u64::MAX)) as u64,
    )
}

fn read_number(path: &Path) -> Option<u64> {
    fs::read_to_string(path).ok()?.trim().parse::<u64>().ok()
}

fn read_signed_number(path: &Path) -> Option<i64> {
    fs::read_to_string(path).ok()?.trim().parse::<i64>().ok()
}

fn drm_device_dirs(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            let text = name.to_string_lossy();
            text.starts_with("card") && !text.contains('-')
        })
        .map(|entry| entry.path().join("device"))
        .collect()
}

fn read_gpu_snapshot_devices(devices: &[PathBuf]) -> GpuSnapshot {
    let mut snapshot = GpuSnapshot::default();
    for device in devices {
        if snapshot.busy_percent.is_none() {
            snapshot.busy_percent =
                read_number(&device.join("gpu_busy_percent")).map(|value| value.min(100) as u8);
        }
        if snapshot.vram_total_bytes.is_none() {
            snapshot.vram_total_bytes = read_number(&device.join("mem_info_vram_total"));
        }
        if snapshot.vram_used_bytes.is_none() {
            snapshot.vram_used_bytes = read_number(&device.join("mem_info_vram_used"));
        }
        if snapshot.temp_millic.is_none() {
            snapshot.temp_millic = read_hwmon_temperature(&device.join("hwmon"));
        }
    }
    snapshot
}

fn read_hwmon_temperature(root: &Path) -> Option<i64> {
    let entries = fs::read_dir(root).ok()?;
    entries.filter_map(Result::ok).find_map(|entry| {
        let value = read_signed_number(&entry.path().join("temp1_input"))?;
        plausible_millic(value)
    })
}

fn cpu_temperature_paths(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("temp"))
        .filter(|path| path.is_file())
        .collect()
}

fn read_cpu_temperature_paths(paths: &[PathBuf]) -> Option<i64> {
    paths
        .iter()
        .filter_map(|path| read_signed_number(path))
        .filter_map(plausible_millic)
        .max()
}

fn plausible_millic(value: i64) -> Option<i64> {
    (0..=120_000).contains(&value).then_some(value)
}
