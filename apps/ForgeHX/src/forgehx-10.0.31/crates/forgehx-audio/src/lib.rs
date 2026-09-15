use forgehx_core::{AudioNode, ForgeHxError};
use std::process::Command;

pub trait AudioBackend: Send + Sync {
    fn discover(&self) -> Result<Vec<AudioNode>, ForgeHxError>;
    fn set_volume(&self, node_id: u32, volume: f32) -> Result<(), ForgeHxError>;
    fn set_mute(&self, node_id: u32, muted: bool) -> Result<(), ForgeHxError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WpctlBackend;

impl AudioBackend for WpctlBackend {
    fn discover(&self) -> Result<Vec<AudioNode>, ForgeHxError> {
        let output = Command::new("wpctl")
            .args(["status", "-n"])
            .output()
            .or_else(|_| Command::new("wpctl").arg("status").output())
            .map_err(|e| ForgeHxError::AudioUnavailable(format!("wpctl is not available: {e}")))?;
        if !output.status.success() {
            return Err(ForgeHxError::AudioUnavailable(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ));
        }
        let wpctl_nodes = parse_wpctl_status(&String::from_utf8_lossy(&output.stdout));
        let registry_nodes = Command::new("pw-cli")
            .args(["ls", "Node"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| parse_pw_cli_audio_nodes(&String::from_utf8_lossy(&output.stdout)))
            .unwrap_or_default();
        Ok(merge_audio_nodes(wpctl_nodes, registry_nodes))
    }

    fn set_volume(&self, node_id: u32, volume: f32) -> Result<(), ForgeHxError> {
        let args = volume_args(node_id, volume);
        run_wpctl(&args)
    }

    fn set_mute(&self, node_id: u32, muted: bool) -> Result<(), ForgeHxError> {
        let id = node_id.to_string();
        let muted = if muted { "1" } else { "0" };
        run_wpctl(&["set-mute".into(), id, muted.into()])
    }
}

pub fn discover_audio_nodes() -> Result<Vec<AudioNode>, ForgeHxError> {
    WpctlBackend.discover()
}

/// Kept for v0.1 source compatibility. v0.2 no longer filters by brand.
pub fn discover_hyperx_nodes() -> Result<Vec<AudioNode>, ForgeHxError> {
    discover_audio_nodes()
}

fn run_wpctl(args: &[String]) -> Result<(), ForgeHxError> {
    let output = Command::new("wpctl")
        .args(args)
        .output()
        .map_err(|e| ForgeHxError::AudioUnavailable(format!("wpctl is not available: {e}")))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(ForgeHxError::AudioUnavailable(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ))
    }
}

pub fn parse_pw_cli_audio_nodes(input: &str) -> Vec<AudioNode> {
    #[derive(Default)]
    struct PendingNode {
        id: Option<u32>,
        node_name: Option<String>,
        description: Option<String>,
        media_class: Option<String>,
    }

    fn finish(pending: &mut PendingNode, nodes: &mut Vec<AudioNode>) {
        let Some(id) = pending.id.take() else {
            *pending = PendingNode::default();
            return;
        };
        let kind = match pending.media_class.as_deref() {
            Some("Audio/Source") => "source",
            Some("Audio/Sink") => "sink",
            _ => {
                *pending = PendingNode::default();
                return;
            }
        };
        let name = pending
            .node_name
            .take()
            .or_else(|| pending.description.take())
            .unwrap_or_default();
        if !name.is_empty() {
            nodes.push(AudioNode {
                id,
                name,
                kind: kind.to_owned(),
                volume: None,
                muted: None,
            });
        }
        *pending = PendingNode::default();
    }

    fn property_value(line: &str, key: &str) -> Option<String> {
        let value = line.strip_prefix(key)?.trim().strip_prefix('=')?.trim();
        Some(value.trim_matches('"').to_owned())
    }

    let mut nodes = Vec::new();
    let mut pending = PendingNode::default();
    for raw_line in input.lines() {
        let line = raw_line.trim();
        if let Some(rest) = line.strip_prefix("id ") {
            finish(&mut pending, &mut nodes);
            pending.id = rest
                .split(',')
                .next()
                .and_then(|value| value.trim().parse::<u32>().ok());
            continue;
        }
        if let Some(value) = property_value(line, "node.name") {
            pending.node_name = Some(value);
        } else if let Some(value) = property_value(line, "node.description") {
            pending.description = Some(value);
        } else if let Some(value) = property_value(line, "media.class") {
            pending.media_class = Some(value);
        }
    }
    finish(&mut pending, &mut nodes);
    nodes.sort_by_key(|node| node.id);
    nodes.dedup_by_key(|node| node.id);
    nodes
}

pub fn merge_audio_nodes(
    mut preferred: Vec<AudioNode>,
    fallback: Vec<AudioNode>,
) -> Vec<AudioNode> {
    for node in fallback {
        if !preferred.iter().any(|existing| existing.id == node.id) {
            preferred.push(node);
        }
    }
    preferred.sort_by_key(|node| node.id);
    preferred.dedup_by_key(|node| node.id);
    preferred
}

pub fn parse_wpctl_status(input: &str) -> Vec<AudioNode> {
    let mut kind: Option<&str> = None;
    let mut nodes = Vec::new();

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.contains("Sinks:") {
            kind = Some("sink");
            continue;
        }
        if line.contains("Sources:") {
            kind = Some("source");
            continue;
        }
        if line.ends_with(':') && !line.contains("Sinks:") && !line.contains("Sources:") {
            kind = None;
            continue;
        }
        let Some(node_kind) = kind else { continue };
        let Some((id, name, volume, muted)) = parse_node_line(line) else {
            continue;
        };
        nodes.push(AudioNode {
            id,
            name,
            kind: node_kind.to_owned(),
            volume,
            muted,
        });
    }

