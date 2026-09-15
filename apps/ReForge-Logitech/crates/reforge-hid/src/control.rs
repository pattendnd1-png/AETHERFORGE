use super::{feature_index, open_summary_device, transact_retry};
use hidapi::HidDevice;
use reforge_core::{
    ControlChoice, ControlGroup, ControlKind, ControlValue, DeviceControl, DeviceSummary,
    DpiInfo, FeatureInfo, ProviderKind, ReadbackKind,
};
use reforge_protocol::{feature::*, HidppRequest};

fn request(
    handle: &HidDevice,
    device_index: u8,
    feature_index: u8,
    function: u8,
    params: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let request = HidppRequest::new(device_index, feature_index, function, params)
        .map_err(|error| error.to_string())?;
    Ok(transact_retry(handle, &request)?.params)
}

fn control(
    id: &str,
    label: &str,
    backend_id: &str,
    kind: ControlKind,
    value: ControlValue,
    group: ControlGroup,
    writable: bool,
    profile_eligible: bool,
    readback: ReadbackKind,
) -> DeviceControl {
    DeviceControl {
        id: format!("hidpp:{id}"),
        label: label.to_owned(),
        provider: ProviderKind::Hidpp,
        endpoint: "hidpp".into(),
        backend_id: backend_id.to_owned(),
        kind,
        value,
        min: None,
        max: None,
        step: None,
        choices: vec![],
        writable,
        profile_eligible,
        group,
        readback,
    }
}

fn range_control(
    id: &str,
    label: &str,
    backend_id: &str,
    value: i64,
    min: i64,
    max: i64,
    step: i64,
    group: ControlGroup,
) -> DeviceControl {
    let mut item = control(
        id,
        label,
        backend_id,
        ControlKind::Range,
        ControlValue::Int(value),
        group,
        true,
        true,
        ReadbackKind::Readable,
    );
    item.min = Some(min);
    item.max = Some(max);
    item.step = Some(step.max(1));
    item
}

fn toggle_control(
    id: &str,
    label: &str,
    backend_id: &str,
    value: bool,
    group: ControlGroup,
    readback: ReadbackKind,
) -> DeviceControl {
    let mut item = control(
        id,
        label,
        backend_id,
        ControlKind::Toggle,
        ControlValue::Bool(value),
        group,
        true,
        true,
        readback,
    );
    item.min = Some(0);
    item.max = Some(1);
    item.step = Some(1);
    item
}

fn status_control(id: &str, label: &str, value: ControlValue, group: ControlGroup) -> DeviceControl {
    control(
        id,
        label,
        id,
        ControlKind::Status,
        value,
        group,
        false,
        false,
        ReadbackKind::Telemetry,
    )
}

fn choice_control(
    id: &str,
    label: &str,
    backend_id: &str,
    value: i64,
    choices: Vec<ControlChoice>,
    group: ControlGroup,
) -> DeviceControl {
    let mut item = control(
        id,
        label,
        backend_id,
        ControlKind::Choice,
        ControlValue::Int(value),
        group,
        true,
        true,
        ReadbackKind::Readable,
    );
    item.choices = choices;
    item
}

fn feature_version(features: &[FeatureInfo], feature_id: u16) -> u8 {
    features
        .iter()
        .find(|feature| feature.feature_id == feature_id)
        .map(|feature| feature.version)
        .unwrap_or(0)
}

fn probe_dpi(dpi: Option<&DpiInfo>) -> Option<DeviceControl> {
    let dpi = dpi?;
    let mut item = range_control(
        "dpi",
        "Pointer DPI",
        "dpi",
        i64::from(dpi.current),
        i64::from(dpi.min),
        i64::from(dpi.max),
        i64::from(dpi.step.max(1)),
        ControlGroup::Performance,
    );
    if !dpi.supported.is_empty() {
        item.kind = ControlKind::Choice;
        item.choices = dpi
            .supported
            .iter()
            .map(|value| ControlChoice {
                value: i64::from(*value),
                label: format!("{value} DPI"),
            })
            .collect();
    }
    Some(item)
}

fn probe_report_rate(
    handle: &HidDevice,
    device_index: u8,
    features: &[FeatureInfo],
    controls: &mut Vec<DeviceControl>,
) {
    if let Some(index) = feature_index(features, REPORT_RATE) {
        if let (Ok(info), Ok(current)) = (
            request(handle, device_index, index, 0, vec![0, 0, 0]),
            request(handle, device_index, index, 1, vec![0, 0, 0]),
        ) {
            let flags = info.first().copied().unwrap_or(0);
            let choices: Vec<_> = (1_u8..=8)
                .filter(|rate| flags & (1 << (rate - 1)) != 0)
                .map(|rate| ControlChoice {
                    value: i64::from(rate),
                    label: format!("{rate} ms"),
                })
                .collect();
            if !choices.is_empty() {
                controls.push(choice_control(
                    "report_rate",
                    "Report Rate",
                    "report_rate",
                    i64::from(current.first().copied().unwrap_or(choices[0].value as u8)),
                    choices,
                    ControlGroup::Performance,
                ));
            }
        }
    }

    if let Some(index) = feature_index(features, EXTENDED_REPORT_RATE) {
        if let (Ok(info), Ok(current)) = (
            request(handle, device_index, index, 1, vec![0, 0, 0]),
            request(handle, device_index, index, 2, vec![0, 0, 0]),
        ) {
            let flags = u16::from_be_bytes([
                info.first().copied().unwrap_or(0),
                info.get(1).copied().unwrap_or(0),
            ]);
            let labels = ["8 ms", "4 ms", "2 ms", "1 ms", "500 us", "250 us", "125 us"];
            let choices: Vec<_> = (0_u8..7)
                .filter(|rate| flags & (1 << rate) != 0)
                .map(|rate| ControlChoice {
                    value: i64::from(rate),
                    label: labels[usize::from(rate)].to_owned(),
                })
                .collect();
            if !choices.is_empty() {
                controls.push(choice_control(
                    "report_rate_extended",
                    "Report Rate",
                    "report_rate_extended",
                    i64::from(current.first().copied().unwrap_or(choices[0].value as u8)),
                    choices,
                    ControlGroup::Performance,
                ));
            }
        }
    }
}

