use crate::adaptive::health::{HealthIncident, HealthSample, SystemHealth};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct AuditEngine;

impl AuditEngine {
    pub fn quick(&self) -> io::Result<SystemHealth> {
        self.run(false)
    }

    pub fn full(&self) -> io::Result<SystemHealth> {
        self.run(true)
    }

    fn run(&self, full: bool) -> io::Result<SystemHealth> {
        let mut out = SystemHealth::empty_now();

        if let Some(v) = read_load1() {
            out.samples.push(HealthSample::CpuLoad1(v));
        }
        if let Some(v) = read_mem_available_bytes() {
            out.samples.push(HealthSample::MemoryAvailableBytes(v));
        }
        if let Some(v) = read_io_psi_avg10() {
            out.samples.push(HealthSample::IoPsiAvg10(v));
        }
        if let Some(v) = read_cpu_temp_c() {
            out.samples.push(HealthSample::CpuTemperatureC(v));
        }
        if let Some(v) = read_gpu_busy_pct() {
            out.samples.push(HealthSample::GpuBusyPct(v));
        }
        if let Some(v) = read_gpu_junction_c() {
            out.samples.push(HealthSample::GpuJunctionC(v));
        }
        let (rx, tx) = read_network_bytes().unwrap_or((0, 0));
        out.samples.push(HealthSample::NetworkRxBytes(rx));
        out.samples.push(HealthSample::NetworkTxBytes(tx));
        if let Some(v) = read_tcp_retransmits() {
            out.samples.push(HealthSample::NetworkRetransmits(v));
        }
        out.samples
            .push(HealthSample::BeacnConnected(beacn_connected()));

        if full {
            append_failed_units(&mut out, false);
            append_failed_units(&mut out, true);
        }

        Ok(out)
    }
}

fn read_load1() -> Option<f64> {
    fs::read_to_string("/proc/loadavg")
        .ok()?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

fn read_mem_available_bytes() -> Option<u64> {
    for line in fs::read_to_string("/proc/meminfo").ok()?.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            let kb = rest.split_whitespace().next()?.parse::<u64>().ok()?;
            return Some(kb.saturating_mul(1024));
        }
    }
    None
}

fn read_io_psi_avg10() -> Option<f64> {
    let text = fs::read_to_string("/proc/pressure/io").ok()?;
    let line = text.lines().find(|l| l.starts_with("some "))?;
    for field in line.split_whitespace() {
        if let Some(v) = field.strip_prefix("avg10=") {
            return v.parse().ok();
        }
    }
    None
}

fn hwmon_dirs() -> Vec<PathBuf> {
    fs::read_dir("/sys/class/hwmon")
        .ok()
        .into_iter()
        .flat_map(|rd| rd.filter_map(Result::ok))
        .map(|e| e.path())
        .collect()
}

fn hwmon_name(dir: &Path) -> String {
    fs::read_to_string(dir.join("name"))
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
}

fn read_millideg(path: &Path) -> Option<f64> {
    let raw = fs::read_to_string(path).ok()?.trim().parse::<f64>().ok()?;
    Some(raw / 1000.0)
}

fn temp_by_label(dir: &Path, labels: &[&str]) -> Option<f64> {
    for idx in 1..=16 {
        let label_path = dir.join(format!("temp{idx}_label"));
        let input_path = dir.join(format!("temp{idx}_input"));
        let label = fs::read_to_string(label_path)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if labels.iter().any(|needle| label.contains(needle))
            && let Some(v) = read_millideg(&input_path)
        {
            return Some(v);
        }
    }
    None
}

fn read_cpu_temp_c() -> Option<f64> {
    for dir in hwmon_dirs() {
        let name = hwmon_name(&dir);
        if (name.contains("k10temp") || name.contains("zenpower"))
            && let Some(v) = temp_by_label(&dir, &["tctl", "tdie", "ccd"])
        {
            return Some(v);
        }
    }
    None
}

fn read_gpu_busy_pct() -> Option<f64> {
    let entries = fs::read_dir("/sys/class/drm").ok()?;
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("card") || name.contains('-') {
            continue;
        }
        let p = entry.path().join("device/gpu_busy_percent");
        if let Ok(v) = fs::read_to_string(p)
            && let Ok(v) = v.trim().parse::<f64>()
        {
            return Some(v);
        }
    }
    None
}

fn read_gpu_junction_c() -> Option<f64> {
    for dir in hwmon_dirs() {
        if hwmon_name(&dir).contains("amdgpu")
            && let Some(v) = temp_by_label(&dir, &["junction", "hotspot"])
        {
            return Some(v);
        }
    }
    None
}

fn read_network_bytes() -> Option<(u64, u64)> {
    let text = fs::read_to_string("/proc/net/dev").ok()?;
    let mut rx = 0u64;
    let mut tx = 0u64;
    for line in text.lines().skip(2) {
        let (iface, rest) = line.split_once(':')?;
        if iface.trim() == "lo" {
            continue;
        }
        let fields: Vec<&str> = rest.split_whitespace().collect();
        if fields.len() >= 9 {
            rx = rx.saturating_add(fields[0].parse::<u64>().unwrap_or(0));
            tx = tx.saturating_add(fields[8].parse::<u64>().unwrap_or(0));
        }
    }
    Some((rx, tx))
}

fn read_tcp_retransmits() -> Option<u64> {
    let text = fs::read_to_string("/proc/net/snmp").ok()?;
    let mut tcp_lines = text.lines().filter(|line| line.starts_with("Tcp:"));
    loop {
        let header = tcp_lines.next()?;
        let values = tcp_lines.next()?;
        let keys: Vec<&str> = header.split_whitespace().collect();
        let vals: Vec<&str> = values.split_whitespace().collect();
        if keys.len() != vals.len() {
            continue;
        }
        if let Some(index) = keys.iter().position(|k| *k == "RetransSegs") {
            return vals.get(index)?.parse().ok();
        }
    }
}

fn beacn_connected() -> bool {
    let Ok(entries) = fs::read_dir("/sys/bus/usb/devices") else {
        return false;
    };
    for entry in entries.filter_map(Result::ok) {
        let dir = entry.path();
        let vendor = fs::read_to_string(dir.join("idVendor"))
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let product = fs::read_to_string(dir.join("idProduct"))
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if vendor == "33ae" && product == "8001" {
            return true;
        }
    }
    false
}

fn append_failed_units(out: &mut SystemHealth, user: bool) {
    let mut cmd = Command::new("systemctl");
    if user {
        cmd.arg("--user");
    }
    let result = cmd.args(["--failed", "--no-legend", "--plain"]).output();
    let Ok(output) = result else {
        return;
    };
    if !output.status.success() {
        return;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        out.incidents.push(HealthIncident {
            code: if user {
                "FAILED_USER_UNIT".into()
            } else {
                "FAILED_SYSTEM_UNIT".into()
            },
            detail: line.trim().to_string(),
            fatal_for_tuning: false,
        });
    }
}
