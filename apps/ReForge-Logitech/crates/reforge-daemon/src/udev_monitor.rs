use crate::{record_event, refresh_devices, unix_ms, SharedState};
use reforge_core::{DiagnosticLevel, ServiceKind};
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdevEvent {
    pub action: String,
    pub subsystem: String,
    pub devname: Option<String>,
}

pub fn parse_udevadm_block(block: &str) -> Option<UdevEvent> {
    let mut action = None;
    let mut subsystem = None;
    let mut devname = None;
    for line in block.lines() {
        let Some((key, value)) = line.trim().split_once('=') else {
            continue;
        };
        match key {
            "ACTION" => action = Some(value.to_ascii_lowercase()),
            "SUBSYSTEM" => subsystem = Some(value.to_ascii_lowercase()),
            "DEVNAME" => devname = Some(value.to_owned()),
            _ => {}
        }
    }
    let action = action?;
    let subsystem = subsystem?;
    if !matches!(action.as_str(), "add" | "remove" | "change") {
        return None;
    }
    if !matches!(subsystem.as_str(), "hidraw" | "usb") {
        return None;
    }
    Some(UdevEvent {
        action,
        subsystem,
        devname,
    })
}

fn mark_unavailable(shared: &SharedState, message: impl Into<String>) {
    if let Ok(mut services) = shared.services.lock() {
        services.unavailable(ServiceKind::Udev, "udevadm monitor", message);
    }
}

fn mark_degraded(shared: &SharedState, message: impl Into<String>) {
    if let Ok(mut services) = shared.services.lock() {
        services.degraded(ServiceKind::Udev, "udevadm monitor", message);
    }
}

fn mark_ready(shared: &SharedState) {
    if let Ok(mut services) = shared.services.lock() {
        services.ready(
            ServiceKind::Udev,
            "udevadm monitor",
            None,
            "udev hotplug monitor active",
            unix_ms(),
        );
    }
}

fn run_once(shared: &SharedState) -> Result<(), String> {
    let mut child = Command::new("udevadm")
        .args([
            "monitor",
            "--udev",
            "--property",
            "--subsystem-match=hidraw",
            "--subsystem-match=usb",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("failed to start udevadm monitor: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "udevadm monitor did not provide stdout".to_owned())?;
    mark_ready(shared);

    let mut block = String::new();
    let mut last_refresh = Instant::now() - Duration::from_secs(1);
    for line in BufReader::new(stdout).lines() {
        let line = line.map_err(|error| format!("failed reading udev monitor: {error}"))?;
        if line.trim().is_empty() {
            if let Some(event) = parse_udevadm_block(&block)
                && last_refresh.elapsed() >= Duration::from_millis(250)
            {
                if let Err(error) = refresh_devices(shared) {
                    record_event(
                        shared,
                        DiagnosticLevel::Error,
                        "udev",
                        format!("hotplug refresh failed after {} {}: {error}", event.action, event.subsystem),
                        None,
                    );
                } else {
                    last_refresh = Instant::now();
                }
            }
            block.clear();
        } else {
            block.push_str(&line);
            block.push('\n');
        }
    }

    let status = child
        .wait()
        .map_err(|error| format!("failed waiting for udevadm monitor: {error}"))?;
    Err(format!("udevadm monitor exited with {status}"))
}

pub fn monitor_loop(shared: SharedState) {
    loop {
        match run_once(&shared) {
            Ok(()) => {}
            Err(error) => {
                if error.contains("No such file") || error.contains("not found") {
                    mark_unavailable(&shared, &error);
                } else {
                    mark_degraded(&shared, &error);
                }
                record_event(&shared, DiagnosticLevel::Warning, "udev", error, None);
            }
        }
        thread::sleep(Duration::from_secs(2));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_logitech_relevant_hidraw_event() {
        let block = "UDEV [1.0] add /devices/example (hidraw)\nACTION=add\nDEVPATH=/devices/example\nSUBSYSTEM=hidraw\nDEVNAME=/dev/hidraw7\n";
        assert_eq!(
            parse_udevadm_block(block),
            Some(UdevEvent {
                action: "add".into(),
                subsystem: "hidraw".into(),
                devname: Some("/dev/hidraw7".into()),
            })
        );
    }

    #[test]
    fn ignores_irrelevant_subsystem_and_action() {
        assert!(parse_udevadm_block("ACTION=bind\nSUBSYSTEM=hidraw\n").is_none());
        assert!(parse_udevadm_block("ACTION=add\nSUBSYSTEM=net\n").is_none());
    }
}