fn probe_pointer_speed(
    handle: &HidDevice,
    device_index: u8,
    features: &[FeatureInfo],
    controls: &mut Vec<DeviceControl>,
) {
    let Some(index) = feature_index(features, POINTER_SPEED) else {
        return;
    };
    if let Ok(data) = request(handle, device_index, index, 0, vec![0, 0, 0])
        && data.len() >= 2
    {
        let current = u16::from_be_bytes([data[0], data[1]]);
        controls.push(range_control(
            "pointer_speed",
            "Pointer Speed",
            "pointer_speed",
            i64::from(current),
            0x002e,
            0x01ff,
            1,
            ControlGroup::Performance,
        ));
    }
}

fn probe_hires_wheel(
    handle: &HidDevice,
    device_index: u8,
    features: &[FeatureInfo],
    controls: &mut Vec<DeviceControl>,
) {
    let Some(index) = feature_index(features, HIRES_WHEEL) else {
        return;
    };
    let Ok(caps) = request(handle, device_index, index, 0, vec![0, 0, 0]) else {
        return;
    };
    let Ok(mode) = request(handle, device_index, index, 1, vec![0, 0, 0]) else {
        return;
    };
    let flags = mode.first().copied().unwrap_or(0);
    controls.push(toggle_control(
        "hires_divert",
        "High-Resolution Wheel Diversion",
        "hires_divert",
        flags & 0x01 != 0,
        ControlGroup::Performance,
        ReadbackKind::Readable,
    ));
    controls.push(toggle_control(
        "hires_resolution",
        "High-Resolution Wheel",
        "hires_resolution",
        flags & 0x02 != 0,
        ControlGroup::Performance,
        ReadbackKind::Readable,
    ));
    if caps.get(1).copied().unwrap_or(0) & 0x08 != 0 {
        controls.push(toggle_control(
            "hires_invert",
            "Wheel Direction Inverted",
            "hires_invert",
            flags & 0x04 != 0,
            ControlGroup::Performance,
            ReadbackKind::Readable,
        ));
    }
    if caps.get(1).copied().unwrap_or(0) & 0x04 != 0
        && let Ok(ratchet) = request(handle, device_index, index, 3, vec![0, 0, 0])
    {
        controls.push(status_control(
            "hires_ratchet_status",
            "Wheel Ratchet Status",
            ControlValue::Bool(ratchet.first().copied().unwrap_or(0) & 0x01 != 0),
            ControlGroup::Overview,
        ));
    }
}

fn probe_thumb_wheel(
    handle: &HidDevice,
    device_index: u8,
    features: &[FeatureInfo],
    controls: &mut Vec<DeviceControl>,
) {
    let Some(index) = feature_index(features, THUMB_WHEEL) else {
        return;
    };
    if let Ok(mode) = request(handle, device_index, index, 1, vec![0, 0, 0]) {
        controls.push(toggle_control(
            "thumb_divert",
            "Thumb Wheel Diversion",
            "thumb_divert",
            mode.first().copied().unwrap_or(0) & 0x01 != 0,
            ControlGroup::Performance,
            ReadbackKind::Readable,
        ));
        controls.push(toggle_control(
            "thumb_invert",
            "Thumb Wheel Direction Inverted",
            "thumb_invert",
            mode.get(1).copied().unwrap_or(0) & 0x01 != 0,
            ControlGroup::Performance,
            ReadbackKind::Readable,
        ));
    }
}

fn probe_smart_shift(
    handle: &HidDevice,
    device_index: u8,
    features: &[FeatureInfo],
    controls: &mut Vec<DeviceControl>,
) {
    if let Some(index) = feature_index(features, SMART_SHIFT)
        && let Ok(data) = request(handle, device_index, index, 0, vec![0, 0, 0])
    {
        let mode = data.first().copied().unwrap_or(0);
        let raw = data.get(1).copied().unwrap_or(1);
        let threshold = if mode == 1 { 1 } else { raw.min(50).max(1) };
        controls.push(range_control(
            "smart_shift",
            "SmartShift Ratchet Speed",
            "smart_shift",
            i64::from(threshold),
            1,
            50,
            1,
            ControlGroup::Performance,
        ));
    }

    if let Some(index) = feature_index(features, SMART_SHIFT_ENHANCED)
        && let Ok(data) = request(handle, device_index, index, 1, vec![0, 0, 0])
    {
        let threshold = data.get(1).copied().unwrap_or(1).min(50).max(1);
        controls.push(range_control(
            "smart_shift_enhanced",
            "SmartShift Ratchet Speed",
            "smart_shift_enhanced",
            i64::from(threshold),
            1,
            50,
            1,
            ControlGroup::Performance,
        ));
        if let Ok(info) = request(handle, device_index, index, 0, vec![0, 0, 0])
            && info.first().copied().unwrap_or(0) & 0x01 != 0
        {
            controls.push(range_control(
                "smart_shift_torque",
                "Scroll Ratchet Torque",
                "smart_shift_torque",
                i64::from(data.get(2).copied().unwrap_or(50)),
                1,
                100,
                1,
                ControlGroup::Performance,
            ));
        }
    }
}


