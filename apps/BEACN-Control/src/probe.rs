use crate::{pipewire, usb};
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn write_probe(path: &Path) -> io::Result<()> {
    let mut out = String::new();
    let _ = writeln!(out, "AETHERFORGE_BEACN_CONTROL_VERSION=0.1.12");
    let _ = writeln!(out, "AETHERFORGE_BEACN_PROBE_MODE=READ_ONLY");
    let _ = writeln!(
        out,
        "AETHERFORGE_BEACN_DIRECT_USB_CONTROL=BLOCKED_SYSTEM_AUDIO_PROTECTION"
    );
    let _ = writeln!(out, "AETHERFORGE_BEACN_SOFTWARE_DSP_PROFILE_MODEL=ACTIVE");
    let _ = writeln!(
        out,
        "AETHERFORGE_BEACN_AETHERSTREAM_MUTATING_DSP=UNAVAILABLE_UNTIL_VERIFIED_TYPED_ADAPTER"
    );
    let _ = writeln!(out, "UNIX_TIME={}", unix_time());

    let usb_devices = usb::discover_beacn_devices();
    let usb_present = usb_devices
        .as_ref()
        .is_ok_and(|devices| !devices.is_empty());

    match &usb_devices {
        Ok(devices) => {
            let _ = writeln!(out, "BEACN_USB_DEVICE_COUNT={}", devices.len());
            for (index, device) in devices.iter().enumerate() {
                let _ = writeln!(out, "USB_{index}_PRODUCT={}", device.product);
                let _ = writeln!(out, "USB_{index}_MANUFACTURER={}", device.manufacturer);
                let _ = writeln!(out, "USB_{index}_VID={}", device.vendor_id);
                let _ = writeln!(out, "USB_{index}_PID={}", device.product_id);
                let _ = writeln!(out, "USB_{index}_SERIAL={}", device.serial);
                let _ = writeln!(out, "USB_{index}_BUS={}", device.bus_number);
                let _ = writeln!(out, "USB_{index}_DEV={}", device.device_number);
                let _ = writeln!(out, "USB_{index}_SYSFS={}", device.sysfs_path.display());
            }
        }
        Err(error) => {
            let _ = writeln!(out, "BEACN_USB_DISCOVERY_ERROR={error}");
        }
    }

    match pipewire::status() {
        Ok(graph) => {
            let _ = writeln!(out, "PIPEWIRE_DEVICE_COUNT={}", graph.devices.len());
            for node in &graph.devices {
                let _ = writeln!(
                    out,
                    "PIPEWIRE_DEVICE={}|{}|default={}",
                    node.id, node.name, node.is_default
                );
            }
            let _ = writeln!(out, "PIPEWIRE_SOURCE_COUNT={}", graph.sources.len());
            for node in &graph.sources {
                let _ = writeln!(
                    out,
                    "PIPEWIRE_SOURCE={}|{}|default={}",
                    node.id, node.name, node.is_default
                );
            }
            let _ = writeln!(out, "PIPEWIRE_SINK_COUNT={}", graph.sinks.len());
            for node in &graph.sinks {
                let _ = writeln!(
                    out,
                    "PIPEWIRE_SINK={}|{}|default={}",
                    node.id, node.name, node.is_default
                );
            }
            let _ = writeln!(
                out,
                "BEACN_PIPEWIRE_HEALTH={}",
                pipewire::beacn_audio_health(&graph, usb_present)
            );
        }
        Err(error) => {
            let _ = writeln!(out, "PIPEWIRE_DISCOVERY_ERROR={error}");
        }
    }

    match pipewire::pulse_cards() {
        Ok(cards) => {
            if let Some(card) = cards
                .iter()
                .find(|card| pipewire::is_beacn_card_name(&card.name))
            {
                let _ = writeln!(out, "BEACN_PULSE_CARD={}", card.name);
                let _ = writeln!(
                    out,
                    "BEACN_PULSE_ACTIVE_PROFILE={}",
                    card.active_profile.as_deref().unwrap_or("unknown")
                );
                let candidate = pipewire::preferred_output_profile(card)
                    .map_or("none", |profile| profile.name.as_str());
                let _ = writeln!(out, "BEACN_PULSE_OUTPUT_PROFILE_CANDIDATE={candidate}");
            } else {
                let _ = writeln!(out, "BEACN_PULSE_CARD=not-found");
            }
        }
        Err(error) => {
            let _ = writeln!(out, "BEACN_PULSE_PROFILE_ERROR={error}");
        }
    }

    append_command(&mut out, "WPCTL_STATUS", "wpctl", &["status", "-n"]);
    append_command(&mut out, "PACTL_CARDS", "pactl", &["list", "cards"]);
    append_command(&mut out, "ARECORD_LIST", "arecord", &["-l"]);
    append_command(&mut out, "APLAY_LIST", "aplay", &["-l"]);
    append_command(
        &mut out,
        "WIREPLUMBER_SERVICE",
        "systemctl",
        &["--user", "is-active", "wireplumber.service"],
    );
    append_command(&mut out, "LSUSB", "lsusb", &[]);
    fs::write(path, out)
}

fn append_command(out: &mut String, label: &str, command: &str, args: &[&str]) {
    let _ = writeln!(out, "--- {label} ---");
    match Command::new(command).args(args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            out.push_str(&stdout);
            if !stderr.trim().is_empty() {
                let _ = writeln!(out, "STDERR={}", stderr.trim());
            }
            if !output.status.success() {
                let _ = writeln!(out, "EXIT_STATUS={}", output.status);
            }
        }
        Err(error) => {
            let _ = writeln!(out, "COMMAND_ERROR={error}");
        }
    }
}

fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
