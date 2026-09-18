use std::fmt;
use std::io;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub struct AudioNode {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioGraph {
    pub devices: Vec<AudioNode>,
    pub sources: Vec<AudioNode>,
    pub sinks: Vec<AudioNode>,
    pub raw_status: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeState {
    pub volume: f32,
    pub muted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PulseProfile {
    pub name: String,
    pub sinks: u32,
    pub sources: u32,
    pub available: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PulseCard {
    pub name: String,
    pub profiles: Vec<PulseProfile>,
    pub active_profile: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeacnAudioHealth {
    UsbMissing,
    PipeWireCardMissing,
    NodesMissing,
    SourceMissing,
    SinkMissing,
    Healthy,
}

impl BeacnAudioHealth {
    pub const fn is_healthy(self) -> bool {
        matches!(self, Self::Healthy)
    }
}

impl fmt::Display for BeacnAudioHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UsbMissing => "BEACN USB device missing",
            Self::PipeWireCardMissing => "BEACN PipeWire card missing",
            Self::NodesMissing => "BEACN audio nodes missing",
            Self::SourceMissing => "BEACN microphone source missing",
            Self::SinkMissing => "BEACN headphone/output sink missing",
            Self::Healthy => "BEACN PipeWire audio healthy",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    None,
    Devices,
    Sources,
    Sinks,
}

pub fn clamp_volume(value: f32) -> f32 {
    value.clamp(0.0, 1.5)
}

pub fn status() -> io::Result<AudioGraph> {
    let text = run_wpctl(["status", "-n"])?;
    Ok(parse_status(&text))
}

pub fn parse_status(text: &str) -> AudioGraph {
    let mut graph = AudioGraph {
        raw_status: text.to_owned(),
        ..AudioGraph::default()
    };
    let mut section = Section::None;

    for line in text.lines() {
        if line.contains("Devices:") {
            section = Section::Devices;
            continue;
        }
        if line.contains("Sinks:") {
            section = Section::Sinks;
            continue;
        }
        if line.contains("Sources:") {
            section = Section::Sources;
            continue;
        }
        if line.contains("Filters:")
            || line.contains("Streams:")
            || line.contains("Video")
            || line.contains("Settings")
        {
            section = Section::None;
            continue;
        }

        let Some(node) = parse_node_line(line) else {
            continue;
        };
        match section {
            Section::Devices => graph.devices.push(node),
            Section::Sources => graph.sources.push(node),
            Section::Sinks => graph.sinks.push(node),
            Section::None => {}
        }
    }

    graph
}

fn parse_node_line(line: &str) -> Option<AudioNode> {
    let bytes = line.as_bytes();
    let start = bytes.iter().position(u8::is_ascii_digit)?;
    let rest = &line[start..];
    let dot = rest.find('.')?;
    let id = rest[..dot].trim();
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }

    let after_dot = rest[dot + 1..].trim();
    let name = after_dot
        .split_once(" [")
        .map_or(after_dot, |(value, _)| value)
        .trim();
    if name.is_empty() {
        return None;
    }

    Some(AudioNode {
        id: id.to_owned(),
        name: name.to_owned(),
        is_default: line[..start].contains('*'),
    })
}

pub fn selected_by_name<'a>(nodes: &'a [AudioNode], name: Option<&str>) -> Option<&'a AudioNode> {
    let name = name?;
    nodes.iter().find(|node| node.name == name)
}

pub fn reselect_node(
    nodes: &[AudioNode],
    current_name: Option<&str>,
    require_beacn_when_unselected: bool,
) -> Option<String> {
    if let Some(name) = current_name {
        return Some(name.to_owned());
    }

    let beacn = nodes.iter().find(|node| is_beacn_node(node));
    if require_beacn_when_unselected {
        return beacn.map(|node| node.name.clone());
    }

    beacn
        .or_else(|| nodes.iter().find(|node| node.is_default))
        .or_else(|| nodes.first())
        .map(|node| node.name.clone())
}

pub fn is_beacn_node(node: &AudioNode) -> bool {
    let lower = node.name.to_ascii_lowercase();
    lower.contains("beacn")
        && !lower.contains("aetherforge_beacn_private_dsp")
        && !lower.contains("aetherforge beacn processed")
}

pub fn is_private_beacn_node(node: &AudioNode) -> bool {
    let lower = node.name.to_ascii_lowercase();
    lower.contains("aetherforge_beacn_private_dsp") || lower.contains("aetherforge beacn processed")
}

pub fn beacn_audio_health(graph: &AudioGraph, usb_present: bool) -> BeacnAudioHealth {
    if !usb_present {
        return BeacnAudioHealth::UsbMissing;
    }

    let has_device = graph.devices.iter().any(is_beacn_node);
    let has_source = graph.sources.iter().any(is_beacn_node);
    let has_sink = graph.sinks.iter().any(is_beacn_node);

    match (has_device, has_source, has_sink) {
        (_, true, true) => BeacnAudioHealth::Healthy,
        (false, false, false) => BeacnAudioHealth::PipeWireCardMissing,
        (true, false, false) => BeacnAudioHealth::NodesMissing,
        (_, false, true) => BeacnAudioHealth::SourceMissing,
        (_, true, false) => BeacnAudioHealth::SinkMissing,
    }
}

pub fn parse_pactl_cards(text: &str) -> Vec<PulseCard> {
    let mut cards = Vec::new();
    let mut current: Option<PulseCard> = None;
    let mut in_profiles = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Card #") {
            if let Some(card) = current.take()
                && !card.name.is_empty()
            {
                cards.push(card);
            }
            current = Some(PulseCard::default());
            in_profiles = false;
            continue;
        }

        let Some(card) = current.as_mut() else {
            continue;
        };

        if let Some(name) = trimmed.strip_prefix("Name: ") {
            card.name = name.trim().to_owned();
            continue;
        }
        if trimmed == "Profiles:" {
            in_profiles = true;
            continue;
        }
        if let Some(active) = trimmed.strip_prefix("Active Profile: ") {
            card.active_profile = Some(active.trim().to_owned());
            in_profiles = false;
            continue;
        }
        if trimmed == "Ports:" {
            in_profiles = false;
            continue;
        }

        if in_profiles && let Some((name, details)) = trimmed.split_once(": ") {
            let sinks = parse_profile_count(details, "sinks:");
            let sources = parse_profile_count(details, "sources:");
            let available = !details.contains("available: no");
            card.profiles.push(PulseProfile {
                name: name.trim().to_owned(),
                sinks,
                sources,
                available,
            });
        }
    }

    if let Some(card) = current
        && !card.name.is_empty()
    {
        cards.push(card);
    }
    cards
}