#[derive(Debug, Clone)]
struct ReprogKey {
    cid: u16,
    task: u16,
    group: u8,
    group_mask: u8,
}

fn probe_reprogrammable_controls(
    handle: &HidDevice,
    device_index: u8,
    features: &[FeatureInfo],
    controls: &mut Vec<DeviceControl>,
) {
    let Some(index) = feature_index(features, REPROG_CONTROLS_V4) else {
        return;
    };
    let Ok(info) = request(handle, device_index, index, 0, vec![0, 0, 0]) else {
        return;
    };
    let count = usize::from(info.first().copied().unwrap_or(0)).min(64);
    if count == 0 {
        return;
    }

    let mut keys = Vec::new();
    for key_index in 0..count {
        let Ok(data) = request(handle, device_index, index, 1, vec![key_index as u8, 0, 0]) else {
            continue;
        };
        if data.len() < 9 {
            continue;
        }
        keys.push(ReprogKey {
            cid: u16::from_be_bytes([data[0], data[1]]),
            task: u16::from_be_bytes([data[2], data[3]]),
            group: data[6],
            group_mask: data[7],
        });
    }

    for key in &keys {
        if key.group_mask == 0 {
            continue;
        }
        let [cid_hi, cid_lo] = key.cid.to_be_bytes();
        let current = request(handle, device_index, index, 2, vec![cid_hi, cid_lo, 0])
            .ok()
            .and_then(|data| {
                (data.len() >= 5).then(|| {
                    let mapped = u16::from_be_bytes([data[3], data[4]]);
                    if mapped == 0 { key.cid } else { mapped }
                })
            })
            .unwrap_or(key.cid);

        let mut choices = Vec::new();
        choices.push(ControlChoice {
            value: i64::from(key.cid),
            label: format!("Default task 0x{:04X}", key.task),
        });
        for target in &keys {
            if target.cid == key.cid || target.group == 0 || target.group > 8 {
                continue;
            }
            let mask = 1_u8 << (target.group - 1);
            if key.group_mask & mask != 0 {
                choices.push(ControlChoice {
                    value: i64::from(target.cid),
                    label: format!("Control 0x{:04X} / task 0x{:04X}", target.cid, target.task),
                });
            }
        }
        choices.sort_by_key(|choice| choice.value);
        choices.dedup_by_key(|choice| choice.value);
        if choices.len() <= 1 {
            continue;
        }
        controls.push(choice_control(
            &format!("assignment_{:04x}", key.cid),
            &format!("Control 0x{:04X} Assignment", key.cid),
            &format!("reprog:{:04x}", key.cid),
            i64::from(current),
            choices,
            ControlGroup::Assignments,
        ));
    }
}

fn probe_analog_buttons(
    handle: &HidDevice,
    device_index: u8,
    features: &[FeatureInfo],
    controls: &mut Vec<DeviceControl>,
) {
    let Some(index) = feature_index(features, ANALOG_BUTTONS) else {
        return;
    };
    let Ok(caps) = request(handle, device_index, index, 0, vec![0, 0, 0]) else {
        return;
    };
    if caps.len() < 5 {
        return;
    }
    let count = usize::from(caps[1].min(2));
    let max_actuation = i64::from((caps[2] >> 2).max(1));
    let max_rapid = i64::from((caps[3] >> 2).max(1));
    let max_haptics = i64::from(caps[4] >> 2);
    for button in 0..count {
        let Ok(state) = request(handle, device_index, index, 2, vec![button as u8, 0, 0]) else {
            continue;
        };
        if state.len() < 4 {
            continue;
        }
        let name = if button == 0 { "Left" } else { "Right" };
        controls.push(range_control(
            &format!("analog_actuation_{button}"),
            &format!("{name} Button Actuation"),
            &format!("analog:{button}:actuation"),
            i64::from(state[1] >> 2),
            1,
            max_actuation,
            1,
            ControlGroup::Performance,
        ));
        controls.push(range_control(
            &format!("analog_rapid_{button}"),
            &format!("{name} Button Rapid Trigger"),
            &format!("analog:{button}:rapid"),
            i64::from(state[2] >> 2),
            1,
            max_rapid,
            1,
            ControlGroup::Performance,
        ));
        controls.push(range_control(
            &format!("analog_haptics_{button}"),
            &format!("{name} Button Haptics"),
            &format!("analog:{button}:haptics"),
            i64::from(state[3] >> 2),
            0,
            max_haptics,
            1,
            ControlGroup::Performance,
        ));
    }
}