    nodes.sort_by_key(|node| node.id);
    nodes.dedup_by_key(|node| node.id);
    nodes
}

fn parse_node_line(line: &str) -> Option<(u32, String, Option<f32>, Option<bool>)> {
    let digit_start = line.char_indices().find(|(_, c)| c.is_ascii_digit())?.0;
    let tail = &line[digit_start..];
    let dot = tail.find('.')?;
    let id = tail[..dot].trim().parse().ok()?;
    let body = tail[dot + 1..].trim();
    let name = body
        .split("[vol:")
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned();
    if name.is_empty() {
        return None;
    }
    let volume = body
        .split("[vol:")
        .nth(1)
        .and_then(|value| {
            value
                .split(|ch: char| ch == ']' || ch.is_whitespace())
                .next()
        })
        .and_then(|value| value.parse::<f32>().ok());
    let muted = body
        .contains("MUTED")
        .then_some(true)
        .or_else(|| volume.map(|_| false));
    Some((id, name, volume, muted))
}

pub fn volume_args(node_id: u32, volume: f32) -> Vec<String> {
    let volume = volume.clamp(0.0, 1.5);
    vec![
        "set-volume".into(),
        node_id.to_string(),
        format!("{volume:.3}"),
    ]
}

#[derive(Debug, Clone)]
pub struct EqManager {
    config_home: std::path::PathBuf,
}

impl Default for EqManager {
    fn default() -> Self {
        let root = std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config"))
            })
            .unwrap_or_else(|| std::path::PathBuf::from(".config"));
        Self::new(root)
    }
}

impl EqManager {
    pub fn new(config_home: std::path::PathBuf) -> Self {
        Self { config_home }
    }

    fn eq_dir(&self) -> std::path::PathBuf {
        self.config_home.join("forgehx/eq")
    }
    fn profile_json(&self, name: &str) -> Result<std::path::PathBuf, ForgeHxError> {
        validate_eq_name(name)?;
        Ok(self.eq_dir().join(format!("{name}.json")))
    }
    fn profile_txt(&self, name: &str) -> Result<std::path::PathBuf, ForgeHxError> {
        validate_eq_name(name)?;
        Ok(self.eq_dir().join(format!("{name}.txt")))
    }
    fn pipewire_dropin(&self) -> std::path::PathBuf {
        self.config_home
            .join("pipewire/pipewire.conf.d/90-forgehx-eq.conf")
    }

