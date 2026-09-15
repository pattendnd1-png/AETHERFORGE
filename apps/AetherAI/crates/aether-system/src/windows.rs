use crate::{Notification, ProcessSnapshot, ResourceSnapshot, SystemError, SystemIntegration};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
pub struct WindowsSystem;
impl SystemIntegration for WindowsSystem {
    fn platform_name(&self) -> &'static str {
        "Windows"
    }
    fn data_dir(&self) -> Result<PathBuf, SystemError> {
        Ok(env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("AetherAI"))
    }
    fn config_dir(&self) -> Result<PathBuf, SystemError> {
        Ok(env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("AetherAI"))
    }
    fn resource_snapshot(&self) -> Result<ResourceSnapshot, SystemError> {
        let output=Command::new("powershell").args(["-NoProfile","-Command","$o=Get-CimInstance Win32_OperatingSystem; Write-Output ($o.TotalVisibleMemorySize.ToString()+' '+$o.FreePhysicalMemory.ToString())"]).output()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let mut i = text
            .split_whitespace()
            .filter_map(|v| v.parse::<u64>().ok());
        let total = i.next().unwrap_or(0) * 1024;
        let avail = i.next().unwrap_or(0) * 1024;
        Ok(ResourceSnapshot {
            total_memory_bytes: total,
            available_memory_bytes: avail,
            cpu_usage_percent: 0,
            gpu_usage_percent: None,
            vram_total_bytes: None,
            vram_available_bytes: None,
        })
    }
    fn process_snapshot(&self) -> Result<Vec<ProcessSnapshot>, SystemError> {
        Ok(Vec::new())
    }
    fn notify(&self, _: Notification) -> Result<(), SystemError> {
        Ok(())
    }
    fn open_path(&self, path: &Path) -> Result<(), SystemError> {
        Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(())
    }

    fn open_url(&self, url: &str) -> Result<(), SystemError> {
        crate::validate_external_https_url(url)?;
        Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        Ok(())
    }

    fn pick_zip_file(&self, initial_dir: &Path) -> Result<Option<PathBuf>, SystemError> {
        let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Title = 'Import ChatGPT Export'
$dialog.Filter = 'ZIP archives (*.zip)|*.zip'
$dialog.InitialDirectory = $env:AETHERAI_PICK_DIR
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    [Console]::Out.Write($dialog.FileName)
}
"#;
        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-STA", "-Command", script])
            .env("AETHERAI_PICK_DIR", initial_dir)
            .output()?;
        if !output.status.success() {
            return Ok(None);
        }
        let selected = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if selected.is_empty() {
            Ok(None)
        } else {
            Ok(Some(PathBuf::from(selected)))
        }
    }
}
