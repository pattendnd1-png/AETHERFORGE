use crate::hardware::{EqBandKind, HeadphonePower, NoiseStyle, ProcessorMode};
use crate::software_dsp::{HEADPHONE_EQ_BAND_COUNT, MIC_EQ_BAND_COUNT, SoftwareDspState};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const PROFILE_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub name: String,
    pub source_name: String,
    pub source_volume: f32,
    pub source_muted: bool,
    pub sink_name: String,
    pub sink_volume: f32,
    pub sink_muted: bool,
    pub dsp: SoftwareDspState,
}

pub fn encode(profile: &Profile) -> String {
    let mut out = String::new();
    push(&mut out, "schema", PROFILE_SCHEMA_VERSION);
    push(&mut out, "name", clean_value(&profile.name));
    push(&mut out, "source_name", clean_value(&profile.source_name));
    push(
        &mut out,
        "source_volume",
        format!("{:.3}", profile.source_volume),
    );
    push(&mut out, "source_muted", profile.source_muted);
    push(&mut out, "sink_name", clean_value(&profile.sink_name));
    push(
        &mut out,
        "sink_volume",
        format!("{:.3}", profile.sink_volume),
    );
    push(&mut out, "sink_muted", profile.sink_muted);

    let dsp = &profile.dsp;
    push(&mut out, "dsp.mic_gain_db", dsp.mic_gain_db);
    push(
        &mut out,
        "dsp.mic_eq_mode",
        processor_mode_name(dsp.mic_eq_mode),
    );
    for (index, band) in dsp.mic_eq.iter().enumerate() {
        encode_band(&mut out, &format!("dsp.mic_eq.{index}"), band);
    }
    push(
        &mut out,
        "dsp.compressor.mode",
        processor_mode_name(dsp.compressor.mode),
    );
    push(&mut out, "dsp.compressor.enabled", dsp.compressor.enabled);
    push(
        &mut out,
        "dsp.compressor.threshold_db",
        dsp.compressor.threshold_db,
    );
    push(&mut out, "dsp.compressor.ratio", dsp.compressor.ratio);
    push(
        &mut out,
        "dsp.compressor.attack_ms",
        dsp.compressor.attack_ms,
    );
    push(
        &mut out,
        "dsp.compressor.release_ms",
        dsp.compressor.release_ms,
    );
    push(
        &mut out,
        "dsp.compressor.makeup_gain_db",
        dsp.compressor.makeup_gain_db,
    );
    push(
        &mut out,
        "dsp.expander.mode",
        processor_mode_name(dsp.expander.mode),
    );
    push(&mut out, "dsp.expander.enabled", dsp.expander.enabled);
    push(
        &mut out,
        "dsp.expander.threshold_db",
        dsp.expander.threshold_db,
    );
    push(&mut out, "dsp.expander.ratio", dsp.expander.ratio);
    push(&mut out, "dsp.expander.attack_ms", dsp.expander.attack_ms);
    push(&mut out, "dsp.expander.release_ms", dsp.expander.release_ms);
    push(&mut out, "dsp.suppressor.enabled", dsp.suppressor.enabled);
    push(&mut out, "dsp.suppressor.amount", dsp.suppressor.amount);
    push(
        &mut out,
        "dsp.suppressor.style",
        noise_style_name(dsp.suppressor.style),
    );
    push(
        &mut out,
        "dsp.suppressor.sensitivity_db",
        dsp.suppressor.sensitivity_db,
    );
    push(&mut out, "dsp.suppressor.adapt_ms", dsp.suppressor.adapt_ms);
    push(&mut out, "dsp.de_esser.enabled", dsp.de_esser.enabled);
    push(&mut out, "dsp.de_esser.amount", dsp.de_esser.amount);
    push(
        &mut out,
        "dsp.de_esser.frequency_hz",
        dsp.de_esser.frequency_hz,
    );
    push(&mut out, "dsp.exciter.enabled", dsp.exciter.enabled);
    push(&mut out, "dsp.exciter.amount", dsp.exciter.amount);
    push(&mut out, "dsp.exciter.tone", dsp.exciter.tone);
    push(&mut out, "dsp.mic_output_gain_db", dsp.mic_output_gain_db);

    let hp = &dsp.headphones;
    push(&mut out, "dsp.headphones.level_db", hp.level_db);
    push(&mut out, "dsp.headphones.mic_monitor_db", hp.mic_monitor_db);
    push(
        &mut out,
        "dsp.headphones.power",
        headphone_power_name(hp.power),
    );
    push(&mut out, "dsp.headphones.fx_enabled", hp.fx_enabled);
    push(&mut out, "dsp.headphones.mono", hp.mono);
    push(&mut out, "dsp.headphones.balance", hp.balance);
    push(&mut out, "dsp.headphones.eq_linked", hp.eq_linked);
    push(
        &mut out,
        "dsp.headphones.preset_name",
        clean_value(&hp.preset_name),
    );
    push(
        &mut out,
        "dsp.headphones.binaural_personalization",
        hp.binaural_personalization,
    );
    for (index, band) in hp.eq_left.iter().enumerate() {
        encode_band(&mut out, &format!("dsp.headphones.left.{index}"), band);
    }
    for (index, band) in hp.eq_right.iter().enumerate() {
        encode_band(&mut out, &format!("dsp.headphones.right.{index}"), band);
    }
    out
}