    pub fn list(&self) -> Result<Vec<String>, ForgeHxError> {
        let dir = self.eq_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut names = std::fs::read_dir(&dir)
            .map_err(io_error)?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                (path.extension().and_then(|s| s.to_str()) == Some("json"))
                    .then(|| path.file_stem()?.to_str().map(str::to_owned))
                    .flatten()
            })
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        Ok(names)
    }

    pub fn get(&self, name: &str) -> Result<forgehx_core::EqConfig, ForgeHxError> {
        let path = self.profile_json(name)?;
        let bytes = std::fs::read(&path).map_err(io_error)?;
        let config: forgehx_core::EqConfig = serde_json::from_slice(&bytes)
            .map_err(|e| ForgeHxError::InvalidProfile(format!("{}: {e}", path.display())))?;
        config.validate()?;
        Ok(config)
    }

    pub fn save(&self, config: &forgehx_core::EqConfig) -> Result<(), ForgeHxError> {
        config.validate()?;
        validate_eq_name(&config.name)?;
        std::fs::create_dir_all(self.eq_dir()).map_err(io_error)?;
        atomic_write(
            &self.profile_json(&config.name)?,
            &serde_json::to_vec_pretty(config)
                .map_err(|e| ForgeHxError::InvalidProfile(e.to_string()))?,
        )?;
        atomic_write(
            &self.profile_txt(&config.name)?,
            render_eq_text(config).as_bytes(),
        )?;
        Ok(())
    }

    pub fn delete(&self, name: &str) -> Result<(), ForgeHxError> {
        for path in [self.profile_json(name)?, self.profile_txt(name)?] {
            if path.exists() {
                std::fs::remove_file(path).map_err(io_error)?;
            }
        }
        Ok(())
    }

    pub fn apply(&self, name: &str) -> Result<(), ForgeHxError> {
        let config = self.get(name)?;
        let txt = self.profile_txt(name)?;
        if !txt.exists() {
            atomic_write(&txt, render_eq_text(&config).as_bytes())?;
        }
        let dropin = self.pipewire_dropin();
        if let Some(parent) = dropin.parent() {
            std::fs::create_dir_all(parent).map_err(io_error)?;
        }
        let target = config.target_node_id.map(resolve_node_target).transpose()?;
        atomic_write(
            &dropin,
            render_pipewire_config(&config, &txt, target.as_ref()).as_bytes(),
        )?;
        restart_pipewire()
    }

    pub fn bypass(&self) -> Result<(), ForgeHxError> {
        let dropin = self.pipewire_dropin();
        if dropin.exists() {
            std::fs::remove_file(dropin).map_err(io_error)?;
        }
        restart_pipewire()
    }
}

fn validate_eq_name(name: &str) -> Result<(), ForgeHxError> {
    if name.trim().is_empty()
        || name.len() > 96
        || name.contains('/')
        || name.contains('\\')
        || name == "."
        || name == ".."
    {
        Err(ForgeHxError::InvalidProfile(
            "invalid EQ profile name".into(),
        ))
    } else {
        Ok(())
    }
}

fn io_error(error: std::io::Error) -> ForgeHxError {
    ForgeHxError::Io(error.to_string())
}
fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> Result<(), ForgeHxError> {
    let parent = path
        .parent()
        .ok_or_else(|| ForgeHxError::Io("EQ path has no parent".into()))?;
    std::fs::create_dir_all(parent).map_err(io_error)?;
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|s| s.to_str()).unwrap_or("file")
    ));
    std::fs::write(&tmp, bytes).map_err(io_error)?;
    std::fs::rename(tmp, path).map_err(io_error)
}