fn probe_keyboard_controls(
    handle: &HidDevice,
    device_index: u8,
    features: &[FeatureInfo],
    controls: &mut Vec<DeviceControl>,
) {
    for (feature_id, backend, label) in [
        (NEW_FN_INVERSION, "new_fn_inversion", "Function-Key Mode"),
        (FN_INVERSION, "fn_inversion", "Function-Key Mode"),
    ] {
        if let Some(index) = feature_index(features, feature_id)
            && let Ok(data) = request(handle, device_index, index, 0, vec![0, 0, 0])
        {
            controls.push(toggle_control(
                backend,
                label,
                backend,
                data.first().copied().unwrap_or(0) & 0x01 != 0,
                ControlGroup::Assignments,
                ReadbackKind::Readable,
            ));
            break;
        }
    }

    if feature_index(features, GKEY).is_some() {
        controls.push(toggle_control(
            "gkey_divert",
            "Divert G / M Keys",
            "gkey_divert",
            false,
            ControlGroup::Assignments,
            ReadbackKind::CanonicalWriteOnly,
        ));
    }

    if let Some(index) = feature_index(features, ONBOARD_PROFILES)
        && let Ok(data) = request(handle, device_index, index, 2, vec![0, 0, 0])
    {
        controls.push(toggle_control(
            "onboard_profiles",
            "Onboard Profiles",
            "onboard_profiles",
            data.first().copied().unwrap_or(0) == 0x01,
            ControlGroup::Profiles,
            ReadbackKind::Readable,
        ));
    }
}

fn probe_battery(
    handle: &HidDevice,
    device_index: u8,
    features: &[FeatureInfo],
    controls: &mut Vec<DeviceControl>,
) {
    if let Some(index) = feature_index(features, UNIFIED_BATTERY)
        && let Ok(data) = request(handle, device_index, index, 1, vec![0, 0, 0])
        && let Some(level) = data.first().copied()
    {
        let mut item = status_control(
            "battery_percent",
            "Battery",
            ControlValue::Int(i64::from(level)),
            ControlGroup::Power,
        );
        item.min = Some(0);
        item.max = Some(100);
        controls.push(item);
        return;
    }
    if let Some(index) = feature_index(features, BATTERY_STATUS)
        && let Ok(data) = request(handle, device_index, index, 0, vec![0, 0, 0])
        && let Some(level) = data.first().copied()
    {
        let mut item = status_control(
            "battery_percent",
            "Battery",
            ControlValue::Int(i64::from(level)),
            ControlGroup::Power,
        );
        item.min = Some(0);
        item.max = Some(100);
        controls.push(item);
    }
}

fn probe_audio(
    handle: &HidDevice,
    device_index: u8,
    features: &[FeatureInfo],
    controls: &mut Vec<DeviceControl>,
) {
    if let Some(index) = feature_index(features, SIDETONE)
        && let Ok(data) = request(handle, device_index, index, 0, vec![0, 0, 0])
    {
        controls.push(range_control(
            "sidetone",
            "Sidetone",
            "sidetone",
            i64::from(data.first().copied().unwrap_or(0).min(100)),
            0,
            100,
            1,
            ControlGroup::Audio,
        ));
    }

    if let Some(index) = feature_index(features, EQUALIZER)
        && let Ok(info) = request(handle, device_index, index, 0, vec![0, 0, 0])
        && info.len() >= 5
    {
        let count = usize::from(info[0]);
        let db_range = i64::from(info[1]);
        let min = if info[3] == 0 { -db_range } else { i64::from(info[3] as i8) };
        let max = if info[4] == 0 { db_range } else { i64::from(info[4] as i8) };
        if count > 0 && count <= 15
            && let Ok(data) = request(handle, device_index, index, 2, vec![0])
        {
            let values = data
                .iter()
                .take(count)
                .map(|value| i64::from(*value as i8))
                .collect::<Vec<_>>();
            if values.len() == count {
                let mut item = control(
                    "equalizer",
                    "Equalizer",
                    "equalizer",
                    ControlKind::Vector,
                    ControlValue::Vector(values),
                    ControlGroup::Audio,
                    true,
                    true,
                    ReadbackKind::Readable,
                );
                item.min = Some(min);
                item.max = Some(max);
                item.step = Some(1);
                controls.push(item);
            }
        }
    }

    for (feature_id, backend, label, read_fn, write_fn) in [
        (HEADSET_MIC_MUTE, "headset_mic_mute", "Microphone Mute", 1_u8, 2_u8),
        (HEADSET_MIC_SNR, "headset_mic_snr", "Microphone SNR", 0, 1),
        (HEADSET_BATTERY_SAVER, "headset_battery_saver", "Battery Saver", 0, 1),
        (HEADSET_DO_NOT_DISTURB, "headset_dnd", "Do Not Disturb", 0, 1),
    ] {
        if let Some(index) = feature_index(features, feature_id)
            && let Ok(data) = request(handle, device_index, index, read_fn, vec![0, 0, 0])
        {
            let mut item = toggle_control(
                backend,
                label,
                backend,
                data.first().copied().unwrap_or(0) != 0,
                ControlGroup::Audio,
                ReadbackKind::Readable,
            );
            item.backend_id = format!("{backend}:{read_fn}:{write_fn}");
            controls.push(item);
        }
    }

    if let Some(index) = feature_index(features, HEADSET_AI_NOISE_REDUCTION) {
        if let Ok(data) = request(handle, device_index, index, 0, vec![0, 0, 0]) {
            let mut item = toggle_control(
                "headset_ai_nr",
                "AI Noise Reduction",
                "headset_ai_nr:0:1",
                data.first().copied().unwrap_or(0) != 0,
                ControlGroup::Audio,
                ReadbackKind::Readable,
            );
            item.backend_id = "headset_ai_nr:0:1".into();
            controls.push(item);
        }
        if let Ok(data) = request(handle, device_index, index, 2, vec![0, 0, 0]) {
            controls.push(choice_control(
                "headset_ai_nr_level",
                "AI Noise Reduction Level",
                "headset_ai_nr_level",
                i64::from(data.first().copied().unwrap_or(0).min(3)),
                vec![
                    ControlChoice { value: 0, label: "Off".into() },
                    ControlChoice { value: 1, label: "Low".into() },
                    ControlChoice { value: 2, label: "Medium".into() },
                    ControlChoice { value: 3, label: "High".into() },
                ],
                ControlGroup::Audio,
            ));
        }
    }

    if let Some(index) = feature_index(features, HEADSET_MIC_GAIN)
        && let Ok(info) = request(handle, device_index, index, 0, vec![0, 0, 0])
        && info.len() >= 2
        && let Ok(data) = request(handle, device_index, index, 1, vec![0, 0, 0])
    {
        let min = i64::from(info[0] as i8);
        let max = i64::from(info[1] as i8);
        if min < max {
            controls.push(range_control(
                "headset_mic_gain",
                "Microphone Gain",
                "headset_mic_gain",
                i64::from(data.first().copied().unwrap_or(0) as i8),
                min,
                max,
                1,
                ControlGroup::Audio,
            ));
        }
    }

    if let Some(index) = feature_index(features, HEADSET_AUDIO_SIDETONE) {
        let version = feature_version(features, HEADSET_AUDIO_SIDETONE);
        if let Ok(data) = request(handle, device_index, index, 0, vec![0, 0, 0]) {
            let (offset, gain_steps) = if version > 1 {
                let steps = request(handle, device_index, index, 2, vec![0, 0, 0])
                    .ok()
                    .and_then(|reply| reply.get(2).copied())
                    .filter(|steps| *steps > 1)
                    .unwrap_or(101);
                (3_usize, steps)
            } else {
                (2_usize, 101)
            };
            if let Some(raw) = data.get(offset).copied() {
                let percent = if gain_steps > 1 {
                    ((u32::from(raw) * 100 + u32::from(gain_steps - 1) / 2)
                        / u32::from(gain_steps - 1)) as i64
                } else {
                    i64::from(raw)
                };
                let mut item = range_control(
                    "headset_sidetone",
                    "Headset Sidetone",
                    "headset_sidetone",
                    percent.clamp(0, 100),
                    0,
                    100,
                    1,
                    ControlGroup::Audio,
                );
                item.endpoint = format!("hidpp:v{version}:steps={gain_steps}");
                controls.push(item);
            }
        }
    }
}

