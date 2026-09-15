use clap::{Parser, Subcommand};
use forgehx_core::{
    AudioNode, BackendKind, Command, DeviceId, DpiConfig, EqConfig, LightingConfig, LightingEffect,
    MicrophoneDspConfig, Profile, Reply, IPC_PROTOCOL_VERSION,
};
use forgehx_ipc::send;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "forgehx",
    version,
    about = "ForgeHX Linux peripheral control CLI"
)]
struct Cli {
    #[command(subcommand)]
    command: TopCommand,
}

#[derive(Debug, Subcommand)]
enum TopCommand {
    /// List logical HyperX devices; use --all for the full peripheral inventory.
    List {
        #[arg(long)]
        all: bool,
    },
    /// Print grouped device diagnostics.
    Doctor {
        device: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Inspect and rescan ForgeHX control backends.
    Backend {
        #[command(subcommand)]
        command: BackendCommand,
    },
    /// Manage persistent ForgeHX profiles.
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    /// Control lighting through the selected ForgeHX/OpenRGB backend.
    Rgb {
        device: String,
        #[arg(long, default_value = "#ffffff")]
        color: String,
        #[arg(long, default_value_t = 100)]
        brightness: u8,
        #[arg(long, default_value_t = 50)]
        speed: u8,
        #[arg(long, value_enum, default_value_t = EffectArg::Static)]
        effect: EffectArg,
    },
    /// Inspect lighting metadata for a device.
    RgbInfo { device: String },
    /// Set DPI stages (compatibility alias for `mouse dpi`).
    Dpi {
        device: String,
        #[arg(required = true)]
        stages: Vec<u16>,
        #[arg(long, default_value_t = 0)]
        active: usize,
    },
    /// Control gaming-mouse features through the selected native/libratbag backend.
    Mouse {
        #[command(subcommand)]
        command: MouseCommand,
    },
    /// Inspect HyperX keyboard model capabilities and backend ownership.
    Keyboard {
        #[command(subcommand)]
        command: KeyboardCommand,
    },
    /// Control safe PipeWire/WirePlumber audio nodes.
    Audio {
        #[command(subcommand)]
        command: AudioCommand,
    },
    /// Control HyperX microphone input processing.
    Mic {
        #[command(subcommand)]
        command: MicCommand,
    },
    /// Manage ForgeHX PipeWire parametric EQ profiles.
    Eq {
        #[command(subcommand)]
        command: EqCommand,
    },
    /// Inspect or refresh daemon state.
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
}

#[derive(Debug, Subcommand)]
enum BackendCommand {
    Status,
    Rescan,
    /// Start/activate the Linux compatibility control backends.
    Ensure,
    /// Restart one control backend.
    Restart {
        #[arg(value_enum)]
        backend: BackendArg,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum BackendArg {
    Native,
    Dsp,
    Linux,
    Openrgb,
    Ratbag,
    Diagnostic,
}

impl From<BackendArg> for BackendKind {
    fn from(value: BackendArg) -> Self {
        match value {
            BackendArg::Native => BackendKind::ForgeHxNative,
            BackendArg::Dsp => BackendKind::ForgeHxDsp,
            BackendArg::Linux => BackendKind::LinuxStandard,
            BackendArg::Openrgb => BackendKind::OpenRgb,
            BackendArg::Ratbag => BackendKind::Ratbag,
            BackendArg::Diagnostic => BackendKind::Diagnostic,
        }
    }
}

#[derive(Debug, Subcommand)]
enum ProfileCommand {
    List,
    Get { name: String },
    Save { file: PathBuf },
    Apply { name: String, device: String },
}

#[derive(Debug, Subcommand)]
enum MouseCommand {
    State {
        device: String,
    },
    Capabilities {
        device: String,
    },
    Dpi {
        device: String,
        #[arg(required = true)]
        stages: Vec<u16>,
        #[arg(long, default_value_t = 0)]
        active: usize,
    },
    Rate {
        device: String,
        hz: u16,
    },
    Profile {
        device: String,
        profile: u8,
    },
    Button {
        device: String,
        button: u32,
        action: String,
    },
    LiftOff {
        device: String,
        mm: u8,
    },
}

#[derive(Debug, Subcommand)]
enum KeyboardCommand {
    Capabilities { device: String },
}

#[derive(Debug, Subcommand)]
enum AudioCommand {
    List,
    Volume { node: u32, volume: f32 },
    Mute { node: u32, muted: bool },
}

#[derive(Debug, Subcommand)]
enum MicCommand {
    Dsp {
        #[command(subcommand)]
        command: MicDspCommand,
    },
    Firmware {
        #[command(subcommand)]
        command: MicFirmwareCommand,
    },
}

#[derive(Debug, Subcommand)]
enum MicFirmwareCommand {
    Get { device: String },
    Stage { device: String, file: PathBuf },
    Validate { device: String, staged_id: String },
    Begin { device: String, staged_id: String },
    Status { transaction_id: String },
    Forget { staged_id: String },
}

#[derive(Debug, Subcommand)]
enum MicDspCommand {
    Get { device: String },
    Save { device: String, file: PathBuf },
    Apply { device: String, name: String },
    Bypass { device: String },
}

#[derive(Debug, Subcommand)]
enum EqCommand {
    List,
    Get { name: String },
    Save { file: PathBuf },
    Delete { name: String },
    Apply { name: String },
    Bypass,
}

#[derive(Debug, Subcommand)]
enum DaemonCommand {
    Status,
    Refresh,
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum EffectArg {
    Static,
    Breathing,
    Wave,
    Spectrum,
}

impl From<EffectArg> for LightingEffect {
    fn from(value: EffectArg) -> Self {
        match value {
            EffectArg::Static => Self::Static,
            EffectArg::Breathing => Self::Breathing,
            EffectArg::Wave => Self::Wave,
            EffectArg::Spectrum => Self::Spectrum,
        }
    }
}

fn main() {
    let cli = Cli::parse();
    if let Err(error) = run(cli) {
        eprintln!("forgehx: {error}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), String> {
    let p = IPC_PROTOCOL_VERSION;
    match cli.command {
        TopCommand::List { all } => {
            let command = if all {
                Command::ListAllDevices {
                    protocol_version: p,
                }
            } else {
                Command::ListHyperxDevices {
                    protocol_version: p,
                }
            };
            print_reply(send(command)?, false);
        }
        TopCommand::Doctor { device, json } => print_reply(
            send(Command::Doctor {
                protocol_version: p,
                device_id: device.map(DeviceId),
            })?,
            json,
        ),
        TopCommand::Backend { command } => match command {
            BackendCommand::Status => print_reply(
                send(Command::BackendStatus {
                    protocol_version: p,
                })?,
                false,
            ),
            BackendCommand::Rescan => print_reply(
                send(Command::BackendRescan {
                    protocol_version: p,
                })?,
                false,
            ),
            BackendCommand::Ensure => print_reply(
                send(Command::BackendEnsure {
                    protocol_version: p,
                })?,
                false,
            ),
            BackendCommand::Restart { backend } => print_reply(
                send(Command::BackendRestart {
                    protocol_version: p,
                    backend: backend.into(),
                })?,
                false,
            ),
        },
        TopCommand::Profile { command } => match command {
            ProfileCommand::List => print_reply(
                send(Command::ProfileList {
                    protocol_version: p,
                })?,
                false,
            ),
            ProfileCommand::Get { name } => print_reply(
                send(Command::ProfileGet {
                    protocol_version: p,
                    name,
                })?,
                true,
            ),
            ProfileCommand::Save { file } => {
                let bytes =
                    fs::read(&file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
                let profile: Profile = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("invalid profile JSON {}: {e}", file.display()))?;
                let profile = profile.migrate().map_err(|e| e.to_string())?;
                profile.validate().map_err(|e| e.to_string())?;
                print_reply(
                    send(Command::ProfileSave {
                        protocol_version: p,
                        profile,
                    })?,
                    false,
                );
            }
            ProfileCommand::Apply { name, device } => print_reply(
                send(Command::ProfileApply {
                    protocol_version: p,
                    name,
                    device_id: DeviceId(device),
                })?,
                false,
            ),
        },
        TopCommand::Rgb {
            device,
            color,
            brightness,
            speed,
            effect,
        } => {
            let color = parse_hex_color(&color)?;
            print_reply(
                send(Command::SetLighting {
                    protocol_version: p,
                    device_id: DeviceId(device),
                    config: LightingConfig {
                        effect: effect.into(),
                        color,
                        brightness: brightness.min(100),
                        speed: speed.min(100),
                    },
                })?,
                false,
            );
        }
        TopCommand::RgbInfo { device } => print_reply(
            send(Command::LightingMetadata {
                protocol_version: p,
                device_id: DeviceId(device),
            })?,
            true,
        ),
        TopCommand::Dpi {
            device,
            stages,
            active,
        } => print_reply(
            send(Command::SetDpi {
                protocol_version: p,
                device_id: DeviceId(device),
                config: DpiConfig {
                    stages,
                    active_stage: active,
                },
            })?,
            false,
        ),
        TopCommand::Mouse { command } => match command {
            MouseCommand::State { device } => print_reply(
                send(Command::MouseState {
                    protocol_version: p,
                    device_id: DeviceId(device),
                })?,
                true,
            ),
            MouseCommand::Capabilities { device } => print_reply(
                send(Command::MouseCapabilities {
                    protocol_version: p,
                    device_id: DeviceId(device),
                })?,
                true,
            ),
            MouseCommand::Dpi {
                device,
                stages,
                active,
            } => print_reply(
                send(Command::SetDpi {
                    protocol_version: p,
                    device_id: DeviceId(device),
                    config: DpiConfig {
                        stages,
                        active_stage: active,
                    },
                })?,
                false,
            ),
            MouseCommand::Rate { device, hz } => print_reply(
                send(Command::SetPollingRate {
                    protocol_version: p,
                    device_id: DeviceId(device),
                    hz,
                })?,
                false,
            ),
            MouseCommand::Profile { device, profile } => print_reply(
                send(Command::SetMouseProfile {
                    protocol_version: p,
                    device_id: DeviceId(device),
                    profile,
                })?,
                false,
            ),
            MouseCommand::Button {
                device,
                button,
                action,
            } => print_reply(
                send(Command::SetButtonAssignment {
                    protocol_version: p,
                    device_id: DeviceId(device),
                    button,
                    action,
                })?,
                false,
            ),
            MouseCommand::LiftOff { device, mm } => print_reply(
                send(Command::SetLiftOffDistance {
                    protocol_version: p,
                    device_id: DeviceId(device),
                    mm,
                })?,
                false,
            ),
        },
        TopCommand::Keyboard { command } => match command {
            KeyboardCommand::Capabilities { device } => print_reply(
                send(Command::KeyboardCapabilities {
                    protocol_version: p,
                    device_id: DeviceId(device),
                })?,
                true,
            ),
        },
        TopCommand::Audio { command } => match command {
            AudioCommand::List => print_reply(
                send(Command::ListAudio {
                    protocol_version: p,
                })?,
                false,
            ),
            AudioCommand::Volume { node, volume } => print_reply(
                send(Command::AudioSetVolume {
                    protocol_version: p,
                    node_id: node,
                    volume,
                })?,
                false,
            ),
            AudioCommand::Mute { node, muted } => print_reply(
                send(Command::AudioSetMute {
                    protocol_version: p,
                    node_id: node,
                    muted,
                })?,
                false,
            ),
        },
        TopCommand::Mic { command } => match command {
            MicCommand::Dsp { command } => match command {
                MicDspCommand::Get { device } => print_reply(
                    send(Command::MicDspGet {
                        protocol_version: p,
                        device_id: DeviceId(device),
                    })?,
                    true,
                ),
                MicDspCommand::Save { device, file } => {
                    let bytes = fs::read(&file)
                        .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
                    let config: MicrophoneDspConfig =
                        serde_json::from_slice(&bytes).map_err(|e| {
                            format!("invalid microphone DSP JSON {}: {e}", file.display())
                        })?;
                    config.validate().map_err(|e| e.to_string())?;
                    print_reply(
                        send(Command::MicDspSave {
                            protocol_version: p,
                            device_id: DeviceId(device),
                            config,
                        })?,
                        false,
                    );
                }
                MicDspCommand::Apply { device, name } => print_reply(
                    send(Command::MicDspApply {
                        protocol_version: p,
                        device_id: DeviceId(device),
                        name,
                    })?,
                    false,
                ),
                MicDspCommand::Bypass { device } => print_reply(
                    send(Command::MicDspBypass {
                        protocol_version: p,
                        device_id: DeviceId(device),
                    })?,
                    false,
                ),
            },
            MicCommand::Firmware { command } => match command {
                MicFirmwareCommand::Get { device } => print_reply(
                    send(Command::MicFirmwareGet {
                        protocol_version: p,
                        device_id: DeviceId(device),
                    })?,
                    true,
                ),
                MicFirmwareCommand::Stage { device, file } => print_reply(
                    send(Command::MicFirmwareStage {
                        protocol_version: p,
                        device_id: DeviceId(device),
                        path: file.to_string_lossy().into_owned(),
                    })?,
                    true,
                ),
                MicFirmwareCommand::Validate { device, staged_id } => print_reply(
                    send(Command::MicFirmwareValidate {
                        protocol_version: p,
                        device_id: DeviceId(device),
                        staged_id,
                    })?,
                    true,
                ),
                MicFirmwareCommand::Begin { device, staged_id } => print_reply(
                    send(Command::MicFirmwareBegin {
                        protocol_version: p,
                        device_id: DeviceId(device),
                        staged_id,
                    })?,
                    true,
                ),
                MicFirmwareCommand::Status { transaction_id } => print_reply(
                    send(Command::MicFirmwareStatus {
                        protocol_version: p,
                        transaction_id,
                    })?,
                    true,
                ),
                MicFirmwareCommand::Forget { staged_id } => print_reply(
                    send(Command::MicFirmwareForget {
                        protocol_version: p,
                        staged_id,
                    })?,
                    false,
                ),
            },
        },
        TopCommand::Eq { command } => match command {
            EqCommand::List => print_reply(
                send(Command::EqList {
                    protocol_version: p,
                })?,
                false,
            ),
            EqCommand::Get { name } => print_reply(
                send(Command::EqGet {
                    protocol_version: p,
                    name,
                })?,
                true,
            ),
            EqCommand::Save { file } => {
                let bytes =
                    fs::read(&file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
                let config: EqConfig = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("invalid EQ JSON {}: {e}", file.display()))?;
                config.validate().map_err(|e| e.to_string())?;
                print_reply(
                    send(Command::EqSave {
                        protocol_version: p,
                        config,
                    })?,
                    false,
                );
            }
            EqCommand::Delete { name } => print_reply(
                send(Command::EqDelete {
                    protocol_version: p,
                    name,
                })?,
                false,
            ),
            EqCommand::Apply { name } => print_reply(
                send(Command::EqApply {
                    protocol_version: p,
                    name,
                })?,
                false,
            ),
            EqCommand::Bypass => print_reply(
                send(Command::EqBypass {
                    protocol_version: p,
                })?,
                false,
            ),
        },
        TopCommand::Daemon { command } => match command {
            DaemonCommand::Status => print_reply(
                send(Command::DaemonStatus {
                    protocol_version: p,
                })?,
                false,
            ),
            DaemonCommand::Refresh => print_reply(
                send(Command::RefreshDevices {
                    protocol_version: p,
                })?,
                false,
            ),
        },
    }
    Ok(())
}

fn parse_hex_color(input: &str) -> Result<[u8; 3], String> {
    let value = input.strip_prefix('#').unwrap_or(input);
    if value.len() != 6 {
        return Err("RGB color must be RRGGBB or #RRGGBB".into());
    }
    let number =
        u32::from_str_radix(value, 16).map_err(|_| "RGB color contains invalid hex digits")?;
    Ok([
        ((number >> 16) & 0xff) as u8,
        ((number >> 8) & 0xff) as u8,
        (number & 0xff) as u8,
    ])
}

fn print_reply(reply: Reply, force_json: bool) {
    if force_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&reply).unwrap_or_else(|_| "{}".into())
        );
        return;
    }
    match reply {
        Reply::Pong { protocol_version, min_protocol_version } => println!("ForgeHX daemon protocol {protocol_version} (supports {min_protocol_version}..={protocol_version})"),
        Reply::Status { protocol_version, device_count, all_device_count, write_protected_count } => {
            println!("daemon: online (protocol {protocol_version})");
            println!("HyperX devices: {device_count}");
            println!("all devices: {all_device_count}");
            println!("write protected: {write_protected_count}");
        }
        Reply::Devices { devices } => {
            if devices.is_empty() { println!("No matching devices discovered."); }
            for device in devices {
                let usb_id = if device.vendor_id == 0 && device.product_id == 0 { "----:----".to_owned() } else { format!("{:04x}:{:04x}", device.vendor_id, device.product_id) };
                let owners = device.capability_owners.iter().filter(|o| o.writable).map(|o| format!("{:?}={}", o.capability, o.backend)).collect::<Vec<_>>().join(",");
                println!("{}\t{}\t{}\t{}\t{}\t{}\t{}", device.id, device.name, device.vendor_family, device.device_class, device.support_level, usb_id, owners);
            }
        }
        Reply::Doctor { reports } => if reports.is_empty() { println!("No matching device diagnostics."); } else { println!("{}", serde_json::to_string_pretty(&reports).unwrap()); },
        Reply::AudioNodes { nodes } => print_audio_nodes(nodes),
        Reply::BackendStatuses { backends } => {
            for backend in backends { println!("{}\t{:?}\t{}", backend.backend, backend.health, backend.detail.unwrap_or_default()); }
        }
        Reply::LightingController { metadata } => match metadata {
            Some(metadata) => println!("{}", serde_json::to_string_pretty(&metadata).unwrap()),
            None => println!("Lighting is native; no compatibility-backend metadata is required."),
        },
        Reply::MouseState { state } => println!("{}", serde_json::to_string_pretty(&state).unwrap()),
        Reply::MouseCapabilities { model } => println!("{}", serde_json::to_string_pretty(&model).unwrap()),
        Reply::KeyboardCapabilities { model } => println!("{}", serde_json::to_string_pretty(&model).unwrap()),
        Reply::EqProfiles { names } | Reply::Profiles { names } => for name in names { println!("{name}"); },
        Reply::EqProfile { config } => println!("{}", serde_json::to_string_pretty(&config).unwrap()),
        Reply::MicDspState { state } => println!("{}", serde_json::to_string_pretty(&state).unwrap()),
        Reply::OutputDspState { profile, live, generation, target_device_id } => {
            println!("Output DSP: {}", if live { "live" } else { "inactive" });
            println!("generation: {generation}");
            println!("target: {target_device_id}");
            println!("{}", serde_json::to_string_pretty(&profile).unwrap());
        }
        Reply::FirmwareIdentity { identity } => println!("{}", serde_json::to_string_pretty(&identity).unwrap()),
        Reply::FirmwarePackage { package } => println!("{}", serde_json::to_string_pretty(&package).unwrap()),
        Reply::FirmwareTransaction { status } => println!("{}", serde_json::to_string_pretty(&status).unwrap()),
        Reply::Profile { profile } => println!("{}", serde_json::to_string_pretty(&profile).unwrap()),
        Reply::Applied { skipped } => if skipped.is_empty() { println!("Profile applied to all supported sections."); } else { println!("Profile applied; skipped: {}", skipped.join(", ")); },
        Reply::Ok { message } => println!("{message}"),
        Reply::Error { code, message } => { eprintln!("ForgeHX error [{code}]: {message}"); std::process::exit(2); }
    }
}

fn print_audio_nodes(nodes: Vec<AudioNode>) {
    if nodes.is_empty() {
        println!("No PipeWire audio nodes found.");
    }
    for node in nodes {
        let volume = node
            .volume
            .map(|value| format!("{:.0}%", value * 100.0))
            .unwrap_or_else(|| "?".into());
        let mute = node
            .muted
            .map(|value| if value { "muted" } else { "active" })
            .unwrap_or("unknown");
        println!(
            "{}\t{}\t{}\t{}\t{}",
            node.id, node.kind, volume, mute, node.name
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn doctor_json_flag_parses() {
        let cli = Cli::try_parse_from(["forgehx", "doctor", "--json"]).unwrap();
        assert!(matches!(cli.command, TopCommand::Doctor { json: true, .. }));
    }
    #[test]
    fn list_all_flag_parses() {
        let cli = Cli::try_parse_from(["forgehx", "list", "--all"]).unwrap();
        assert!(matches!(cli.command, TopCommand::List { all: true }));
    }
    #[test]
    fn backend_status_parses() {
        let cli = Cli::try_parse_from(["forgehx", "backend", "status"]).unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Backend {
                command: BackendCommand::Status
            }
        ));
    }
    #[test]
    fn backend_ensure_parses() {
        let cli = Cli::try_parse_from(["forgehx", "backend", "ensure"]).unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Backend {
                command: BackendCommand::Ensure
            }
        ));
    }
    #[test]
    fn backend_restart_parses() {
        let cli = Cli::try_parse_from(["forgehx", "backend", "restart", "openrgb"]).unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Backend {
                command: BackendCommand::Restart {
                    backend: BackendArg::Openrgb
                }
            }
        ));
    }
    #[test]
    fn mouse_rate_parses() {
        let cli = Cli::try_parse_from(["forgehx", "mouse", "rate", "dev", "1000"]).unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Mouse {
                command: MouseCommand::Rate { hz: 1000, .. }
            }
        ));
    }
    #[test]
    fn mouse_capabilities_parses() {
        let cli = Cli::try_parse_from(["forgehx", "mouse", "capabilities", "dev"]).unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Mouse {
                command: MouseCommand::Capabilities { .. }
            }
        ));
    }
    #[test]
    fn mouse_lift_off_parses() {
        let cli = Cli::try_parse_from(["forgehx", "mouse", "lift-off", "dev", "2"]).unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Mouse {
                command: MouseCommand::LiftOff { mm: 2, .. }
            }
        ));
    }
    #[test]
    fn keyboard_capabilities_parses() {
        let cli = Cli::try_parse_from(["forgehx", "keyboard", "capabilities", "dev"]).unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Keyboard {
                command: KeyboardCommand::Capabilities { .. }
            }
        ));
    }
    #[test]
    fn eq_bypass_parses() {
        let cli = Cli::try_parse_from(["forgehx", "eq", "bypass"]).unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Eq {
                command: EqCommand::Bypass
            }
        ));
    }
    #[test]
    fn mic_dsp_get_parses() {
        let cli = Cli::try_parse_from(["forgehx", "mic", "dsp", "get", "mic-1"]).unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Mic {
                command: MicCommand::Dsp {
                    command: MicDspCommand::Get { .. }
                }
            }
        ));
    }
    #[test]
    fn mic_dsp_apply_parses() {
        let cli =
            Cli::try_parse_from(["forgehx", "mic", "dsp", "apply", "mic-1", "Broadcast"]).unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Mic {
                command: MicCommand::Dsp {
                    command: MicDspCommand::Apply { .. }
                }
            }
        ));
    }
    #[test]
    fn mic_dsp_bypass_parses() {
        let cli = Cli::try_parse_from(["forgehx", "mic", "dsp", "bypass", "mic-1"]).unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Mic {
                command: MicCommand::Dsp {
                    command: MicDspCommand::Bypass { .. }
                }
            }
        ));
    }
    #[test]
    fn mic_firmware_get_parses() {
        let cli = Cli::try_parse_from(["forgehx", "mic", "firmware", "get", "mic-1"]).unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Mic {
                command: MicCommand::Firmware {
                    command: MicFirmwareCommand::Get { .. }
                }
            }
        ));
    }
    #[test]
    fn mic_firmware_stage_parses() {
        let cli = Cli::try_parse_from([
            "forgehx",
            "mic",
            "firmware",
            "stage",
            "mic-1",
            "firmware.bin",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Mic {
                command: MicCommand::Firmware {
                    command: MicFirmwareCommand::Stage { .. }
                }
            }
        ));
    }
    #[test]
    fn mic_firmware_validate_parses() {
        let cli = Cli::try_parse_from(["forgehx", "mic", "firmware", "validate", "mic-1", "fw-1"])
            .unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Mic {
                command: MicCommand::Firmware {
                    command: MicFirmwareCommand::Validate { .. }
                }
            }
        ));
    }
    #[test]
    fn mic_firmware_begin_parses() {
        let cli =
            Cli::try_parse_from(["forgehx", "mic", "firmware", "begin", "mic-1", "fw-1"]).unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Mic {
                command: MicCommand::Firmware {
                    command: MicFirmwareCommand::Begin { .. }
                }
            }
        ));
    }
    #[test]
    fn mic_firmware_status_parses() {
        let cli = Cli::try_parse_from(["forgehx", "mic", "firmware", "status", "tx-1"]).unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Mic {
                command: MicCommand::Firmware {
                    command: MicFirmwareCommand::Status { .. }
                }
            }
        ));
    }
    #[test]
    fn mic_firmware_forget_parses() {
        let cli = Cli::try_parse_from(["forgehx", "mic", "firmware", "forget", "fw-1"]).unwrap();
        assert!(matches!(
            cli.command,
            TopCommand::Mic {
                command: MicCommand::Firmware {
                    command: MicFirmwareCommand::Forget { .. }
                }
            }
        ));
    }
    #[test]
    fn parses_rgb_hex() {
        assert_eq!(parse_hex_color("#12abef").unwrap(), [0x12, 0xab, 0xef]);
        assert!(parse_hex_color("bad").is_err());
    }
}