pub fn decode(text: &str) -> io::Result<Profile> {
    let mut dsp = SoftwareDspState::default();
    if optional_field(text, "schema").is_some() || optional_field(text, "dsp.mic_gain_db").is_some()
    {
        dsp.mic_gain_db = opt_f32(text, "dsp.mic_gain_db", dsp.mic_gain_db)?;
        dsp.mic_eq_mode = opt_processor_mode(text, "dsp.mic_eq_mode", dsp.mic_eq_mode)?;
        for index in 0..MIC_EQ_BAND_COUNT {
            decode_band(text, &format!("dsp.mic_eq.{index}"), &mut dsp.mic_eq[index])?;
        }
        dsp.compressor.mode = opt_processor_mode(text, "dsp.compressor.mode", dsp.compressor.mode)?;
        dsp.compressor.enabled = opt_bool(text, "dsp.compressor.enabled", dsp.compressor.enabled)?;
        dsp.compressor.threshold_db = opt_f32(
            text,
            "dsp.compressor.threshold_db",
            dsp.compressor.threshold_db,
        )?;
        dsp.compressor.ratio = opt_f32(text, "dsp.compressor.ratio", dsp.compressor.ratio)?;
        dsp.compressor.attack_ms =
            opt_f32(text, "dsp.compressor.attack_ms", dsp.compressor.attack_ms)?;
        dsp.compressor.release_ms =
            opt_f32(text, "dsp.compressor.release_ms", dsp.compressor.release_ms)?;
        dsp.compressor.makeup_gain_db = opt_f32(
            text,
            "dsp.compressor.makeup_gain_db",
            dsp.compressor.makeup_gain_db,
        )?;
        dsp.expander.mode = opt_processor_mode(text, "dsp.expander.mode", dsp.expander.mode)?;
        dsp.expander.enabled = opt_bool(text, "dsp.expander.enabled", dsp.expander.enabled)?;
        dsp.expander.threshold_db =
            opt_f32(text, "dsp.expander.threshold_db", dsp.expander.threshold_db)?;
        dsp.expander.ratio = opt_f32(text, "dsp.expander.ratio", dsp.expander.ratio)?;
        dsp.expander.attack_ms = opt_f32(text, "dsp.expander.attack_ms", dsp.expander.attack_ms)?;
        dsp.expander.release_ms =
            opt_f32(text, "dsp.expander.release_ms", dsp.expander.release_ms)?;
        dsp.suppressor.enabled = opt_bool(text, "dsp.suppressor.enabled", dsp.suppressor.enabled)?;
        dsp.suppressor.amount = opt_f32(text, "dsp.suppressor.amount", dsp.suppressor.amount)?;
        dsp.suppressor.style = opt_noise_style(text, "dsp.suppressor.style", dsp.suppressor.style)?;
        dsp.suppressor.sensitivity_db = opt_f32(
            text,
            "dsp.suppressor.sensitivity_db",
            dsp.suppressor.sensitivity_db,
        )?;
        dsp.suppressor.adapt_ms =
            opt_f32(text, "dsp.suppressor.adapt_ms", dsp.suppressor.adapt_ms)?;
        dsp.de_esser.enabled = opt_bool(text, "dsp.de_esser.enabled", dsp.de_esser.enabled)?;
        dsp.de_esser.amount = opt_f32(text, "dsp.de_esser.amount", dsp.de_esser.amount)?;
        dsp.de_esser.frequency_hz =
            opt_f32(text, "dsp.de_esser.frequency_hz", dsp.de_esser.frequency_hz)?;
        dsp.exciter.enabled = opt_bool(text, "dsp.exciter.enabled", dsp.exciter.enabled)?;
        dsp.exciter.amount = opt_f32(text, "dsp.exciter.amount", dsp.exciter.amount)?;
        dsp.exciter.tone = opt_f32(text, "dsp.exciter.tone", dsp.exciter.tone)?;
        dsp.mic_output_gain_db = opt_f32(text, "dsp.mic_output_gain_db", dsp.mic_output_gain_db)?;
        dsp.headphones.level_db =
            opt_f32(text, "dsp.headphones.level_db", dsp.headphones.level_db)?;
        dsp.headphones.mic_monitor_db = opt_f32(
            text,
            "dsp.headphones.mic_monitor_db",
            dsp.headphones.mic_monitor_db,
        )?;
        dsp.headphones.power =
            opt_headphone_power(text, "dsp.headphones.power", dsp.headphones.power)?;
        dsp.headphones.fx_enabled =
            opt_bool(text, "dsp.headphones.fx_enabled", dsp.headphones.fx_enabled)?;
        dsp.headphones.mono = opt_bool(text, "dsp.headphones.mono", dsp.headphones.mono)?;
        dsp.headphones.balance = opt_i32(text, "dsp.headphones.balance", dsp.headphones.balance)?;
        dsp.headphones.eq_linked =
            opt_bool(text, "dsp.headphones.eq_linked", dsp.headphones.eq_linked)?;
        if let Some(value) = optional_field(text, "dsp.headphones.preset_name") {
            dsp.headphones.preset_name = value.to_owned();
        }
        dsp.headphones.binaural_personalization = opt_bool(
            text,
            "dsp.headphones.binaural_personalization",
            dsp.headphones.binaural_personalization,
        )?;
        for index in 0..HEADPHONE_EQ_BAND_COUNT {
            decode_band(
                text,
                &format!("dsp.headphones.left.{index}"),
                &mut dsp.headphones.eq_left[index],
            )?;
            decode_band(
                text,
                &format!("dsp.headphones.right.{index}"),
                &mut dsp.headphones.eq_right[index],
            )?;
        }
    }
    dsp.sanitize();

    Ok(Profile {
        name: field(text, "name")?.to_owned(),
        source_name: field(text, "source_name")?.to_owned(),
        source_volume: parse_f32(field(text, "source_volume")?, "source_volume")?,
        source_muted: parse_bool(field(text, "source_muted")?, "source_muted")?,
        sink_name: field(text, "sink_name")?.to_owned(),
        sink_volume: parse_f32(field(text, "sink_volume")?, "sink_volume")?,
        sink_muted: parse_bool(field(text, "sink_muted")?, "sink_muted")?,
        dsp,
    })
}

