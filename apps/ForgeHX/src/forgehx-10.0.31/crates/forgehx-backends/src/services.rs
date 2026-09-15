use crate::{BackendError, OpenRgbClient, RatbagClient};
use forgehx_core::BackendKind;
use std::process::Command;
use std::thread;
use std::time::Duration;

#[derive(Debug, Default, Clone, Copy)]
pub struct BackendServiceManager;

impl BackendServiceManager {
    pub fn ensure(&self, backend: BackendKind) -> Result<String, BackendError> {
        match backend {
            BackendKind::ForgeHxNative => Ok("ForgeHX native backend is in-process".into()),
            BackendKind::ForgeHxDsp => {
                if command_succeeds("wpctl", &["status"]) {
                    return Ok("ForgeHX input DSP has a reachable PipeWire control plane".into());
                }
                user_systemctl("start", &["pipewire.service", "wireplumber.service"])?;
                wait_for_command(
                    "wpctl",
                    &["status"],
                    "PipeWire started but ForgeHX input DSP still cannot reach it",
                )?;
                Ok("ForgeHX input DSP control plane started".into())
            }
            BackendKind::Diagnostic => Ok("diagnostic backend is in-process".into()),
            BackendKind::OpenRgb => {
                if OpenRgbClient::default().probe().is_ok() {
                    return Ok("OpenRGB SDK is already reachable".into());
                }
                user_systemctl("start", &["forgehx-openrgb.service"])?;
                wait_for_openrgb()?;
                Ok("OpenRGB compatibility service started".into())
            }
            BackendKind::LinuxStandard => {
                if command_succeeds("wpctl", &["status"]) {
                    return Ok("PipeWire/WirePlumber are already reachable".into());
                }
                user_systemctl("start", &["pipewire.service", "wireplumber.service"])?;
                wait_for_command(
                    "wpctl",
                    &["status"],
                    "PipeWire started but wpctl still cannot reach it",
                )?;
                Ok("PipeWire and WirePlumber started".into())
            }
            BackendKind::Ratbag => RatbagClient.devices().map(|devices| {
                format!("ratbagd D-Bus backend ready ({} device(s))", devices.len())
            }),
        }
    }

    pub fn restart(&self, backend: BackendKind) -> Result<String, BackendError> {
        match backend {
            BackendKind::ForgeHxNative => {
                Ok("ForgeHX native backend reloads with the daemon".into())
            }
            BackendKind::ForgeHxDsp => {
                user_systemctl("restart", &["pipewire.service", "wireplumber.service"])?;
                wait_for_command(
                    "wpctl",
                    &["status"],
                    "PipeWire restarted but ForgeHX input DSP still cannot reach it",
                )?;
                Ok("ForgeHX input DSP control plane restarted".into())
            }
            BackendKind::Diagnostic => Ok("diagnostic backend does not require restart".into()),
            BackendKind::OpenRgb => {
                user_systemctl("restart", &["forgehx-openrgb.service"])?;
                wait_for_openrgb()?;
                Ok("OpenRGB compatibility service restarted".into())
            }
            BackendKind::LinuxStandard => {
                user_systemctl("restart", &["pipewire.service", "wireplumber.service"])?;
                wait_for_command(
                    "wpctl",
                    &["status"],
                    "PipeWire restarted but wpctl still cannot reach it",
                )?;
                Ok("PipeWire and WirePlumber restarted".into())
            }
            BackendKind::Ratbag => {
                // ratbagd is a system D-Bus activated service; invoking ratbagctl
                // requests activation without requiring ForgeHX to elevate privileges.
                RatbagClient.devices().map(|devices| {
                    format!("ratbagd activation requested ({} device(s))", devices.len())
                })
            }
        }
    }

    pub fn ensure_all(&self) -> (Vec<String>, Vec<String>) {
        let mut ready = Vec::new();
        let mut errors = Vec::new();
        for backend in [
            BackendKind::LinuxStandard,
            BackendKind::OpenRgb,
            BackendKind::Ratbag,
        ] {
            match self.ensure(backend) {
                Ok(message) => ready.push(message),
                Err(error) => errors.push(format!("{backend}: {error}")),
            }
        }
        (ready, errors)
    }
}

fn command_succeeds(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .output()
        .is_ok_and(|output| output.status.success())
}

fn wait_for_openrgb() -> Result<(), BackendError> {
    let client = OpenRgbClient::default();
    for _ in 0..20 {
        if client.probe().is_ok() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(BackendError::Unavailable(
        "OpenRGB service started but SDK port 6742 is not reachable".into(),
    ))
}

fn wait_for_command(program: &str, args: &[&str], message: &str) -> Result<(), BackendError> {
    for _ in 0..20 {
        if command_succeeds(program, args) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(BackendError::Unavailable(message.into()))
}

fn user_systemctl(action: &str, services: &[&str]) -> Result<(), BackendError> {
    let mut args = vec!["--user", action];
    args.extend_from_slice(services);
    let output = Command::new("systemctl").args(args).output().map_err(|e| {
        BackendError::Unavailable(format!("systemctl --user is not available: {e}"))
    })?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(BackendError::Io(if stderr.is_empty() {
            format!("systemctl --user {action} failed")
        } else {
            stderr
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_process_backends_do_not_require_system_services() {
        let manager = BackendServiceManager;
        assert!(manager.ensure(BackendKind::ForgeHxNative).is_ok());
        assert!(manager.restart(BackendKind::Diagnostic).is_ok());
    }
}