pub fn render_eq_text(config: &forgehx_core::EqConfig) -> String {
    use forgehx_core::EqFilterKind;
    let mut out = format!("Preamp: {:.2} dB\n", config.preamp_db);
    for (index, band) in config
        .filters
        .iter()
        .filter(|band| band.enabled)
        .enumerate()
    {
        let kind = match band.kind {
            EqFilterKind::Peaking => "PK",
            EqFilterKind::LowShelf => "LSC",
            EqFilterKind::HighShelf => "HSC",
        };
        out.push_str(&format!(
            "Filter {}: ON {} Fc {:.2} Hz Gain {:.2} dB Q {:.3}\n",
            index + 1,
            kind,
            band.frequency_hz,
            band.gain_db,
            band.q
        ));
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PipeWireTarget {
    NodeName(String),
    ObjectSerial(u64),
}

impl PipeWireTarget {
    fn config_value(&self) -> String {
        match self {
            Self::NodeName(name) => format!("\"{}\"", escape_spa(name)),
            Self::ObjectSerial(serial) => serial.to_string(),
        }
    }
}

fn render_pipewire_config(
    config: &forgehx_core::EqConfig,
    txt: &std::path::Path,
    target_object: Option<&PipeWireTarget>,
) -> String {
    let channel_map = config.channel_map.join(" ");
    let target = target_object
        .map(|target| {
            format!(
                "        target.object = {}\n        node.dont-reconnect = true\n",
                target.config_value()
            )
        })
        .unwrap_or_default();
    format!(
        r#"# Managed by ForgeHX. Delete or use `forgehx eq bypass` to disable.
context.modules = [
  {{ name = libpipewire-module-parametric-equalizer
    args = {{
      equalizer.filepath = "{}"
      equalizer.description = "ForgeHX EQ - {}"
      audio.channels = {}
      audio.position = [ {} ]
      capture.props = {{
        node.name = "forgehx_eq_input"
        media.class = "Audio/Sink"
      }}
      playback.props = {{
        node.name = "forgehx_eq_output"
        node.passive = true
{}      }}
    }}
  }}
]
"#,
        escape_spa(&txt.to_string_lossy()),
        escape_spa(&config.name),
        config.channels,
        channel_map,
        target
    )
}

fn resolve_node_target(node_id: u32) -> Result<PipeWireTarget, ForgeHxError> {
    let output = Command::new("wpctl")
        .args(["inspect", &node_id.to_string()])
        .output()
        .map_err(|e| {
            ForgeHxError::AudioUnavailable(format!("wpctl inspect is not available: {e}"))
        })?;
    if !output.status.success() {
        return Err(ForgeHxError::AudioUnavailable(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    parse_pipewire_target(&String::from_utf8_lossy(&output.stdout)).ok_or_else(||ForgeHxError::AudioUnavailable(format!("PipeWire node {node_id} has neither node.name nor object.serial; EQ routing for this node is unavailable")))
}

fn parse_pipewire_property(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{key} =");
    text.lines().find_map(|line| {
        let line = line.trim().trim_start_matches('*').trim();
        let value = line
            .strip_prefix(&prefix)?
            .trim()
            .trim_matches('"')
            .trim()
            .to_owned();
        (!value.is_empty()).then_some(value)
    })
}

fn parse_pipewire_target(text: &str) -> Option<PipeWireTarget> {
    if let Some(name) = parse_pipewire_property(text, "node.name") {
        return Some(PipeWireTarget::NodeName(name));
    }
    parse_pipewire_property(text, "object.serial")
        .and_then(|serial| serial.parse::<u64>().ok())
        .map(PipeWireTarget::ObjectSerial)
}

pub fn parse_node_name(text: &str) -> Option<String> {
    match parse_pipewire_target(text) {
        Some(PipeWireTarget::NodeName(name)) => Some(name),
        _ => None,
    }
}

fn escape_spa(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
fn restart_pipewire() -> Result<(), ForgeHxError> {
    let output = Command::new("systemctl")
        .args(["--user", "restart", "pipewire.service"])
        .output()
        .map_err(|e| ForgeHxError::AudioUnavailable(format!("systemctl is not available: {e}")))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(ForgeHxError::AudioUnavailable(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_keeps_hyperx_and_non_hyperx_audio_nodes() {
        let input = r#"
Audio
 ├─ Sinks:
 │  *   54. HyperX Cloud III Analog Stereo [vol: 0.75]
 │      61. Logitech G Pro X Wireless [vol: 0.50 MUTED]
 │      62. Built-in Audio Analog Stereo [vol: 1.00]
 ├─ Sources:
 │      73. HyperX QuadCast S Mono [vol: 1.00]
Video
 ├─ Devices:
 │      90. Webcam
"#;
        let nodes = parse_wpctl_status(input);
        assert_eq!(nodes.len(), 4);
        assert_eq!(nodes[0].id, 54);
        assert_eq!(nodes[1].id, 61);
        assert_eq!(nodes[1].muted, Some(true));
        assert_eq!(nodes[2].id, 62);
        assert_eq!(nodes[3].id, 73);
    }

    #[test]
    fn volume_command_uses_normalized_value() {
        assert_eq!(volume_args(54, 0.42), vec!["set-volume", "54", "0.420"]);
        assert_eq!(volume_args(54, 9.0), vec!["set-volume", "54", "1.500"]);
    }

    #[test]
    fn eq_renderer_uses_pipewire_parametric_tokens() {
        let config = forgehx_core::EqConfig {
            name: "Gaming".into(),
            preamp_db: -3.0,
            filters: vec![forgehx_core::ParametricEqBand {
                enabled: true,
                kind: forgehx_core::EqFilterKind::LowShelf,
                frequency_hz: 80.0,
                gain_db: 2.5,
                q: 0.7,
            }],
            ..Default::default()
        };
        let text = render_eq_text(&config);
        assert!(text.contains("Preamp: -3.00 dB"));
        assert!(text.contains("ON LSC Fc 80.00 Hz Gain 2.50 dB Q 0.700"));
    }

    #[test]
    fn parses_stable_pipewire_node_name() {
        assert_eq!(
            parse_node_name("  node.name = \"alsa_output.usb-HyperX-00.analog-stereo\"\n"),
            Some("alsa_output.usb-HyperX-00.analog-stereo".into())
        );
    }

    #[test]
    fn falls_back_to_pipewire_object_serial_when_node_name_is_missing() {
        assert_eq!(
            parse_pipewire_target("  object.serial = \"734\"\n"),
            Some(PipeWireTarget::ObjectSerial(734))
        );
        assert_eq!(
            parse_pipewire_target("* object.serial = 735\n"),
            Some(PipeWireTarget::ObjectSerial(735))
        );
    }

    #[test]
    fn pipewire_target_prefers_node_name_over_serial() {
        assert_eq!(parse_pipewire_target("  object.serial = 734\n  node.name = \"alsa_output.usb-HyperX-00.analog-stereo\"\n"), Some(PipeWireTarget::NodeName("alsa_output.usb-HyperX-00.analog-stereo".into())));
    }

    #[test]
    fn eq_delete_removes_managed_profile_files() {
        let root =
            std::env::temp_dir().join(format!("forgehx-eq-delete-test-{}", std::process::id()));
        let manager = EqManager::new(root.clone());
        let config = forgehx_core::EqConfig {
            name: "DeleteMe".into(),
            ..Default::default()
        };
        manager.save(&config).unwrap();
        manager.delete("DeleteMe").unwrap();
        assert!(!root.join("forgehx/eq/DeleteMe.json").exists());
        assert!(!root.join("forgehx/eq/DeleteMe.txt").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn eq_manager_confines_files_to_its_config_root() {
        let root = std::env::temp_dir().join(format!("forgehx-eq-test-{}", std::process::id()));
        let manager = EqManager::new(root.clone());
        let config = forgehx_core::EqConfig {
            name: "Safe".into(),
            ..Default::default()
        };
        manager.save(&config).unwrap();
        assert!(root.join("forgehx/eq/Safe.json").exists());
        assert!(manager
            .save(&forgehx_core::EqConfig {
                name: "../escape".into(),
                ..Default::default()
            })
            .is_err());
        let _ = std::fs::remove_dir_all(root);
    }
}

#[cfg(test)]
mod pipewire_registry_discovery_tests {
    use super::{merge_audio_nodes, parse_pw_cli_audio_nodes, AudioNode};

    #[test]
    fn registry_parser_recovers_physical_source_missing_from_wpctl_view() {
        let text = r#"
id 371683, type PipeWire:Interface:Node/3
    node.name = "alsa_input.usb-HP__Inc_HyperX_SoloCast_2_SERIAL-00.analog-stereo"
    node.description = "HyperX SoloCast 2 Analog Stereo"
    media.class = "Audio/Source"
id 413543, type PipeWire:Interface:Node/3
    node.name = "forgehx_mic_a20cca526b23023a"
    media.class = "Audio/Source"
"#;
        let nodes = parse_pw_cli_audio_nodes(text);
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].id, 371683);
        assert_eq!(nodes[0].kind, "source");
        assert!(nodes[0].name.contains("HyperX_SoloCast_2"));
    }

    #[test]
    fn wpctl_entry_wins_when_registry_has_same_node_id() {
        let wpctl = vec![AudioNode {
            id: 81,
            name: "HyperX SoloCast 2 Analog Stereo".into(),
            kind: "source".into(),
            volume: Some(0.75),
            muted: Some(false),
        }];
        let registry = vec![AudioNode {
            id: 81,
            name: "alsa_input.usb-HP__Inc_HyperX_SoloCast_2_SERIAL-00.analog-stereo".into(),
            kind: "source".into(),
            volume: None,
            muted: None,
        }];
        let merged = merge_audio_nodes(wpctl, registry);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].volume, Some(0.75));
        assert_eq!(merged[0].name, "HyperX SoloCast 2 Analog Stereo");
    }
}