pub(super) fn probe_controls(
    handle: &HidDevice,
    device_index: u8,
    features: &[FeatureInfo],
    dpi: Option<&DpiInfo>,
) -> Vec<DeviceControl> {
    let mut controls = Vec::new();
    if let Some(dpi) = probe_dpi(dpi) {
        controls.push(dpi);
    }
    probe_report_rate(handle, device_index, features, &mut controls);
    probe_pointer_speed(handle, device_index, features, &mut controls);
    probe_hires_wheel(handle, device_index, features, &mut controls);
    probe_thumb_wheel(handle, device_index, features, &mut controls);
    probe_smart_shift(handle, device_index, features, &mut controls);
    probe_reprogrammable_controls(handle, device_index, features, &mut controls);
    probe_analog_buttons(handle, device_index, features, &mut controls);
    probe_keyboard_controls(handle, device_index, features, &mut controls);
    probe_battery(handle, device_index, features, &mut controls);
    probe_audio(handle, device_index, features, &mut controls);
    controls.sort_by(|a, b| {
        format!("{:?}", a.group)
            .cmp(&format!("{:?}", b.group))
            .then(a.label.cmp(&b.label))
    });
    controls
}

fn control_feature(device: &DeviceSummary, feature_id: u16) -> Result<u8, String> {
    feature_index(&device.features, feature_id)
        .ok_or_else(|| format!("{} no longer advertises HID++ feature {feature_id:#06x}", device.product))
}

fn scalar(value: &ControlValue, label: &str) -> Result<i64, String> {
    value
        .as_i64()
        .ok_or_else(|| format!("{label} requires a scalar value"))
}

fn boolean(value: &ControlValue, label: &str) -> Result<bool, String> {
    value
        .as_bool()
        .ok_or_else(|| format!("{label} requires an on/off value"))
}

fn request_for_device(
    handle: &HidDevice,
    device: &DeviceSummary,
    feature_id: u16,
    function: u8,
    params: Vec<u8>,
) -> Result<Vec<u8>, String> {
    request(
        handle,
        device.device_index,
        control_feature(device, feature_id)?,
        function,
        params,
    )
}