fn encode_band(out: &mut String, prefix: &str, band: &crate::hardware::EqBandState) {
    push(out, &format!("{prefix}.kind"), eq_kind_name(band.kind));
    push(out, &format!("{prefix}.gain_db"), band.gain_db);
    push(out, &format!("{prefix}.frequency_hz"), band.frequency_hz);
    push(out, &format!("{prefix}.q"), band.q);
    push(out, &format!("{prefix}.enabled"), band.enabled);
}

fn decode_band(
    text: &str,
    prefix: &str,
    band: &mut crate::hardware::EqBandState,
) -> io::Result<()> {
    band.kind = opt_eq_kind(text, &format!("{prefix}.kind"), band.kind)?;
    band.gain_db = opt_f32(text, &format!("{prefix}.gain_db"), band.gain_db)?;
    band.frequency_hz = opt_f32(text, &format!("{prefix}.frequency_hz"), band.frequency_hz)?;
    band.q = opt_f32(text, &format!("{prefix}.q"), band.q)?;
    band.enabled = opt_bool(text, &format!("{prefix}.enabled"), band.enabled)?;
    Ok(())
}

fn push(out: &mut String, key: &str, value: impl std::fmt::Display) {
    out.push_str(key);
    out.push('=');
    out.push_str(&value.to_string());
    out.push('\n');
}

