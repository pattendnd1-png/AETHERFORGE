use forgehx_core::{Command, Reply, IPC_PROTOCOL_VERSION};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;
use std::thread;
use std::time::Duration;

const MAX_FRAME: usize = 4 * 1024 * 1024;
const RESTART_ATTEMPTS: usize = 30;
const RESTART_DELAY: Duration = Duration::from_millis(50);

pub fn socket_path() -> PathBuf {
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime).join("forgehx/forgehx.sock")
    } else {
        let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
        PathBuf::from(format!("/tmp/forgehx-{user}/forgehx.sock"))
    }
}

pub fn send(command: Command) -> Result<Reply, String> {
    // Negotiate before sending the real command. This is essential for new
    // command variants: an older daemon cannot report a version mismatch for
    // JSON it cannot deserialize in the first place.
    let ping = Command::Ping {
        protocol_version: IPC_PROTOCOL_VERSION,
    };
    match raw_send(&ping) {
        Ok(Reply::Pong {
            protocol_version: daemon_version,
            min_protocol_version: daemon_min,
        }) => send_after_negotiation(command, daemon_version, daemon_min),
        Ok(Reply::Error { code, message }) if code == "protocol_version" => {
            recover_legacy_mismatch(command, &message)
        }
        Ok(other) => Err(format!(
            "unexpected ForgeHX protocol negotiation reply: {other:?}"
        )),
        Err(error) => Err(error),
    }
}

fn send_after_negotiation(
    command: Command,
    daemon_version: u32,
    daemon_min: u32,
) -> Result<Reply, String> {
    if IPC_PROTOCOL_VERSION < daemon_min {
        return Err(format!(
            "ForgeHX client protocol {IPC_PROTOCOL_VERSION} is too old for daemon protocol {daemon_version} (daemon requires {daemon_min}..={daemon_version})."
        ));
    }

    if daemon_version < IPC_PROTOCOL_VERSION {
        // Most commonly this is a package upgrade with the previous user
        // daemon still resident. Prefer the newly installed daemon before
        // intentionally downgrading a compatible command.
        if restart_daemon_service().is_ok() && wait_for_current_daemon() {
            return raw_send(&command.with_protocol_version(IPC_PROTOCOL_VERSION));
        }
    }

    let wire_version = daemon_version.min(IPC_PROTOCOL_VERSION);
    if wire_version < command.minimum_protocol_version() {
        return Err(format!(
            "ForgeHX IPC mismatch: daemon protocol {daemon_version}, client protocol {IPC_PROTOCOL_VERSION}; this command requires protocol {}. Restart with `systemctl --user restart forgehx-daemon.service`.",
            command.minimum_protocol_version()
        ));
    }
    raw_send(&command.with_protocol_version(wire_version))
}

fn recover_legacy_mismatch(command: Command, message: &str) -> Result<Reply, String> {
    let daemon_version = parse_daemon_version(message);

    // ForgeHX v0.2 rejects even Ping when its version differs. Restart the
    // installed service first so upgrades self-heal without manual cleanup.
    if restart_daemon_service().is_ok() && wait_for_current_daemon() {
        return raw_send(&command.with_protocol_version(IPC_PROTOCOL_VERSION));
    }

    if let Some(version) = daemon_version {
        return send_after_negotiation(command, version, version);
    }

    Err(format!(
        "ForgeHX IPC mismatch: {message}. Restart the daemon with `systemctl --user restart forgehx-daemon.service`."
    ))
}

fn raw_send(command: &Command) -> Result<Reply, String> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path).map_err(|e| {
        format!(
            "cannot connect to ForgeHX daemon at {}: {e}. Start it with `systemctl --user start forgehx-daemon.service` or run `forgehx-daemon`.",
            path.display()
        )
    })?;
    let payload = serde_json::to_vec(command).map_err(|e| e.to_string())?;
    let length = u32::try_from(payload.len()).map_err(|_| "request too large")?;
    stream
        .write_all(&length.to_be_bytes())
        .map_err(|e| e.to_string())?;
    stream.write_all(&payload).map_err(|e| e.to_string())?;

    let mut length = [0u8; 4];
    stream.read_exact(&mut length).map_err(|e| e.to_string())?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > MAX_FRAME {
        return Err("invalid ForgeHX response frame".into());
    }
    let mut payload = vec![0u8; length];
    stream.read_exact(&mut payload).map_err(|e| e.to_string())?;
    serde_json::from_slice(&payload).map_err(|e| e.to_string())
}

fn restart_daemon_service() -> Result<(), String> {
    let output = ProcessCommand::new("systemctl")
        .args(["--user", "restart", "forgehx-daemon.service"])
        .output()
        .map_err(|e| format!("cannot run systemctl --user: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

fn wait_for_current_daemon() -> bool {
    for _ in 0..RESTART_ATTEMPTS {
        thread::sleep(RESTART_DELAY);
        let ping = Command::Ping {
            protocol_version: IPC_PROTOCOL_VERSION,
        };
        if matches!(
            raw_send(&ping),
            Ok(Reply::Pong {
                protocol_version: IPC_PROTOCOL_VERSION,
                ..
            })
        ) {
            return true;
        }
    }
    false
}

fn parse_daemon_version(message: &str) -> Option<u32> {
    let tail = message.split("daemon=").nth(1)?;
    let digits = tail
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    (!digits.is_empty()).then(|| digits.parse().ok()).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use forgehx_core::DeviceId;

    #[test]
    fn parses_old_daemon_mismatch_message() {
        assert_eq!(
            parse_daemon_version("protocol version mismatch: daemon=2, client=3"),
            Some(2)
        );
        assert_eq!(parse_daemon_version("different error"), None);
    }

    #[test]
    fn new_control_command_is_not_compatible_with_v2() {
        let command = Command::MouseState {
            protocol_version: IPC_PROTOCOL_VERSION,
            device_id: DeviceId("mouse".into()),
        };
        assert_eq!(command.minimum_protocol_version(), 3);
    }

    #[test]
    fn mouse_capabilities_is_v5_only() {
        let command = Command::MouseCapabilities {
            protocol_version: IPC_PROTOCOL_VERSION,
            device_id: DeviceId("mouse".into()),
        };
        assert_eq!(command.minimum_protocol_version(), 5);
    }

    #[test]
    fn v2_inventory_command_can_be_downgraded() {
        let command = Command::ListAllDevices {
            protocol_version: IPC_PROTOCOL_VERSION,
        };
        assert_eq!(command.minimum_protocol_version(), 2);
        assert_eq!(command.with_protocol_version(2).protocol_version(), 2);
    }

    #[test]
    fn parser_reads_v2_strict_mismatch() {
        let message = "protocol version mismatch: daemon=2, client=3";
        assert_eq!(parse_daemon_version(message), Some(2));
    }
}