pub fn set_control_with_handle(
    handle: &HidDevice,
    device: &DeviceSummary,
    control: &DeviceControl,
    value: &ControlValue,
) -> Result<ControlValue, String> {
    if control.provider != ProviderKind::Hidpp {
        return Err(format!("{} is not a HID++ control", control.label));
    }
    if control.backend_id == "dpi" {
        let dpi = u16::try_from(scalar(value, &control.label)?)
            .map_err(|_| format!("invalid DPI value for {}", control.label))?;
        return super::set_dpi_with_handle(handle, device, dpi).map(|state| ControlValue::Int(i64::from(state.current)));
    }

    match control.backend_id.as_str() {
        "report_rate" => {
            let rate = u8::try_from(scalar(value, &control.label)?)
                .map_err(|_| "report rate is outside byte range".to_owned())?;
            request_for_device(handle, device, REPORT_RATE, 2, vec![rate, 0, 0])?;
        }
        "report_rate_extended" => {
            let rate = u8::try_from(scalar(value, &control.label)?)
                .map_err(|_| "extended report rate is outside byte range".to_owned())?;
            request_for_device(handle, device, EXTENDED_REPORT_RATE, 3, vec![rate, 0, 0])?;
        }
        "pointer_speed" => {
            let speed = u16::try_from(scalar(value, &control.label)?)
                .map_err(|_| "pointer speed is outside uint16 range".to_owned())?;
            let [hi, lo] = speed.to_be_bytes();
            request_for_device(handle, device, POINTER_SPEED, 1, vec![hi, lo, 0])?;
        }
        "hires_divert" | "hires_resolution" | "hires_invert" => {
            let enabled = boolean(value, &control.label)?;
            let mut current = request_for_device(handle, device, HIRES_WHEEL, 1, vec![0, 0, 0])?;
            let bit = match control.backend_id.as_str() {
                "hires_divert" => 0x01,
                "hires_resolution" => 0x02,
                _ => 0x04,
            };
            if current.is_empty() { current.push(0); }
            if enabled { current[0] |= bit; } else { current[0] &= !bit; }
            request_for_device(handle, device, HIRES_WHEEL, 2, vec![current[0], 0, 0])?;
        }
        "thumb_divert" | "thumb_invert" => {
            let enabled = boolean(value, &control.label)?;
            let mut current = request_for_device(handle, device, THUMB_WHEEL, 1, vec![0, 0, 0])?;
            current.resize(2, 0);
            let offset = if control.backend_id == "thumb_divert" { 0 } else { 1 };
            if enabled { current[offset] |= 0x01; } else { current[offset] &= !0x01; }
            request_for_device(handle, device, THUMB_WHEEL, 2, vec![current[0], current[1], 0])?;
        }
        "smart_shift" => {
            let threshold = scalar(value, &control.label)?.clamp(1, 50) as u8;
            let wire = if threshold >= 50 { 255 } else { threshold };
            request_for_device(handle, device, SMART_SHIFT, 1, vec![0, wire, 0])?;
        }
        "smart_shift_enhanced" | "smart_shift_torque" => {
            let mut current = request_for_device(handle, device, SMART_SHIFT_ENHANCED, 1, vec![0, 0, 0])?;
            current.resize(3, 0);
            if control.backend_id == "smart_shift_enhanced" {
                current[1] = scalar(value, &control.label)?.clamp(1, 50) as u8;
            } else {
                current[2] = scalar(value, &control.label)?.clamp(1, 100) as u8;
            }
            request_for_device(handle, device, SMART_SHIFT_ENHANCED, 2, current[..3].to_vec())?;
        }
        "fn_inversion" => {
            request_for_device(handle, device, FN_INVERSION, 1, vec![u8::from(boolean(value, &control.label)?), 0, 0])?;
        }
        "new_fn_inversion" => {
            request_for_device(handle, device, NEW_FN_INVERSION, 1, vec![u8::from(boolean(value, &control.label)?), 0, 0])?;
        }
        "gkey_divert" => {
            request_for_device(handle, device, GKEY, 2, vec![u8::from(boolean(value, &control.label)?), 0, 0])?;
            return Ok(value.clone());
        }
        "onboard_profiles" => {
            let payload = if boolean(value, &control.label)? { 0x01 } else { 0x02 };
            request_for_device(handle, device, ONBOARD_PROFILES, 1, vec![payload, 0, 0])?;
        }
        backend if backend.starts_with("reprog:") => {
            let cid = u16::from_str_radix(backend.trim_start_matches("reprog:"), 16)
                .map_err(|_| format!("invalid assignment backend {backend}"))?;
            let target = u16::try_from(scalar(value, &control.label)?)
                .map_err(|_| "assignment target is outside uint16 range".to_owned())?;
            if !control.choices.iter().any(|choice| choice.value == i64::from(target)) {
                return Err(format!("assignment target 0x{target:04X} was not advertised for {}", control.label));
            }
            let [cid_hi, cid_lo] = cid.to_be_bytes();
            let [target_hi, target_lo] = target.to_be_bytes();
            request_for_device(handle, device, REPROG_CONTROLS_V4, 3, vec![cid_hi, cid_lo, 0, target_hi, target_lo])?;
        }
        backend if backend.starts_with("analog:") => {
            let parts = backend.split(':').collect::<Vec<_>>();
            let button = parts.get(1).and_then(|value| value.parse::<u8>().ok())
                .ok_or_else(|| format!("invalid analog backend {backend}"))?;
            let field = *parts.get(2).ok_or_else(|| format!("invalid analog backend {backend}"))?;
            let mut current = request_for_device(handle, device, ANALOG_BUTTONS, 2, vec![button, 0, 0])?;
            if current.len() < 4 { return Err("analog-button response is too short".into()); }
            let logical = scalar(value, &control.label)?.clamp(0, 0x3f) as u8;
            match field {
                "actuation" => current[1] = logical << 2,
                "rapid" => current[2] = (logical << 2) | (current[2] & 0x01),
                "haptics" => current[3] = logical << 2,
                _ => return Err(format!("unsupported analog field {field}")),
            }
            request_for_device(handle, device, ANALOG_BUTTONS, 1, vec![button, current[1], current[2], current[3]])?;
        }
        "sidetone" => {
            let level = scalar(value, &control.label)?.clamp(0, 100) as u8;
            request_for_device(handle, device, SIDETONE, 1, vec![level, 0, 0])?;
        }
        "equalizer" => {
            let values = value
                .as_vector()
                .ok_or_else(|| "equalizer requires a vector".to_owned())?;
            if values.len() > 15 { return Err("equalizer has too many bands".into()); }
            let mut payload = vec![0x02];
            for band in values {
                let signed = i8::try_from(*band).map_err(|_| format!("equalizer band {band} is outside int8 range"))?;
                payload.push(signed as u8);
            }
            request_for_device(handle, device, EQUALIZER, 3, payload)?;
        }
        "headset_mic_gain" => {
            let gain = i8::try_from(scalar(value, &control.label)?)
                .map_err(|_| "microphone gain is outside int8 range".to_owned())?;
            request_for_device(handle, device, HEADSET_MIC_GAIN, 2, vec![gain as u8, 0, 0])?;
        }
        "headset_ai_nr_level" => {
            let level = scalar(value, &control.label)?.clamp(0, 3) as u8;
            request_for_device(handle, device, HEADSET_AI_NOISE_REDUCTION, 3, vec![level, 0, 0])?;
        }
        "headset_sidetone" => {
            let percent = scalar(value, &control.label)?.clamp(0, 100) as u8;
            let version = feature_version(&device.features, HEADSET_AUDIO_SIDETONE);
            let gain_steps = control
                .endpoint
                .split("steps=")
                .nth(1)
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(101)
                .max(2);
            let raw = ((u32::from(percent) * u32::from(gain_steps - 1) + 50) / 100)
                .min(255) as u8;
            let payload = if version > 1 { vec![1, 0xff, raw] } else { vec![1, raw] };
            request_for_device(handle, device, HEADSET_AUDIO_SIDETONE, 1, payload)?;
        }
        backend if backend.starts_with("headset_mic_mute:")
            || backend.starts_with("headset_mic_snr:")
            || backend.starts_with("headset_battery_saver:")
            || backend.starts_with("headset_dnd:")
            || backend.starts_with("headset_ai_nr:") =>
        {
            let parts = backend.split(':').collect::<Vec<_>>();
            let write_fn = parts.get(2).and_then(|value| value.parse::<u8>().ok()).unwrap_or(1);
            let feature = if backend.starts_with("headset_mic_mute:") {
                HEADSET_MIC_MUTE
            } else if backend.starts_with("headset_mic_snr:") {
                HEADSET_MIC_SNR
            } else if backend.starts_with("headset_battery_saver:") {
                HEADSET_BATTERY_SAVER
            } else if backend.starts_with("headset_dnd:") {
                HEADSET_DO_NOT_DISTURB
            } else {
                HEADSET_AI_NOISE_REDUCTION
            };
            request_for_device(handle, device, feature, write_fn, vec![u8::from(boolean(value, &control.label)?), 0, 0])?;
        }
        other => return Err(format!("unsupported HID++ control backend: {other}")),
    }
    read_control_with_handle(handle, device, control).or_else(|_| Ok(value.clone()))
}