fn field<'a>(text: &'a str, key: &str) -> io::Result<&'a str> {
    optional_field(text, key)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("missing {key}")))
}

fn optional_field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("{key}=");
    text.lines().find_map(|line| line.strip_prefix(&prefix))
}

pub fn save(profile: &Profile) -> io::Result<PathBuf> {
    let dir = profiles_dir()?;
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.profile", safe_name(&profile.name)));
    atomic_write(&path, &encode(profile))?;
    Ok(path)
}

pub fn load(name: &str) -> io::Result<Profile> {
    let path = profiles_dir()?.join(format!("{}.profile", safe_name(name)));
    decode(&fs::read_to_string(path)?)
}

pub fn list() -> io::Result<Vec<String>> {
    list_extension(&profiles_dir()?, "profile")
}

pub fn save_snapshot(profile: &Profile, label: &str) -> io::Result<PathBuf> {
    let dir = snapshots_dir()?;
    fs::create_dir_all(&dir)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let path = dir.join(format!(
        "{}--{}--{}.snapshot",
        safe_name(&profile.name),
        safe_name(label),
        stamp
    ));
    atomic_write(&path, &encode(profile))?;
    Ok(path)
}

pub fn list_snapshots() -> io::Result<Vec<String>> {
    list_extension(&snapshots_dir()?, "snapshot")
}

pub fn load_snapshot(name: &str) -> io::Result<Profile> {
    let path = snapshots_dir()?.join(format!("{}.snapshot", safe_name(name)));
    decode(&fs::read_to_string(path)?)
}

fn list_extension(dir: &Path, extension: &str) -> io::Result<Vec<String>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some(extension) {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|value| value.to_str()) {
            names.push(stem.to_owned());
        }
    }
    names.sort();
    Ok(names)
}

fn profiles_dir() -> io::Result<PathBuf> {
    Ok(config_root()?.join("profiles"))
}

fn snapshots_dir() -> io::Result<PathBuf> {
    Ok(config_root()?.join("snapshots"))
}

fn config_root() -> io::Result<PathBuf> {
    let home = env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is not set"))?;
    Ok(home.join(".config/aetherforge-beacn-control"))
}

fn atomic_write(path: &Path, contents: &str) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, contents)?;
    fs::rename(tmp, path)
}

fn safe_name(name: &str) -> String {
    let filtered: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if filtered.is_empty() {
        "profile".to_owned()
    } else {
        filtered
    }
}

fn clean_value(value: &str) -> String {
    value.replace(['\n', '\r'], " ")
}

fn parse_f32(value: &str, field: &str) -> io::Result<f32> {
    value.parse::<f32>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid {field}: {error}"),
        )
    })
}

fn parse_i32(value: &str, field: &str) -> io::Result<i32> {
    value.parse::<i32>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid {field}: {error}"),
        )
    })
}

fn parse_bool(value: &str, field: &str) -> io::Result<bool> {
    value.parse::<bool>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid {field}: {error}"),
        )
    })
}

