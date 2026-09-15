use crate::MouseError;
use forgehx_core::{
    DeviceInfo, DpiConfig, InterfaceSource, KeyBinding, LightingConfig, LightingEffect,
    MouseDeviceState,
};
use hidapi::HidApi;
use std::ffi::CString;

pub const DRIVER_ID: &str = "hyperx-pulsefire-haste-wireless-v1";
pub const HYPERX_VID: u16 = 0x03f0;
pub const HASTE_WIRELESS_PID: u16 = 0x028e;
pub const HASTE_WIRED_PID: u16 = 0x048e;
const PACKET_SIZE: usize = 64;
const HIDAPI_OUTPUT_SIZE: usize = PACKET_SIZE + 1;
const CONFIG_USAGE_PAGE: u16 = 0xff00;
const CONFIG_INTERFACE: i32 = 2;

const CMD_POLLING_RATE: u8 = 0xd0;
const CMD_LED_SETTINGS: u8 = 0xd2;
const CMD_DPI: u8 = 0xd3;
const CMD_BUTTON_ASSIGNMENT: u8 = 0xd4;
const CMD_MACRO_ASSIGNMENT: u8 = 0xd5;
const CMD_MACRO_DATA: u8 = 0xd6;
const CMD_LED_MODE: u8 = 0xd9;
const CMD_LED_EFFECT: u8 = 0xda;
const CMD_SAVE_SETTINGS: u8 = 0xde;

const REPORT_CONNECTION_STATUS: u8 = 0x46;
const REPORT_DEVICE_INFO: u8 = 0x50;
const REPORT_HEARTBEAT: u8 = 0x51;
const REPORT_LED_SETTINGS: u8 = 0x52;
const REPORT_DPI_SETTINGS: u8 = 0x53;
const REPORT_BUTTON_ASSIGNMENTS: u8 = 0x54;

const DPI_SELECTED_PROFILE: u8 = 0x00;
const DPI_ENABLED_PROFILES: u8 = 0x01;
const DPI_VALUE: u8 = 0x02;
const DPI_LIFT_OFF: u8 = 0x05;
const SAVE_ALL: u8 = 0xff;
const SAVE_DPI: u8 = 0x03;