pub fn set_control(
    device: &DeviceSummary,
    control: &DeviceControl,
    value: &ControlValue,
) -> Result<ControlValue, String> {
    let handle = open_summary_device(device)?;
    set_control_with_handle(&handle, device, control, value)
}

pub fn read_control_with_handle(
    handle: &HidDevice,
    device: &DeviceSummary,
    control: &DeviceControl,
) -> Result<ControlValue, String> {
    match control.backend_id.as_str() {
        "dpi" => super::read_dpi(handle, device.device_index)
            .map(|state| ControlValue::Int(i64::from(state.current))),
        "report_rate" => request_for_device(handle, device, REPORT_RATE, 1, vec![0, 0, 0])
            .map(|data| ControlValue::Int(i64::from(data.first().copied().unwrap_or(0)))),
        "report_rate_extended" => request_for_device(handle, device, EXTENDED_REPORT_RATE, 2, vec![0, 0, 0])
            .map(|data| ControlValue::Int(i64::from(data.first().copied().unwrap_or(0)))),
        "pointer_speed" => request_for_device(handle, device, POINTER_SPEED, 0, vec![0, 0, 0]).and_then(|data| {
            if data.len() < 2 { Err("pointer-speed response is too short".into()) } else { Ok(ControlValue::Int(i64::from(u16::from_be_bytes([data[0], data[1]])))) }
        }),
        "hires_divert" | "hires_resolution" | "hires_invert" => request_for_device(handle, device, HIRES_WHEEL, 1, vec![0, 0, 0]).map(|data| {
            let bit = if control.backend_id == "hires_divert" { 1 } else if control.backend_id == "hires_resolution" { 2 } else { 4 };
            ControlValue::Bool(data.first().copied().unwrap_or(0) & bit != 0)
        }),
        "thumb_divert" | "thumb_invert" => request_for_device(handle, device, THUMB_WHEEL, 1, vec![0, 0, 0]).map(|data| {
            let offset = if control.backend_id == "thumb_divert" { 0 } else { 1 };
            ControlValue::Bool(data.get(offset).copied().unwrap_or(0) & 1 != 0)
        }),
        "smart_shift" => request_for_device(handle, device, SMART_SHIFT, 0, vec![0, 0, 0]).map(|data| {
            let threshold = if data.first().copied().unwrap_or(0) == 1 { 1 } else { data.get(1).copied().unwrap_or(1).min(50) };
            ControlValue::Int(i64::from(threshold))
        }),
        "smart_shift_enhanced" | "smart_shift_torque" => request_for_device(handle, device, SMART_SHIFT_ENHANCED, 1, vec![0, 0, 0]).map(|data| {
            let offset = if control.backend_id == "smart_shift_torque" { 2 } else { 1 };
            ControlValue::Int(i64::from(data.get(offset).copied().unwrap_or(0)))
        }),
        "fn_inversion" => request_for_device(handle, device, FN_INVERSION, 0, vec![0, 0, 0]).map(|data| ControlValue::Bool(data.first().copied().unwrap_or(0) & 1 != 0)),
        "new_fn_inversion" => request_for_device(handle, device, NEW_FN_INVERSION, 0, vec![0, 0, 0]).map(|data| ControlValue::Bool(data.first().copied().unwrap_or(0) & 1 != 0)),
        "onboard_profiles" => request_for_device(handle, device, ONBOARD_PROFILES, 2, vec![0, 0, 0]).map(|data| ControlValue::Bool(data.first().copied().unwrap_or(0) == 1)),
        backend if backend.starts_with("reprog:") => {
            let cid = u16::from_str_radix(backend.trim_start_matches("reprog:"), 16)
                .map_err(|_| format!("invalid assignment backend {backend}"))?;
            let [hi, lo] = cid.to_be_bytes();
            request_for_device(handle, device, REPROG_CONTROLS_V4, 2, vec![hi, lo, 0]).map(|data| {
                let mapped = if data.len() >= 5 { u16::from_be_bytes([data[3], data[4]]) } else { 0 };
                ControlValue::Int(i64::from(if mapped == 0 { cid } else { mapped }))
            })
        }
        backend if backend.starts_with("analog:") => {
            let parts = backend.split(':').collect::<Vec<_>>();
            let button = parts.get(1).and_then(|value| value.parse::<u8>().ok())
                .ok_or_else(|| format!("invalid analog backend {backend}"))?;
            let field = *parts.get(2).ok_or_else(|| format!("invalid analog backend {backend}"))?;
            request_for_device(handle, device, ANALOG_BUTTONS, 2, vec![button, 0, 0]).and_then(|data| {
                if data.len() < 4 { return Err("analog-button response is too short".into()); }
                let raw = match field { "actuation" => data[1], "rapid" => data[2], "haptics" => data[3], _ => return Err(format!("unsupported analog field {field}")) };
                Ok(ControlValue::Int(i64::from(raw >> 2)))
            })
        }
        "sidetone" => request_for_device(handle, device, SIDETONE, 0, vec![0, 0, 0]).map(|data| ControlValue::Int(i64::from(data.first().copied().unwrap_or(0)))),
        "equalizer" => request_for_device(handle, device, EQUALIZER, 2, vec![0]).map(|data| {
            let count = control.value.as_vector().map(|values| values.len()).unwrap_or(0);
            ControlValue::Vector(data.iter().take(count).map(|value| i64::from(*value as i8)).collect())
        }),
        "headset_mic_gain" => request_for_device(handle, device, HEADSET_MIC_GAIN, 1, vec![0, 0, 0]).map(|data| ControlValue::Int(i64::from(data.first().copied().unwrap_or(0) as i8))),
        "headset_ai_nr_level" => request_for_device(handle, device, HEADSET_AI_NOISE_REDUCTION, 2, vec![0, 0, 0]).map(|data| ControlValue::Int(i64::from(data.first().copied().unwrap_or(0)))),
        backend if backend.starts_with("headset_mic_mute:")
            || backend.starts_with("headset_mic_snr:")
            || backend.starts_with("headset_battery_saver:")
            || backend.starts_with("headset_dnd:")
            || backend.starts_with("headset_ai_nr:") =>
        {
            let parts = backend.split(':').collect::<Vec<_>>();
            let read_fn = parts.get(1).and_then(|value| value.parse::<u8>().ok()).unwrap_or(0);
            let feature = if backend.starts_with("headset_mic_mute:") { HEADSET_MIC_MUTE }
                else if backend.starts_with("headset_mic_snr:") { HEADSET_MIC_SNR }
                else if backend.starts_with("headset_battery_saver:") { HEADSET_BATTERY_SAVER }
                else if backend.starts_with("headset_dnd:") { HEADSET_DO_NOT_DISTURB }
                else { HEADSET_AI_NOISE_REDUCTION };
            request_for_device(handle, device, feature, read_fn, vec![0, 0, 0])
                .map(|data| ControlValue::Bool(data.first().copied().unwrap_or(0) != 0))
        }
        "gkey_divert" | "headset_sidetone" => Ok(control.value.clone()),
        _ if control.readback == ReadbackKind::Telemetry => Ok(control.value.clone()),
        other => Err(format!("no readback implementation for HID++ control {other}")),
    }
}

pub fn read_control(device: &DeviceSummary, control: &DeviceControl) -> Result<ControlValue, String> {
    if control.readback == ReadbackKind::CanonicalWriteOnly || control.readback == ReadbackKind::Telemetry {
        return Ok(control.value.clone());
    }
    let handle = open_summary_device(device)?;
    read_control_with_handle(&handle, device, control)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpi_control_uses_device_choices() {
        let dpi = DpiInfo {
            sensor: 0,
            min: 400,
            max: 1600,
            step: 400,
            current: 800,
            default: 800,
            supported: vec![400, 800, 1600],
        };
        let control = probe_dpi(Some(&dpi)).unwrap();
        assert_eq!(control.kind, ControlKind::Choice);
        assert_eq!(control.value, ControlValue::Int(800));
        assert_eq!(control.choices.len(), 3);
    }

    #[test]
    fn scalar_accepts_bool_and_int() {
        assert_eq!(scalar(&ControlValue::Bool(true), "test").unwrap(), 1);
        assert_eq!(scalar(&ControlValue::Int(12), "test").unwrap(), 12);
    }
}