fn parse_profile_count(details: &str, label: &str) -> u32 {
    let Some((_, rest)) = details.split_once(label) else {
        return 0;
    };
    rest.trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

pub fn is_beacn_card_name(name: &str) -> bool {
    name.to_ascii_lowercase().contains("beacn")
}

pub fn preferred_output_profile(card: &PulseCard) -> Option<&PulseProfile> {
    let active_duplex = card.active_profile.as_deref().and_then(|active| {
        card.profiles.iter().find(|profile| {
            profile.name == active && profile.available && profile.sinks > 0 && profile.sources > 0
        })
    });

    active_duplex
        .or_else(|| {
            card.profiles
                .iter()
                .find(|profile| profile.available && profile.sinks > 0 && profile.sources > 0)
        })
        .or_else(|| {
            card.profiles
                .iter()
                .find(|profile| profile.available && profile.sinks > 0)
        })
}

pub fn pulse_cards() -> io::Result<Vec<PulseCard>> {
    let text = run_command("pactl", ["list", "cards"])?;
    Ok(parse_pactl_cards(&text))
}

pub fn recover_beacn_output_profile() -> io::Result<Option<String>> {
    let cards = pulse_cards()?;
    let Some(card) = cards.iter().find(|card| is_beacn_card_name(&card.name)) else {
        return Ok(None);
    };
    let Some(profile) = preferred_output_profile(card) else {
        return Ok(None);
    };

    run_command(
        "pactl",
        [
            "set-card-profile",
            card.name.as_str(),
            profile.name.as_str(),
        ],
    )?;
    Ok(Some(profile.name.clone()))
}

pub fn repair_beacn_output_profile_only(
    timeout: Duration,
) -> io::Result<Option<(String, AudioGraph)>> {
    let Some(profile) = recover_beacn_output_profile()? else {
        return Ok(None);
    };
    let deadline = Instant::now() + timeout;

    loop {
        if let Ok(graph) = status()
            && beacn_audio_health(&graph, true).is_healthy()
        {
            return Ok(Some((profile, graph)));
        }
        if Instant::now() >= deadline {
            return Err(io::Error::other(
                "BEACN output profile changed but audio nodes did not become healthy in time",
            ));
        }
        thread::sleep(Duration::from_millis(150));
    }
}

pub fn get_volume(target: &str) -> io::Result<VolumeState> {
    let text = run_wpctl(["get-volume", target])?;
    let volume = text
        .split_whitespace()
        .find_map(|token| token.parse::<f32>().ok())
        .unwrap_or(0.0);
    Ok(VolumeState {
        volume: clamp_volume(volume),
        muted: text.contains("MUTED"),
    })
}

pub fn set_volume(target: &str, volume: f32) -> io::Result<()> {
    let value = format!("{:.3}", clamp_volume(volume));
    run_wpctl(["set-volume", target, &value])?;
    Ok(())
}

pub fn set_mute(target: &str, muted: bool) -> io::Result<()> {
    let value = if muted { "1" } else { "0" };
    run_wpctl(["set-mute", target, value])?;
    Ok(())
}

pub fn recover_beacn_nodes(timeout: Duration) -> io::Result<AudioGraph> {
    let deadline = Instant::now() + timeout;

    // If the ALSA/PipeWire card survived but its playback node disappeared, first restore an
    // output-capable card profile. This does not alter the system default source or sink.
    if let Ok(Some((_, graph))) = repair_beacn_output_profile_only(Duration::from_secs(1)) {
        return Ok(graph);
    }

    restart_wireplumber()?;
    thread::sleep(Duration::from_millis(300));
    let _ = recover_beacn_output_profile();

    match wait_for_beacn_nodes(deadline)? {
        Some(graph) => Ok(graph),
        None => Err(io::Error::other("BEACN audio recovery timed out")),
    }
}

fn wait_for_beacn_nodes(deadline: Instant) -> io::Result<Option<AudioGraph>> {
    loop {
        if let Ok(graph) = status()
            && beacn_audio_health(&graph, true).is_healthy()
        {
            return Ok(Some(graph));
        }
        if Instant::now() >= deadline {
            return Ok(None);
        }
        thread::sleep(Duration::from_millis(150));
    }
}

fn restart_wireplumber() -> io::Result<()> {
    run_command("systemctl", ["--user", "restart", "wireplumber.service"])?;
    Ok(())
}

fn run_wpctl<const N: usize>(args: [&str; N]) -> io::Result<String> {
    run_command("wpctl", args)
}

fn run_command<const N: usize>(program: &str, args: [&str; N]) -> io::Result<String> {
    let output = Command::new(program).args(args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        return Err(io::Error::other(if detail.is_empty() {
            format!("{program} exited with {}", output.status)
        } else {
            detail
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