fn opt_f32(text: &str, key: &str, default: f32) -> io::Result<f32> {
    optional_field(text, key).map_or(Ok(default), |value| parse_f32(value, key))
}

fn opt_i32(text: &str, key: &str, default: i32) -> io::Result<i32> {
    optional_field(text, key).map_or(Ok(default), |value| parse_i32(value, key))
}

fn opt_bool(text: &str, key: &str, default: bool) -> io::Result<bool> {
    optional_field(text, key).map_or(Ok(default), |value| parse_bool(value, key))
}

fn processor_mode_name(value: ProcessorMode) -> &'static str {
    match value {
        ProcessorMode::Simple => "simple",
        ProcessorMode::Advanced => "advanced",
    }
}
fn opt_processor_mode(text: &str, key: &str, default: ProcessorMode) -> io::Result<ProcessorMode> {
    match optional_field(text, key) {
        None => Ok(default),
        Some("simple") => Ok(ProcessorMode::Simple),
        Some("advanced") => Ok(ProcessorMode::Advanced),
        Some(value) => Err(invalid_enum(key, value)),
    }
}
fn noise_style_name(value: NoiseStyle) -> &'static str {
    match value {
        NoiseStyle::Instant => "instant",
        NoiseStyle::Adaptive => "adaptive",
        NoiseStyle::Snapshot => "snapshot",
    }
}
fn opt_noise_style(text: &str, key: &str, default: NoiseStyle) -> io::Result<NoiseStyle> {
    match optional_field(text, key) {
        None => Ok(default),
        Some("instant") => Ok(NoiseStyle::Instant),
        Some("adaptive") => Ok(NoiseStyle::Adaptive),
        Some("snapshot") => Ok(NoiseStyle::Snapshot),
        Some(value) => Err(invalid_enum(key, value)),
    }
}
fn eq_kind_name(value: EqBandKind) -> &'static str {
    match value {
        EqBandKind::NotSet => "not-set",
        EqBandKind::LowPass => "low-pass",
        EqBandKind::HighPass => "high-pass",
        EqBandKind::Notch => "notch",
        EqBandKind::Bell => "bell",
        EqBandKind::LowShelf => "low-shelf",
        EqBandKind::HighShelf => "high-shelf",
    }
}
fn opt_eq_kind(text: &str, key: &str, default: EqBandKind) -> io::Result<EqBandKind> {
    match optional_field(text, key) {
        None => Ok(default),
        Some("not-set") => Ok(EqBandKind::NotSet),
        Some("low-pass") => Ok(EqBandKind::LowPass),
        Some("high-pass") => Ok(EqBandKind::HighPass),
        Some("notch") => Ok(EqBandKind::Notch),
        Some("bell") => Ok(EqBandKind::Bell),
        Some("low-shelf") => Ok(EqBandKind::LowShelf),
        Some("high-shelf") => Ok(EqBandKind::HighShelf),
        Some(value) => Err(invalid_enum(key, value)),
    }
}
fn headphone_power_name(value: HeadphonePower) -> &'static str {
    match value {
        HeadphonePower::LineLevel => "line-level",
        HeadphonePower::Normal => "normal",
        HeadphonePower::HighImpedance => "high-impedance",
        HeadphonePower::InEarMonitors => "iem",
    }
}
fn opt_headphone_power(
    text: &str,
    key: &str,
    default: HeadphonePower,
) -> io::Result<HeadphonePower> {
    match optional_field(text, key) {
        None => Ok(default),
        Some("line-level") => Ok(HeadphonePower::LineLevel),
        Some("normal") => Ok(HeadphonePower::Normal),
        Some("high-impedance") => Ok(HeadphonePower::HighImpedance),
        Some("iem") => Ok(HeadphonePower::InEarMonitors),
        Some(value) => Err(invalid_enum(key, value)),
    }
}
fn invalid_enum(key: &str, value: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("invalid {key}: {value}"),
    )
}
