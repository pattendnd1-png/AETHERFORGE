use crate::{Notification, ProcessSnapshot, ResourceSnapshot, SystemError, SystemIntegration};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
pub struct LinuxSystem;
impl SystemIntegration for LinuxSystem {
    fn platform_name(&self) -> &'static str {
        "Linux"
    }
    fn data_dir(&self) -> Result<PathBuf, SystemError> {
        Ok(env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("aetherai"))
    }
    fn config_dir(&self) -> Result<PathBuf, SystemError> {
        Ok(env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("aetherai"))
    }
    fn resource_snapshot(&self) -> Result<ResourceSnapshot, SystemError> {
        let mem = fs::read_to_string("/proc/meminfo")?;
        let total = mem_kib(&mem, "MemTotal:") * 1024;
        let avail = mem_kib(&mem, "MemAvailable:") * 1024;
        let a = cpu_totals()?;
        thread::sleep(Duration::from_millis(40));
        let b = cpu_totals()?;
        let total_delta = b.0.saturating_sub(a.0);
        let idle_delta = b.1.saturating_sub(a.1);
        let busy_delta = total_delta.saturating_sub(idle_delta);
        let cpu = busy_delta
            .saturating_mul(100)
            .checked_div(total_delta)
            .unwrap_or(0)
            .min(100) as u8;
        Ok(ResourceSnapshot {
            total_memory_bytes: total,
            available_memory_bytes: avail,
            cpu_usage_percent: cpu,
            gpu_usage_percent: None,
            vram_total_bytes: None,
            vram_available_bytes: None,
        })
    }
    fn process_snapshot(&self) -> Result<Vec<ProcessSnapshot>, SystemError> {
        let mut out = Vec::new();
        for e in fs::read_dir("/proc")? {
            let e = match e {
                Ok(v) => v,
                Err(_) => continue,
            };
            let name = e.file_name();
            let Some(s) = name.to_str() else { continue };
            let Ok(pid) = s.parse::<u32>() else { continue };
            let root = e.path();
            let pname = fs::read_to_string(root.join("comm"))
                .unwrap_or_default()
                .trim()
                .to_string();
            let executable = fs::read_link(root.join("exe")).ok();
            let status = fs::read_to_string(root.join("status")).unwrap_or_default();
            let memory_bytes = status
                .lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0)
                * 1024;
            out.push(ProcessSnapshot {
                pid,
                name: pname,
                executable,
                cpu_usage_percent: 0,
                memory_bytes,
            });
        }
        Ok(out)
    }
    fn notify(&self, n: Notification) -> Result<(), SystemError> {
        let status = Command::new("notify-send")
            .arg(n.title)
            .arg(n.body)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        match status {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
    fn open_path(&self, path: &Path) -> Result<(), SystemError> {
        Command::new("xdg-open")
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(())
    }

    fn open_url(&self, url: &str) -> Result<(), SystemError> {
        crate::validate_external_https_url(url)?;
        Command::new("xdg-open")
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(())
    }

    fn pick_zip_file(&self, initial_dir: &Path) -> Result<Option<PathBuf>, SystemError> {
        let kdialog = Command::new("kdialog")
            .arg("--getopenfilename")
            .arg(initial_dir)
            .arg("ZIP archives (*.zip)")
            .output();

        match kdialog {
            Ok(output) if output.status.success() => {
                let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !selected.is_empty() {
                    return Ok(Some(PathBuf::from(selected)));
                }
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }

        let initial = format!("{}/", initial_dir.display());
        let zenity = Command::new("zenity")
            .args([
                "--file-selection",
                "--title=Import ChatGPT Export",
                "--file-filter=ZIP archives | *.zip",
            ])
            .arg(format!("--filename={initial}"))
            .output();

        match zenity {
            Ok(output) if output.status.success() => {
                let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if selected.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(PathBuf::from(selected)))
                }
            }
            Ok(_) => Ok(None),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}
fn mem_kib(text: &str, key: &str) -> u64 {
    text.lines()
        .find(|l| l.starts_with(key))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}
fn cpu_totals() -> Result<(u64, u64), SystemError> {
    let s = fs::read_to_string("/proc/stat")?;
    let line = s
        .lines()
        .next()
        .ok_or_else(|| SystemError::Platform("/proc/stat empty".into()))?;
    let v: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .filter_map(|x| x.parse().ok())
        .collect();
    let total = v.iter().sum();
    let idle = v.get(3).copied().unwrap_or(0) + v.get(4).copied().unwrap_or(0);
    Ok((total, idle))
}