const ACTION_DISABLED: u8 = 0x00;
const ACTION_MOUSE: u8 = 0x01;
const ACTION_KEYBOARD: u8 = 0x02;
const ACTION_MEDIA: u8 = 0x03;
const ACTION_MACRO: u8 = 0x04;
const ACTION_SHORTCUT: u8 = 0x05;
const ACTION_DPI_TOGGLE: u8 = 0x07;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HasteBatteryStatus {
    pub percent: Option<u8>,
    pub charging: Option<bool>,
    pub full: Option<bool>,
    pub wired: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct PulsefireHasteWireless {
    path: String,
}

impl PulsefireHasteWireless {
    pub fn from_device(device: &DeviceInfo) -> Result<Self, MouseError> {
        if device.vendor_id != HYPERX_VID
            || !matches!(device.product_id, HASTE_WIRELESS_PID | HASTE_WIRED_PID)
            || device.protocol.as_deref() != Some(DRIVER_ID)
        {
            return Err(MouseError::Hid(
                "device is not a registered Pulsefire Haste Wireless".into(),
            ));
        }
        let path = select_config_interface(device)
            .or_else(|| discover_config_interface(device.product_id))
            .ok_or_else(|| {
                MouseError::Hid(
                    "Pulsefire Haste configuration HID interface was not found in grouped inventory or direct HID enumeration".into(),
                )
            })?;
        Ok(Self { path })
    }

    pub fn state(&self) -> Result<MouseDeviceState, MouseError> {
        let device = self.open()?;
        drain(&device);
        let settings = request_report(&device, device_settings_query_packet(), REPORT_DEVICE_INFO)?;
        let dpi = request_report(
            &device,
            report_query_packet(REPORT_DPI_SETTINGS),
            REPORT_DPI_SETTINGS,
        )?;
        let buttons = request_report(
            &device,
            report_query_packet(REPORT_BUTTON_ASSIGNMENTS),
            REPORT_BUTTON_ASSIGNMENTS,
        )?;
        let lighting = request_report(
            &device,
            report_query_packet(REPORT_LED_SETTINGS),
            REPORT_LED_SETTINGS,
        )?;
        parse_state_reports(&settings, &dpi, &buttons, &lighting)
    }

    pub fn battery_status(&self) -> Result<HasteBatteryStatus, MouseError> {
        let device = self.open()?;
        drain(&device);
        let response = request_report(
            &device,
            report_query_packet(REPORT_HEARTBEAT),
            REPORT_HEARTBEAT,
        )?;
        parse_battery_status(&response)
            .ok_or_else(|| MouseError::Protocol("invalid Pulsefire Haste heartbeat".into()))
    }

    pub fn connection_status(&self) -> Result<Option<bool>, MouseError> {
        let device = self.open()?;
        drain(&device);
        let response = request_report(
            &device,
            report_query_packet(REPORT_CONNECTION_STATUS),
            REPORT_CONNECTION_STATUS,
        )?;
        Ok(parse_connection_is_wireless(&response))
    }

    pub fn set_polling_rate(&self, hz: u16) -> Result<(), MouseError> {
        let device = self.open()?;
        write_packet(&device, &polling_rate_packet(hz)?)?;
        write_packet(&device, &save_settings_packet(SAVE_ALL))
    }

    pub fn set_dpi(&self, config: &DpiConfig) -> Result<(), MouseError> {
        validate_dpi_config(config)?;
        let device = self.open()?;
        for (stage, dpi) in config.stages.iter().copied().enumerate() {
            write_packet(&device, &dpi_profile_packet(stage, dpi)?)?;
        }
        let enabled_mask = ((1u16 << config.stages.len()) - 1) as u8;
        write_packet(
            &device,
            &dpi_config_packet(DPI_ENABLED_PROFILES, enabled_mask),
        )?;
        write_packet(
            &device,
            &dpi_config_packet(DPI_SELECTED_PROFILE, config.active_stage as u8),
        )?;
        write_packet(&device, &save_settings_packet(SAVE_DPI))
    }

    pub fn set_lift_off_distance(&self, mm: u8) -> Result<(), MouseError> {
        let device = self.open()?;
        write_packet(&device, &lift_off_distance_packet(mm)?)?;
        write_packet(&device, &save_settings_packet(SAVE_DPI))
    }

    pub fn set_button_assignment(&self, button: u32, action: &str) -> Result<(), MouseError> {
        if button >= 6 {
            return Err(MouseError::Invalid(
                "Pulsefire Haste button index must be 0..=5".into(),
            ));
        }
        let parsed = parse_button_action(action)?;
        let device = self.open()?;
        match parsed {
            ParsedButtonAction::Simple { action_type, code } => {
                write_packet(
                    &device,
                    &button_assignment_packet(button as u8, action_type, code),
                )?;
            }
            ParsedButtonAction::Macro {
                modifiers,
                keys,
                delay_ms,
            } => {
                write_packet(
                    &device,
                    &button_assignment_packet(button as u8, ACTION_MACRO, button as u8),
                )?;
                write_packet(
                    &device,
                    &keyboard_macro_packet(button as u8, modifiers, &keys, delay_ms)?,
                )?;
                write_packet(&device, &macro_assignment_packet(button as u8, 2, 0))?;
            }
        }
        write_packet(&device, &save_settings_packet(SAVE_ALL))
    }

    pub fn set_lighting(&self, config: &LightingConfig) -> Result<(), MouseError> {
        let device = self.open()?;
        // Keep the documented 0x52 LED-settings report synchronized with the live
        // editor values before programming the persisted effect packets.
        write_packet(
            &device,
            &direct_led_settings_packet(config.color, config.brightness),
        )?;
        match config.effect {
            LightingEffect::Static => {
                for packet in static_led_effect_packets(config.color, config.brightness) {
                    write_packet(&device, &packet)?;
                }
                write_packet(&device, &led_mode_packet(0x01))?;
            }
            LightingEffect::Breathing => {
                write_packet(
                    &device,
                    &fade_led_settings_packet(config.color, config.brightness, config.speed),
                )?;
                for packet in empty_led_effect_packets(0x01) {
                    write_packet(&device, &packet)?;
                }
                write_packet(&device, &led_mode_packet(0x01))?;
            }
            LightingEffect::Spectrum | LightingEffect::Wave => {
                let mode = cycle_mode(config.speed);
                for packet in spectrum_led_effect_packets(config.brightness, mode) {
                    write_packet(&device, &packet)?;
                }
                write_packet(&device, &led_mode_packet(mode))?;
            }
        }
        write_packet(&device, &save_settings_packet(SAVE_ALL))
    }

    pub fn save_onboard_profile(&self, profile: u8) -> Result<(), MouseError> {
        if profile != 0 {
            return Err(MouseError::Invalid(
                "Pulsefire Haste Wireless exposes exactly one onboard profile (profile 0)".into(),
            ));
        }
        let device = self.open()?;
        write_packet(&device, &save_settings_packet(SAVE_ALL))
    }

    fn open(&self) -> Result<hidapi::HidDevice, MouseError> {
        let api = HidApi::new().map_err(|error| MouseError::Hid(error.to_string()))?;
        let path = CString::new(self.path.as_bytes())
            .map_err(|_| MouseError::Hid("invalid hidraw path".into()))?;
        api.open_path(&path)
            .map_err(|error| MouseError::Hid(error.to_string()))
    }
}

fn is_config_candidate(
    vendor_id: u16,
    product_id: u16,
    interface_number: i32,
    usage_page: u16,
    expected_product_id: u16,
) -> bool {
    vendor_id == HYPERX_VID
        && product_id == expected_product_id
        && matches!(product_id, HASTE_WIRELESS_PID | HASTE_WIRED_PID)
        && interface_number == CONFIG_INTERFACE
        && usage_page == CONFIG_USAGE_PAGE
}

pub fn discover_config_interface(expected_product_id: u16) -> Option<String> {
    if !matches!(expected_product_id, HASTE_WIRELESS_PID | HASTE_WIRED_PID) {
        return None;
    }
    let api = HidApi::new().ok()?;
    let selected_path = api
        .device_list()
        .find(|device| {
            is_config_candidate(
                device.vendor_id(),
                device.product_id(),
                device.interface_number(),
                device.usage_page(),
                expected_product_id,
            )
        })
        .or_else(|| {
            api.device_list().find(|device| {
                device.vendor_id() == HYPERX_VID
                    && device.product_id() == expected_product_id
                    && matches!(device.product_id(), HASTE_WIRELESS_PID | HASTE_WIRED_PID)
                    && device.interface_number() == CONFIG_INTERFACE
            })
        })
        .map(|device| device.path().to_string_lossy().into_owned());
    selected_path
}

pub fn select_config_interface(device: &DeviceInfo) -> Option<String> {
    if device.vendor_id != HYPERX_VID
        || !matches!(device.product_id, HASTE_WIRELESS_PID | HASTE_WIRED_PID)
    {
        return None;
    }
    let is_haste_hid = |interface: &&forgehx_core::DeviceInterface| {
        interface.source == InterfaceSource::Hid
            && interface.vendor_id == Some(HYPERX_VID)
            && matches!(
                interface.product_id,
                Some(HASTE_WIRELESS_PID | HASTE_WIRED_PID)
            )
    };
    device
        .interfaces
        .iter()
        .filter(is_haste_hid)
        .find(|interface| {
            interface.interface_number == Some(CONFIG_INTERFACE)
                && interface.usage_page == Some(CONFIG_USAGE_PAGE)
        })
        .or_else(|| {
            device
                .interfaces
                .iter()
                .filter(is_haste_hid)
                .find(|interface| interface.interface_number == Some(CONFIG_INTERFACE))
        })
        .map(|interface| interface.path.clone())
}

pub fn polling_rate_packet(hz: u16) -> Result<[u8; PACKET_SIZE], MouseError> {
    let rate_index = polling_rate_index(hz)?;
    let mut packet = [0u8; PACKET_SIZE];
    packet[0] = CMD_POLLING_RATE;
    packet[3] = 1;
    packet[4] = rate_index;
    Ok(packet)
}

pub fn dpi_profile_packet(stage: usize, dpi: u16) -> Result<[u8; PACKET_SIZE], MouseError> {
    if stage >= 5 {
        return Err(MouseError::Invalid(
            "Pulsefire Haste supports at most five DPI stages".into(),
        ));
    }
    if !(200..=16000).contains(&dpi) || !dpi.is_multiple_of(100) {
        return Err(MouseError::Invalid(
            "Pulsefire Haste DPI must be 200..=16000 in 100-DPI steps".into(),
        ));
    }
    let steps = dpi / 100;
    let mut packet = [0u8; PACKET_SIZE];
    packet[0] = CMD_DPI;
    packet[1] = DPI_VALUE;
    packet[2] = stage as u8;
    packet[3] = 2;
    packet[4..6].copy_from_slice(&steps.to_le_bytes());
    Ok(packet)
}

pub fn lift_off_distance_packet(mm: u8) -> Result<[u8; PACKET_SIZE], MouseError> {
    if !matches!(mm, 1 | 2) {
        return Err(MouseError::Invalid(
            "Pulsefire Haste lift-off distance must be 1 mm or 2 mm".into(),
        ));
    }
    let mut packet = [0u8; PACKET_SIZE];
    packet[0] = CMD_DPI;
    packet[1] = DPI_LIFT_OFF;
    packet[3] = 1;
    packet[4] = mm;
    packet[5] = mm;
    Ok(packet)
}

pub fn button_assignment_packet(button: u8, action_type: u8, code: u8) -> [u8; PACKET_SIZE] {
    let mut packet = [0u8; PACKET_SIZE];
    packet[0] = CMD_BUTTON_ASSIGNMENT;
    packet[1] = button;
    packet[2] = action_type;
    packet[3] = 2;
    packet[4] = code;
    packet
}

pub fn parse_battery_status(packet: &[u8]) -> Option<HasteBatteryStatus> {
    if packet.len() < 6 || packet[0] != REPORT_HEARTBEAT {
        return None;
    }
    let state = packet[5];
    Some(HasteBatteryStatus {
        percent: (packet[4] <= 100).then_some(packet[4]),
        charging: match state {
            0x00 => Some(false),
            0x01 | 0x02 => Some(true),
            _ => None,
        },
        full: match state {
            0x02 => Some(true),
            0x00 | 0x01 => Some(false),
            _ => None,
        },
        wired: match state {
            0x00 => Some(false),
            0x01 | 0x02 => Some(true),
            _ => None,
        },
    })
}

pub fn parse_state_reports(
    settings: &[u8],
    dpi: &[u8],
    buttons: &[u8],
    lighting: &[u8],
) -> Result<MouseDeviceState, MouseError> {
    if settings.len() < 54 || settings[0] != REPORT_DEVICE_INFO || settings[1] != 0x03 {
        return Err(MouseError::Protocol(
            "Pulsefire Haste device-settings report is malformed".into(),
        ));
    }
    if dpi.len() < 38 || dpi[0] != REPORT_DPI_SETTINGS {
        return Err(MouseError::Protocol(
            "Pulsefire Haste DPI report is malformed".into(),
        ));
    }
    if buttons.len() < 22 || buttons[0] != REPORT_BUTTON_ASSIGNMENTS {
        return Err(MouseError::Protocol(
            "Pulsefire Haste button report is malformed".into(),
        ));
    }
    let lighting = parse_lighting_report(lighting)?;

    let enabled = dpi[5];
    let active_raw = dpi[4] as usize;
    let mut stages = Vec::new();
    let mut active = None;
    for stage in 0..5usize {
        let start = 10 + stage * 2;
        let step = u16::from_le_bytes([settings[start], settings[start + 1]]);
        if enabled & (1 << stage) != 0 {
            if stage == active_raw {
                active = Some(stages.len());
            }
            stages.push(step.saturating_mul(100));
        }
    }

    let button_assignments = (0..6usize)
        .map(|button| {
            let base = 4 + button * 3;
            KeyBinding {
                key: button.to_string(),
                action: describe_button_action(buttons[base], buttons[base + 1]),
            }
        })
        .collect();

    Ok(MouseDeviceState {
        backend_device_id: DRIVER_ID.into(),
        active_profile: Some(0),
        dpi_stages: stages,
        active_dpi_stage: active,
        report_rate_hz: polling_rate_from_index(settings[53]),
        button_count: Some(6),
        button_assignments,
        lift_off_distance_mm: matches!(dpi[37], 1 | 2).then_some(dpi[37]),
        lighting: Some(lighting),
    })
}

pub fn parse_lighting_report(packet: &[u8]) -> Result<LightingConfig, MouseError> {
    if packet.len() < 14 || packet[0] != REPORT_LED_SETTINGS {
        return Err(MouseError::Protocol(
            "Pulsefire Haste LED-settings report is malformed".into(),
        ));
    }
    Ok(LightingConfig {
        // The documented 0x52 report exposes accepted RGB/brightness but does not
        // report the current effect mode or speed. ForgeHX preserves those two
        // editor fields while reconciling the values the hardware can read back.
        effect: LightingEffect::Static,
        color: [packet[8], packet[9], packet[10]],
        brightness: packet[7].min(100),
        speed: 0,
    })
}

fn device_settings_query_packet() -> [u8; PACKET_SIZE] {
    let mut packet = report_query_packet(REPORT_DEVICE_INFO);
    packet[1] = 0x03;
    packet
}

fn report_query_packet(report: u8) -> [u8; PACKET_SIZE] {
    let mut packet = [0u8; PACKET_SIZE];
    packet[0] = report;
    packet
}

fn request_report(
    device: &hidapi::HidDevice,
    request: [u8; PACKET_SIZE],
    expected: u8,
) -> Result<[u8; PACKET_SIZE], MouseError> {
    write_packet(device, &request)?;
    for _ in 0..12 {
        let mut response = [0u8; PACKET_SIZE];
        let length = device
            .read_timeout(&mut response, 80)
            .map_err(|error| MouseError::Hid(error.to_string()))?;
        if length == 0 {
            continue;
        }
        if response[0] == expected {
            return Ok(response);
        }
    }
    Err(MouseError::Protocol(format!(
        "Pulsefire Haste report 0x{expected:02x} timed out"
    )))
}

fn parse_connection_is_wireless(packet: &[u8]) -> Option<bool> {
    if packet.len() < 4 || packet[0] != REPORT_CONNECTION_STATUS {
        return None;
    }
    match packet[3] {
        0x01 => Some(true),
        0x02 => Some(false),
        _ => None,
    }
}

fn polling_rate_index(hz: u16) -> Result<u8, MouseError> {
    match hz {
        125 => Ok(0),
        250 => Ok(1),
        500 => Ok(2),
        1000 => Ok(3),
        _ => Err(MouseError::Invalid(
            "Pulsefire Haste polling rate must be 125, 250, 500, or 1000 Hz".into(),
        )),
    }
}

fn polling_rate_from_index(index: u8) -> Option<u16> {
    match index {
        0 => Some(125),
        1 => Some(250),
        2 => Some(500),
        3 => Some(1000),
        _ => None,
    }
}

fn dpi_config_packet(kind: u8, value: u8) -> [u8; PACKET_SIZE] {
    let mut packet = [0u8; PACKET_SIZE];
    packet[0] = CMD_DPI;
    packet[1] = kind;
    packet[3] = 1;
    packet[4] = value;
    packet
}

fn save_settings_packet(save_type: u8) -> [u8; PACKET_SIZE] {
    let mut packet = [0u8; PACKET_SIZE];
    packet[0] = CMD_SAVE_SETTINGS;
    packet[1] = save_type;
    packet
}

fn validate_dpi_config(config: &DpiConfig) -> Result<(), MouseError> {
    if config.stages.is_empty()
        || config.stages.len() > 5
        || config.active_stage >= config.stages.len()
    {
        return Err(MouseError::Invalid(
            "Pulsefire Haste requires 1..=5 DPI stages and a valid active stage".into(),
        ));
    }
    for (stage, dpi) in config.stages.iter().copied().enumerate() {
        dpi_profile_packet(stage, dpi)?;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedButtonAction {
    Simple {
        action_type: u8,
        code: u8,
    },
    Macro {
        modifiers: u8,
        keys: Vec<u8>,
        delay_ms: u16,
    },
}

fn parse_button_action(action: &str) -> Result<ParsedButtonAction, MouseError> {
    let action = action.trim().to_ascii_lowercase();
    if action == "disabled" {
        return Ok(ParsedButtonAction::Simple {
            action_type: ACTION_DISABLED,
            code: 0,
        });
    }
    if action == "dpi-toggle" || action == "dpi_toggle" {
        return Ok(ParsedButtonAction::Simple {
            action_type: ACTION_DPI_TOGGLE,
            code: 0x08,
        });
    }
    if let Some(value) = action.strip_prefix("mouse:") {
        let code = match value {
            "left" => 0x01,
            "right" => 0x02,
            "middle" => 0x03,
            "back" => 0x04,
            "forward" => 0x05,
            _ => {
                return Err(MouseError::Invalid(
                    "mouse action must be left/right/middle/back/forward".into(),
                ))
            }
        };
        return Ok(ParsedButtonAction::Simple {
            action_type: ACTION_MOUSE,
            code,
        });
    }
    if let Some(value) = action.strip_prefix("key:") {
        return Ok(ParsedButtonAction::Simple {
            action_type: ACTION_KEYBOARD,
            code: parse_u8_code(value)?,
        });
    }
    if let Some(value) = action.strip_prefix("media:") {
        let code = match value {
            "play" | "play-pause" | "play_pause" => 0x00,
            "stop" => 0x01,
            "previous" | "prev" => 0x02,
            "next" => 0x03,
            "mute" => 0x04,
            "volume-down" | "volume_down" => 0x05,
            "volume-up" | "volume_up" => 0x06,
            _ => parse_u8_code(value)?,
        };
        return Ok(ParsedButtonAction::Simple {
            action_type: ACTION_MEDIA,
            code,
        });
    }
    if let Some(value) = action.strip_prefix("shortcut:") {
        let code = match value {
            "task-manager" | "task_manager" => 0x01,
            "system-utility" | "system_utility" => 0x02,
            "show-desktop" | "show_desktop" => 0x03,
            "cycle-apps" | "cycle_apps" => 0x04,
            "close-window" | "close_window" => 0x05,
            "cut" => 0x06,
            "copy" => 0x07,
            "paste" => 0x08,
            _ => parse_u8_code(value)?,
        };
        return Ok(ParsedButtonAction::Simple {
            action_type: ACTION_SHORTCUT,
            code,
        });
    }
    if let Some(value) = action.strip_prefix("macro:") {
        return parse_macro(value);
    }
    Err(MouseError::Invalid(
        "button action must use disabled, dpi-toggle, mouse:<action>, key:<HID>, media:<action>, shortcut:<action>, or macro:<chord>".into(),
    ))
}

fn parse_macro(value: &str) -> Result<ParsedButtonAction, MouseError> {
    let (chord, delay_ms) = if let Some((chord, delay)) = value.split_once(";delay=") {
        let delay = delay.parse::<u16>().map_err(|_| {
            MouseError::Invalid("macro delay must be an integer number of milliseconds".into())
        })?;
        (chord, delay)
    } else {
        (value, 20)
    };
    let mut modifiers = 0u8;
    let mut keys = Vec::new();
    for token in chord
        .split('+')
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        let bit = match token {
            "lctrl" | "ctrl" => Some(0x01),
            "lshift" | "shift" => Some(0x02),
            "lalt" | "alt" => Some(0x04),
            "lwin" | "win" | "meta" => Some(0x08),
            "rctrl" => Some(0x10),
            "rshift" => Some(0x20),
            "ralt" => Some(0x40),
            "rwin" => Some(0x80),
            _ => None,
        };
        if let Some(bit) = bit {
            modifiers |= bit;
        } else {
            keys.push(parse_u8_code(token)?);
        }
    }
    if keys.is_empty() || keys.len() > 6 {
        return Err(MouseError::Invalid(
            "macro chord must contain 1..=6 HID key usage codes".into(),
        ));
    }
    Ok(ParsedButtonAction::Macro {
        modifiers,
        keys,
        delay_ms,
    })
}

fn parse_u8_code(value: &str) -> Result<u8, MouseError> {
    let value = value.trim();
    let parsed = if let Some(hex) = value.strip_prefix("0x") {
        u8::from_str_radix(hex, 16)
    } else {
        value.parse::<u8>()
    };
    parsed.map_err(|_| MouseError::Invalid(format!("invalid HID/action code: {value}")))
}

fn describe_button_action(action_type: u8, code: u8) -> String {
    match action_type {
        ACTION_DISABLED => "disabled".into(),
        ACTION_MOUSE => match code {
            0x01 => "mouse:left".into(),
            0x02 => "mouse:right".into(),
            0x03 => "mouse:middle".into(),
            0x04 => "mouse:back".into(),
            0x05 => "mouse:forward".into(),
            _ => format!("mouse:0x{code:02x}"),
        },
        ACTION_KEYBOARD => format!("key:0x{code:02x}"),
        ACTION_MEDIA => match code {
            0x00 => "media:play-pause".into(),
            0x01 => "media:stop".into(),
            0x02 => "media:previous".into(),
            0x03 => "media:next".into(),
            0x04 => "media:mute".into(),
            0x05 => "media:volume-down".into(),
            0x06 => "media:volume-up".into(),
            _ => format!("media:0x{code:02x}"),
        },
        ACTION_MACRO => format!("macro:stored:{code}"),
        ACTION_SHORTCUT => match code {
            0x01 => "shortcut:task-manager".into(),
            0x02 => "shortcut:system-utility".into(),
            0x03 => "shortcut:show-desktop".into(),
            0x04 => "shortcut:cycle-apps".into(),
            0x05 => "shortcut:close-window".into(),
            0x06 => "shortcut:cut".into(),
            0x07 => "shortcut:copy".into(),
            0x08 => "shortcut:paste".into(),
            _ => format!("shortcut:0x{code:02x}"),
        },
        ACTION_DPI_TOGGLE => "dpi-toggle".into(),
        _ => format!("unknown:0x{action_type:02x}:0x{code:02x}"),
    }
}

fn keyboard_macro_packet(
    button: u8,
    modifiers: u8,
    keys: &[u8],
    delay_ms: u16,
) -> Result<[u8; PACKET_SIZE], MouseError> {
    if keys.is_empty() || keys.len() > 6 {
        return Err(MouseError::Invalid(
            "Pulsefire Haste macro packet requires 1..=6 keys".into(),
        ));
    }
    let mut packet = [0u8; PACKET_SIZE];
    packet[0] = CMD_MACRO_DATA;
    packet[1] = button;
    packet[2] = 0;
    packet[3] = 2;
    packet[4] = 0x1a;
    packet[5] = modifiers;
    packet[6..6 + keys.len()].copy_from_slice(keys);
    packet[12..14].copy_from_slice(&delay_ms.to_le_bytes());
    packet[14] = 0x1a;
    packet[22..24].copy_from_slice(&delay_ms.to_le_bytes());
    Ok(packet)
}

fn macro_assignment_packet(button: u8, event_count: u8, repeat_mode: u8) -> [u8; PACKET_SIZE] {
    let mut packet = [0u8; PACKET_SIZE];
    packet[0] = CMD_MACRO_ASSIGNMENT;
    packet[1] = button;
    packet[3] = 5;
    packet[4] = event_count;
    packet[6] = repeat_mode;
    packet
}

fn static_led_effect_packets(color: [u8; 3], brightness: u8) -> Vec<[u8; PACKET_SIZE]> {
    let adjusted = adjusted_color(color, brightness);
    let mut packets = empty_led_effect_packets(0x01);
    packets[0][4..7].copy_from_slice(&adjusted);
    packets
}

fn empty_led_effect_packets(mode: u8) -> Vec<[u8; PACKET_SIZE]> {
    (0..6u8)
        .map(|index| {
            let mut packet = [0u8; PACKET_SIZE];
            packet[0] = CMD_LED_EFFECT;
            packet[1] = mode;
            packet[2] = index;
            packet[3] = 60;
            packet
        })
        .collect()
}

fn spectrum_led_effect_packets(brightness: u8, mode: u8) -> Vec<[u8; PACKET_SIZE]> {
    let mut colors = Vec::with_capacity(120);
    for index in 0..120u16 {
        let phase = index * 6;
        let sector = phase / 120;
        let offset = ((phase % 120) * 255 / 120) as u8;
        let color = match sector {
            0 => [255, offset, 0],
            1 => [255u8.saturating_sub(offset), 255, 0],
            2 => [0, 255, offset],
            3 => [0, 255u8.saturating_sub(offset), 255],
            4 => [offset, 0, 255],
            _ => [255, 0, 255u8.saturating_sub(offset)],
        };
        colors.push(adjusted_color(color, brightness));
    }
    (0..6usize)
        .map(|packet_index| {
            let mut packet = [0u8; PACKET_SIZE];
            packet[0] = CMD_LED_EFFECT;
            packet[1] = mode;
            packet[2] = packet_index as u8;
            packet[3] = 60;
            for color_index in 0..20usize {
                let color = colors[packet_index * 20 + color_index];
                let start = 4 + color_index * 3;
                packet[start..start + 3].copy_from_slice(&color);
            }
            packet
        })
        .collect()
}

fn direct_led_settings_packet(color: [u8; 3], brightness: u8) -> [u8; PACKET_SIZE] {
    let mut packet = [0u8; PACKET_SIZE];
    packet[0] = CMD_LED_SETTINGS;
    packet[3] = 8;
    packet[4..7].copy_from_slice(&color);
    packet[10] = brightness.min(100);
    packet
}

fn fade_led_settings_packet(color: [u8; 3], brightness: u8, speed: u8) -> [u8; PACKET_SIZE] {
    let mut packet = [0u8; PACKET_SIZE];
    packet[0] = CMD_LED_SETTINGS;
    packet[2] = 0x30;
    packet[3] = 8;
    packet[4..7].copy_from_slice(&color);
    packet[10] = brightness.min(100);
    packet[11] = speed.min(100);
    packet
}

fn led_mode_packet(mode: u8) -> [u8; PACKET_SIZE] {
    let mut packet = [0u8; PACKET_SIZE];
    packet[0] = CMD_LED_MODE;
    packet[3] = 3;
    packet[4] = 0x55;
    packet[5] = mode;
    packet[6] = 0x23;
    packet
}

fn cycle_mode(speed: u8) -> u8 {
    let speed = u16::from(speed.min(100));
    (120u16.saturating_sub((speed * 2) / 5)) as u8
}

fn adjusted_color(color: [u8; 3], brightness: u8) -> [u8; 3] {
    let brightness = u16::from(brightness.min(100));
    color.map(|value| ((u16::from(value) * brightness + 50) / 100) as u8)
}

fn hidapi_output_frame(packet: &[u8; PACKET_SIZE]) -> [u8; HIDAPI_OUTPUT_SIZE] {
    // HIDAPI requires byte 0 to be the Report ID. This Haste interface has
    // unnumbered 64-byte reports, so the Report ID is 0 and the protocol
    // payload begins at byte 1.
    let mut frame = [0u8; HIDAPI_OUTPUT_SIZE];
    frame[0] = 0;
    frame[1..].copy_from_slice(packet);
    frame
}

fn write_packet(device: &hidapi::HidDevice, packet: &[u8; PACKET_SIZE]) -> Result<(), MouseError> {
    let frame = hidapi_output_frame(packet);
    let written = device
        .write(&frame)
        .map_err(|error| MouseError::Hid(error.to_string()))?;
    if written != HIDAPI_OUTPUT_SIZE {
        return Err(MouseError::Hid(format!(
            "short Pulsefire Haste HID write: {written}/{HIDAPI_OUTPUT_SIZE}"
        )));
    }
    Ok(())
}

fn drain(device: &hidapi::HidDevice) {
    loop {
        let mut buf = [0u8; PACKET_SIZE];
        match device.read_timeout(&mut buf, 0) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forgehx_core::{
        Capability, ConnectionKind, DeviceClass, DeviceId, DeviceInterface, SupportLevel,
        VendorFamily,
    };

    #[test]
    fn hidapi_frame_prefixes_zero_report_id_without_shifting_payload() {
        let mut packet = [0u8; PACKET_SIZE];
        packet[0] = CMD_DPI;
        packet[1] = DPI_VALUE;
        packet[63] = 0xa5;
        let frame = hidapi_output_frame(&packet);
        assert_eq!(frame.len(), 65);
        assert_eq!(frame[0], 0);
        assert_eq!(&frame[1..], &packet);
    }

    fn device_with_interfaces() -> DeviceInfo {
        DeviceInfo {
            id: DeviceId("haste".into()),
            name: "HyperX Pulsefire Haste Wireless".into(),
            manufacturer: Some("HP, Inc".into()),
            vendor_id: HYPERX_VID,
            product_id: HASTE_WIRELESS_PID,
            serial: None,
            vendor_family: VendorFamily::HyperX,
            device_class: DeviceClass::Mouse,
            support_level: SupportLevel::FullySupported,
            connection_kind: ConnectionKind::Hid,
            interfaces: vec![
                DeviceInterface {
                    source: InterfaceSource::Hid,
                    path: "/dev/hidraw0".into(),
                    vendor_id: Some(HYPERX_VID),
                    product_id: Some(HASTE_WIRELESS_PID),
                    interface_number: Some(0),
                    usage_page: Some(0x01),
                    usage: Some(0x02),
                    audio_node_id: None,
                    usb_parent: None,
                },
                DeviceInterface {
                    source: InterfaceSource::Hid,
                    path: "/dev/hidraw2".into(),
                    vendor_id: Some(HYPERX_VID),
                    product_id: Some(HASTE_WIRELESS_PID),
                    interface_number: Some(2),
                    usage_page: Some(CONFIG_USAGE_PAGE),
                    usage: Some(0x01),
                    audio_node_id: None,
                    usb_parent: None,
                },
            ],
            capabilities: vec![
                Capability::Diagnostics,
                Capability::Lighting,
                Capability::Dpi,
                Capability::PollingRate,
                Capability::Bindings,
                Capability::BatteryStatus,
            ],
            generic_capabilities: vec![],
            capability_owners: vec![],
            battery_percent: None,
            battery_state: None,
            protocol: Some(DRIVER_ID.into()),
        }
    }

    #[test]
    fn polling_packet_uses_documented_rate_index() {
        assert_eq!(
            &polling_rate_packet(1000).unwrap()[..5],
            &[0xd0, 0, 0, 1, 3]
        );
        assert!(polling_rate_packet(2000).is_err());
    }

    #[test]
    fn dpi_packet_encodes_hundred_dpi_steps_little_endian() {
        assert_eq!(
            &dpi_profile_packet(2, 3200).unwrap()[..6],
            &[0xd3, 0x02, 2, 2, 32, 0]
        );
        assert!(dpi_profile_packet(5, 800).is_err());
        assert!(dpi_profile_packet(0, 850).is_err());
    }

    #[test]
    fn selects_vendor_configuration_interface() {
        assert_eq!(
            select_config_interface(&device_with_interfaces()).as_deref(),
            Some("/dev/hidraw2")
        );
    }

    #[test]
    fn direct_candidate_requires_exact_haste_interface_metadata() {
        assert!(is_config_candidate(
            HYPERX_VID,
            HASTE_WIRED_PID,
            2,
            CONFIG_USAGE_PAGE,
            HASTE_WIRED_PID
        ));
        assert!(!is_config_candidate(
            HYPERX_VID,
            HASTE_WIRED_PID,
            1,
            CONFIG_USAGE_PAGE,
            HASTE_WIRED_PID
        ));
        assert!(!is_config_candidate(
            HYPERX_VID,
            HASTE_WIRED_PID,
            2,
            0x0001,
            HASTE_WIRED_PID
        ));
        assert!(!is_config_candidate(
            HYPERX_VID,
            HASTE_WIRELESS_PID,
            2,
            CONFIG_USAGE_PAGE,
            HASTE_WIRED_PID
        ));
    }

    #[test]
    fn parses_documented_heartbeat() {
        let status = parse_battery_status(&[0x51, 0, 0, 9, 76, 1]).unwrap();
        assert_eq!(status.percent, Some(76));
        assert_eq!(status.charging, Some(true));
        assert_eq!(status.wired, Some(true));
    }

    #[test]
    fn parses_documented_mouse_state_reports() {
        let mut settings = [0u8; PACKET_SIZE];
        settings[0] = 0x50;
        settings[1] = 0x03;
        settings[10..12].copy_from_slice(&4u16.to_le_bytes());
        settings[12..14].copy_from_slice(&8u16.to_le_bytes());
        settings[14..16].copy_from_slice(&16u16.to_le_bytes());
        settings[16..18].copy_from_slice(&32u16.to_le_bytes());
        settings[18..20].copy_from_slice(&64u16.to_le_bytes());
        settings[53] = 3;
        let mut dpi = [0u8; PACKET_SIZE];
        dpi[0] = 0x53;
        dpi[4] = 2;
        dpi[5] = 0b0_1111;
        dpi[37] = 1;
        let mut buttons = [0u8; PACKET_SIZE];
        buttons[0] = 0x54;
        buttons[4..7].copy_from_slice(&[ACTION_MOUSE, 1, 1]);
        buttons[7..10].copy_from_slice(&[ACTION_MOUSE, 2, 2]);
        buttons[10..13].copy_from_slice(&[ACTION_MOUSE, 3, 4]);
        buttons[13..16].copy_from_slice(&[ACTION_MOUSE, 4, 8]);
        buttons[16..19].copy_from_slice(&[ACTION_MOUSE, 5, 16]);
        buttons[19..22].copy_from_slice(&[ACTION_DPI_TOGGLE, 8, 8]);
        let mut lighting = [0u8; PACKET_SIZE];
        lighting[0] = REPORT_LED_SETTINGS;
        lighting[7] = 67;
        lighting[8..11].copy_from_slice(&[12, 34, 56]);
        let state = parse_state_reports(&settings, &dpi, &buttons, &lighting).unwrap();
        assert_eq!(state.dpi_stages, vec![400, 800, 1600, 3200]);
        assert_eq!(state.active_dpi_stage, Some(2));
        assert_eq!(state.report_rate_hz, Some(1000));
        assert_eq!(state.lift_off_distance_mm, Some(1));
        assert_eq!(state.button_assignments[5].action, "dpi-toggle");
        assert_eq!(state.lighting.as_ref().unwrap().color, [12, 34, 56]);
        assert_eq!(state.lighting.as_ref().unwrap().brightness, 67);
    }

    #[test]
    fn button_packets_cover_native_assignment_types() {
        assert_eq!(
            &button_assignment_packet(4, ACTION_MOUSE, 5)[..6],
            &[0xd4, 4, 1, 2, 5, 0]
        );
        assert_eq!(
            parse_button_action("shortcut:paste").unwrap(),
            ParsedButtonAction::Simple {
                action_type: ACTION_SHORTCUT,
                code: 8
            }
        );
        assert!(parse_button_action("mouse:teleport").is_err());
    }

    #[test]
    fn lift_off_packet_is_bounded_to_documented_values() {
        assert_eq!(
            &lift_off_distance_packet(2).unwrap()[..6],
            &[0xd3, 5, 0, 1, 2, 2]
        );
        assert!(lift_off_distance_packet(3).is_err());
    }

    #[test]
    fn direct_lighting_packet_matches_documented_led_settings_report_fields() {
        let packet = direct_led_settings_packet([12, 34, 56], 67);
        assert_eq!(&packet[..12], &[0xd2, 0, 0, 8, 12, 34, 56, 0, 0, 0, 67, 0]);
    }

    #[test]
    fn static_lighting_uses_six_documented_effect_packets() {
        let packets = static_led_effect_packets([100, 50, 25], 50);
        assert_eq!(packets.len(), 6);
        assert_eq!(&packets[0][..7], &[0xda, 1, 0, 60, 50, 25, 13]);
        assert_eq!(packets[5][2], 5);
    }
}
