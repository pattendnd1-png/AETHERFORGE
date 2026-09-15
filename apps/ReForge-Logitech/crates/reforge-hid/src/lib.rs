mod control;
pub mod session;

use hidapi::{BusType, DeviceInfo, HidApi, HidDevice};
use reforge_core::{
    BrightnessInfo, DeviceClass, DeviceSummary, DpiInfo, FeatureInfo, LightingCapabilities, LightingEffect,
    LightingState, LightingZone, ProviderKind, RgbColor, TransportKind,
};
use reforge_protocol::{
    dpi::{
        build_set_dpi_request, parse_dpi_range, parse_dpi_state, parse_dpi_values,
        validate_dpi_value,
    },
    feature::{
        feature_name, parse_feature_index, parse_feature_set_count, parse_feature_set_entry,
        ADJUSTABLE_DPI, BACKLIGHT2, BRIGHTNESS_CONTROL, COLOR_LED_EFFECTS, DEVICE_FW_VERSION, FEATURE_SET,
        ONBOARD_PROFILES, PER_KEY_LIGHTING_V2, RGB_EFFECTS,
    },
    lighting::{
        LedEffectParameters, backlight2_info_request, backlight2_range_request,
        backlight2_write_request, brightness_info_request, brightness_power_request,
        brightness_read_request, brightness_write_request, color_led_control_request,
        color_led_info_request, color_led_set_effect_request, effect_info_request,
        encode_effect_parameters, onboard_profile_mode_request, parse_backlight2_info,
        parse_backlight2_range, parse_brightness_range, parse_brightness_value, parse_effect_probe,
        parse_led_protocol_info,
        parse_per_key_bitmap_page, parse_zone_probe, per_key_bitmap_request,
        per_key_frame_end_request, per_key_set_uniform_request, rgb_info_request,
        rgb_set_effect_request, rgb_sw_control_request, rgb_zone_info_request, zone_info_request,
    },
    HidppRequest, HidppResponse,
};
use std::{
    collections::{BTreeMap, HashSet},
    ffi::CString,
    thread,
    time::{Duration, Instant},
};

pub use control::{read_control, read_control_with_handle, set_control, set_control_with_handle};
pub use session::{classify_packet, classify_response, HidppNotification, HidppPacketKind, SessionManager};

pub const LOGITECH_VENDOR_ID: u16 = 0x046d;
const TRANSACTION_TIMEOUT_MS: u64 = 750;
const READ_SLICE_MS: i32 = 75;
const RETRY_BACKOFF_MS: [u64; 3] = [30, 60, 90];

pub fn probe_indices() -> [u8; 7] {
    [0xff, 1, 2, 3, 4, 5, 6]
}

pub fn feature_indices(count: u8) -> Vec<u8> {
    (1..=count).collect()
}

pub fn response_matches(request: &HidppRequest, response: &HidppResponse) -> bool {
    if response.device_index != request.device_index {
        return false;
    }
    if response.is_hidpp_error() {
        return response.function_swid == request.feature_index
            && response.params.first().copied() == Some(request.function_swid());
    }
    response.feature_index == request.feature_index
        && response.function_swid == request.function_swid()
}

fn hidpp_error_message(response: &HidppResponse) -> String {
    let code = response.params.get(1).copied().unwrap_or(0xff);
    let label = match code {
        0x01 => "unknown error",
        0x02 => "invalid argument",
        0x03 => "out of range",
        0x04 => "hardware error",
        0x05 => "internal error",
        0x06 => "invalid feature index",
        0x07 => "invalid function",
        0x08 => "busy",
        0x09 => "unsupported operation",
        _ => "unrecognized HID++ error",
    };
    format!("{label} ({code:#04x})")
}

fn transact(device: &HidDevice, request: &HidppRequest) -> Result<HidppResponse, String> {
    let report = request.encode();
    let written = device
        .write(&report)
        .map_err(|error| format!("HID++ write failed: {error}"))?;
    if written != report.len() {
        return Err(format!(
            "short HID++ write: wrote {written} of {} bytes",
            report.len()
        ));
    }

    let started = Instant::now();
    let mut buffer = [0_u8; 64];
    while started.elapsed().as_millis() < u128::from(TRANSACTION_TIMEOUT_MS) {
        let read = device
            .read_timeout(&mut buffer, READ_SLICE_MS)
            .map_err(|error| format!("HID++ read failed: {error}"))?;
        if read == 0 {
            continue;
        }
        let response = match HidppResponse::parse(&buffer[..read]) {
            Ok(response) => response,
            Err(_) => continue,
        };
        if !response_matches(request, &response) {
            continue;
        }
        if response.is_hidpp_error() {
            return Err(format!(
                "HID++ request feature {:#04x} function {} failed: {}",
                request.feature_index,
                request.function,
                hidpp_error_message(&response)
            ));
        }
        return Ok(response);
    }
    Err(format!(
        "timed out waiting for HID++ response (device index {:#04x}, feature index {:#04x})",
        request.device_index, request.feature_index
    ))
}

fn transact_retry(device: &HidDevice, request: &HidppRequest) -> Result<HidppResponse, String> {
    let mut last_error = String::new();
    for attempt in 0..=RETRY_BACKOFF_MS.len() {
        match transact(device, request) {
            Ok(response) => return Ok(response),
            Err(error) => {
                let transient = error.contains("busy") || error.contains("timed out");
                last_error = error;
                if !transient || attempt == RETRY_BACKOFF_MS.len() {
                    break;
                }
                thread::sleep(Duration::from_millis(RETRY_BACKOFF_MS[attempt]));
            }
        }
    }
    Err(last_error)
}

fn root_feature(device: &HidDevice, device_index: u8, feature_id: u16) -> Result<u8, String> {
    let [high, low] = feature_id.to_be_bytes();
    let request = HidppRequest::new(device_index, 0x00, 0x00, vec![high, low, 0])
        .map_err(|error| error.to_string())?;
    let response = transact(device, &request)?;
    parse_feature_index(&response)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("HID++ feature {feature_id:#06x} is not supported"))
}

fn feature_index(features: &[FeatureInfo], feature_id: u16) -> Option<u8> {
    features
        .iter()
        .find(|feature| feature.feature_id == feature_id)
        .map(|feature| feature.index)
}

fn enumerate_features(device: &HidDevice, device_index: u8) -> Result<Vec<FeatureInfo>, String> {
    let feature_set_index = root_feature(device, device_index, FEATURE_SET)?;
    let count_request = HidppRequest::new(device_index, feature_set_index, 0x00, vec![0, 0, 0])
        .map_err(|error| error.to_string())?;
    let count_response = transact(device, &count_request)?;
    let count = parse_feature_set_count(&count_response).map_err(|error| error.to_string())?;

    let mut features = Vec::with_capacity(usize::from(count));
    for entry in feature_indices(count) {
        let request = HidppRequest::new(device_index, feature_set_index, 0x01, vec![entry, 0, 0])
            .map_err(|error| error.to_string())?;
        let response = transact(device, &request)?;
        let parsed = parse_feature_set_entry(entry, &response).map_err(|error| error.to_string())?;
        features.push(FeatureInfo {
            index: parsed.index,
            feature_id: parsed.feature_id,
            name: feature_name(parsed.feature_id).to_owned(),
            metadata: parsed.metadata,
            version: parsed.version,
        });
    }
    Ok(features)
}

fn read_dpi(device: &HidDevice, device_index: u8) -> Result<DpiInfo, String> {
    let dpi_index = root_feature(device, device_index, ADJUSTABLE_DPI)?;

    let sensor_count_request = HidppRequest::new(device_index, dpi_index, 0x00, vec![0, 0, 0])
        .map_err(|error| error.to_string())?;
    let sensor_count_response = transact(device, &sensor_count_request)?;
    let sensor_count = reforge_protocol::dpi::parse_sensor_count(&sensor_count_response)
        .map_err(|error| error.to_string())?;
    if sensor_count == 0 {
        return Err("device reports zero adjustable-DPI sensors".into());
    }

    let range_request = HidppRequest::new(device_index, dpi_index, 0x01, vec![0, 0, 0])
        .map_err(|error| error.to_string())?;
    let range_response = transact(device, &range_request)?;
    let supported = parse_dpi_values(&range_response).map_err(|error| error.to_string())?;
    let range = parse_dpi_range(&range_response).map_err(|error| error.to_string())?;

    let state_request = HidppRequest::new(device_index, dpi_index, 0x02, vec![0, 0, 0])
        .map_err(|error| error.to_string())?;
    let state = parse_dpi_state(&transact(device, &state_request)?)
        .map_err(|error| error.to_string())?;

    Ok(DpiInfo {
        sensor: state.sensor,
        min: range.min,
        max: range.max,
        step: range.step,
        current: state.current,
        default: state.default,
        supported,
    })
}

fn location_name(location: u16) -> String {
    match location {
        0x01 => "Primary".into(),
        0x02 => "Logo".into(),
        0x03 => "Left Side".into(),
        0x04 => "Right Side".into(),
        0x05 => "Combined".into(),
        0x06..=0x0b => format!("Primary {}", location - 5),
        _ => format!("Zone {location:#04x}"),
    }
}

fn probe_brightness(
    device: &HidDevice,
    device_index: u8,
    features: &[FeatureInfo],
) -> Option<BrightnessInfo> {
    if let Some(index) = feature_index(features, BRIGHTNESS_CONTROL) {
        let range = parse_brightness_range(
            &transact(device, &brightness_info_request(device_index, index).ok()?).ok()?,
        )
        .ok()?;
        let current = parse_brightness_value(
            &transact(device, &brightness_read_request(device_index, index).ok()?).ok()?,
        )
        .ok()?;
        return Some(BrightnessInfo {
            min: range.min,
            max: range.max,
            current,
            can_switch_off: range.can_switch_off,
            steps: range.steps,
        });
    }

    let index = feature_index(features, BACKLIGHT2)?;
    let info = parse_backlight2_info(
        &transact(device, &backlight2_info_request(device_index, index).ok()?).ok()?,
    )
    .ok()?;
    let range = parse_backlight2_range(
        &transact(device, &backlight2_range_request(device_index, index).ok()?).ok()?,
    )
    .ok()?;
    Some(BrightnessInfo {
        min: range.min,
        max: range.max,
        current: u16::from(info.level).min(range.max),
        can_switch_off: false,
        steps: range.steps,
    })
}

fn probe_zones(
    device: &HidDevice,
    device_index: u8,
    feature: u8,
    rgb_effects: bool,
) -> Result<Vec<LightingZone>, String> {
    let info_request = if rgb_effects {
        rgb_info_request(device_index, feature)
    } else {
        color_led_info_request(device_index, feature)
    }
    .map_err(|error| error.to_string())?;
    let info = parse_led_protocol_info(&transact(device, &info_request)?, rgb_effects)
        .map_err(|error| error.to_string())?;
    let mut zones = Vec::with_capacity(usize::from(info.zone_count));
    for zone_index in 0..info.zone_count {
        let request = if rgb_effects {
            rgb_zone_info_request(device_index, feature, zone_index)
        } else {
            zone_info_request(device_index, feature, zone_index)
        }
        .map_err(|error| error.to_string())?;
        let zone = parse_zone_probe(&transact(device, &request)?, rgb_effects)
            .map_err(|error| error.to_string())?;
        let mut effect_ids = Vec::with_capacity(usize::from(zone.effect_count));
        for effect_index in 0..zone.effect_count {
            let request = effect_info_request(
                device_index,
                feature,
                rgb_effects,
                zone_index,
                effect_index,
            )
            .map_err(|error| error.to_string())?;
            let effect = parse_effect_probe(&transact(device, &request)?)
                .map_err(|error| error.to_string())?;
            effect_ids.push(effect.effect_id);
        }
        zones.push(LightingZone {
            index: zone_index,
            location: zone.location,
            name: location_name(zone.location),
            effect_ids,
        });
    }
    Ok(zones)
}

fn supported_keys_from_pages(pages: &[Vec<u8>]) -> Vec<u8> {
    let bitmap: Vec<u8> = pages.iter().flatten().copied().collect();
    (1_u16..=254)
        .filter_map(|key| {
            let byte = usize::from(key / 8);
            let bit = (key % 8) as u8;
            bitmap
                .get(byte)
                .filter(|value| (**value >> bit) & 1 == 1)
                .map(|_| key as u8)
        })
        .collect()
}

fn probe_per_key(
    device: &HidDevice,
    device_index: u8,
    features: &[FeatureInfo],
) -> Vec<u8> {
    let Some(index) = feature_index(features, PER_KEY_LIGHTING_V2) else {
        return Vec::new();
    };
    let mut pages = Vec::new();
    for page in 0..=2 {
        let Ok(request) = per_key_bitmap_request(device_index, index, page) else {
            return Vec::new();
        };
        let Ok(response) = transact(device, &request) else {
            return Vec::new();
        };
        let Ok(bitmap) = parse_per_key_bitmap_page(&response) else {
            return Vec::new();
        };
        pages.push(bitmap.to_vec());
    }
    supported_keys_from_pages(&pages)
}

fn probe_lighting(
    device: &HidDevice,
    device_index: u8,
    features: &[FeatureInfo],
) -> Option<LightingCapabilities> {
    let brightness = probe_brightness(device, device_index, features);
    let rgb_index = feature_index(features, RGB_EFFECTS);
    let color_index = feature_index(features, COLOR_LED_EFFECTS);
    let rgb_effects = rgb_index.is_some();
    let color_led_effects = color_index.is_some();
    let zones = if let Some(index) = rgb_index {
        probe_zones(device, device_index, index, true).unwrap_or_default()
    } else if let Some(index) = color_index {
        probe_zones(device, device_index, index, false).unwrap_or_default()
    } else {
        Vec::new()
    };
    let supported_keys = probe_per_key(device, device_index, features);
    let caps = LightingCapabilities {
        brightness,
        backlight_v2: feature_index(features, BACKLIGHT2).is_some(),
        color_led_effects,
        rgb_effects,
        per_key_v2: feature_index(features, PER_KEY_LIGHTING_V2).is_some(),
        supported_keys,
        zones,
    };
    caps.supported().then_some(caps)
}

fn bus_kind(bus_type: BusType) -> TransportKind {
    match bus_type {
        BusType::Usb => TransportKind::Usb,
        BusType::Bluetooth => TransportKind::Bluetooth,
        BusType::I2c => TransportKind::I2c,
        BusType::Spi => TransportKind::Spi,
        _ => TransportKind::Unknown,
    }
}

fn device_key(info: &DeviceInfo, device_index: u8) -> String {
    format!(
        "{:04x}:{:02x}:{:02x}:{}",
        info.product_id(),
        info.interface_number(),
        device_index,
        info.path().to_string_lossy()
    )
}

fn classify_product(product: &str) -> DeviceClass {
    let lower = product.to_ascii_lowercase();
    if lower.contains("keyboard") || lower.contains("g515") || lower.contains("g915") || lower.contains("g815") || lower.contains("keys") {
        DeviceClass::Keyboard
    } else if lower.contains("mouse") || lower.contains("g502") || lower.contains("g703") || lower.contains("g903") || lower.contains("master") {
        DeviceClass::Mouse
    } else if lower.contains("receiver") {
        DeviceClass::Receiver
    } else {
        DeviceClass::Other
    }
}

fn probe_unit_id(device: &HidDevice, device_index: u8, features: &[FeatureInfo]) -> Option<String> {
    let index = feature_index(features, DEVICE_FW_VERSION)?;
    let request = HidppRequest::new(device_index, index, 0, vec![0, 0, 0]).ok()?;
    let response = transact(device, &request).ok()?;
    if response.params.len() < 5 {
        return None;
    }
    let unit = &response.params[1..5];
    if unit.iter().all(|byte| *byte == 0 || *byte == 0xff) {
        return None;
    }
    Some(unit.iter().map(|byte| format!("{byte:02X}")).collect::<Vec<_>>().join(""))
}

fn receiver_fingerprint(device: &DeviceSummary) -> String {
    let mut features: Vec<_> = device.features.iter().map(|f| format!("{:04x}:{}", f.feature_id, f.metadata)).collect();
    features.sort();
    let dpi = device.dpi.as_ref().map(|d| format!("{}:{}:{}:{:?}", d.min, d.max, d.step, d.supported)).unwrap_or_default();
    let lighting = device.lighting.as_ref().map(|l| format!("{:?}", l)).unwrap_or_default();
    let mut controls: Vec<_> = device.controls.iter().map(|control| control.id.as_str()).collect();
    controls.sort_unstable();
    format!("{:04x}|{}|{:?}|{}|{}|{:?}", device.product_id, device.hardware_id.as_deref().unwrap_or(""), features, dpi, lighting, controls)
}

pub fn collapse_receiver_mirrors(devices: Vec<DeviceSummary>) -> Vec<DeviceSummary> {
    let mut direct_by_path = BTreeMap::<String, String>::new();
    for device in &devices {
        if device.device_index == 0xff {
            direct_by_path.insert(device.path.clone(), receiver_fingerprint(device));
        }
    }
    let mut seen = HashSet::new();
    devices
        .into_iter()
        .filter(|device| {
            if device.device_index != 0xff
                && direct_by_path
                    .get(&device.path)
                    .is_some_and(|fingerprint| *fingerprint == receiver_fingerprint(device))
            {
                return false;
            }
            seen.insert((device.path.clone(), device.device_index, receiver_fingerprint(device)))
        })
        .collect()
}

fn summary_for(
    info: &DeviceInfo,
    device_index: u8,
    features: Vec<FeatureInfo>,
    dpi: Option<DpiInfo>,
    lighting: Option<LightingCapabilities>,
    controls: Vec<reforge_core::DeviceControl>,
    serial_override: Option<String>,
    error: Option<String>,
) -> DeviceSummary {
    let base_product = info
        .product_string()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("Logitech {:04x}", info.product_id()));
    let product = if device_index == 0xff {
        base_product
    } else {
        format!("{base_product} [receiver slot {device_index}]")
    };
    let device_class = classify_product(&product);
    DeviceSummary {
        key: device_key(info, device_index),
        path: info.path().to_string_lossy().into_owned(),
        device_index,
        vendor_id: info.vendor_id(),
        product_id: info.product_id(),
        product,
        serial: info.serial_number().map(ToOwned::to_owned),
        hardware_id: serial_override,
        interface_number: info.interface_number(),
        usage_page: info.usage_page(),
        usage: info.usage(),
        transport: bus_kind(info.bus_type()),
        hidpp: true,
        device_class,
        providers: vec![ProviderKind::Hidpp],
        controls,
        features,
        dpi,
        lighting,
        error,
    }
}


pub fn scan_logitech_nonusb_hid_inventory() -> Result<Vec<DeviceSummary>, String> {
    let api = HidApi::new().map_err(|error| format!("failed to initialize HIDAPI: {error}"))?;
    let mut devices = Vec::new();
    let mut seen = HashSet::new();
    for info in api
        .device_list()
        .filter(|info| info.vendor_id() == LOGITECH_VENDOR_ID && !matches!(info.bus_type(), BusType::Usb))
    {
        let product = info.product_string().map(ToOwned::to_owned).unwrap_or_else(|| format!("Logitech {:04x}", info.product_id()));
        let path = info.path().to_string_lossy().into_owned();
        if !seen.insert(path.clone()) { continue; }
        devices.push(DeviceSummary {
            key: format!("hid:{}", path),
            path,
            device_index: 0xff,
            vendor_id: info.vendor_id(),
            product_id: info.product_id(),
            product: product.clone(),
            serial: info.serial_number().map(ToOwned::to_owned),
            hardware_id: None,
            interface_number: info.interface_number(),
            usage_page: info.usage_page(),
            usage: info.usage(),
            transport: bus_kind(info.bus_type()),
            hidpp: false,
            device_class: classify_product(&product),
            providers: vec![ProviderKind::Hid],
            controls: vec![], features: vec![], dpi: None, lighting: None, error: None,
        });
    }
    Ok(devices)
}

pub fn scan_logitech_devices() -> Result<Vec<DeviceSummary>, String> {
    let api = HidApi::new().map_err(|error| format!("failed to initialize HIDAPI: {error}"))?;
    let mut devices = Vec::new();
    let mut keys = HashSet::new();

    for info in api
        .device_list()
        .filter(|info| info.vendor_id() == LOGITECH_VENDOR_ID)
    {
        let device = match info.open_device(&api) {
            Ok(device) => device,
            Err(_) => continue,
        };

        for device_index in probe_indices() {
            if root_feature(&device, device_index, FEATURE_SET).is_err() {
                continue;
            }
            let features = match enumerate_features(&device, device_index) {
                Ok(features) => features,
                Err(error) => {
                    let summary = summary_for(
                        info,
                        device_index,
                        Vec::new(),
                        None,
                        None,
                        Vec::new(),
                        None,
                        Some(error),
                    );
                    if keys.insert(summary.key.clone()) {
                        devices.push(summary);
                    }
                    continue;
                }
            };
            let dpi = read_dpi(&device, device_index).ok();
            let lighting = probe_lighting(&device, device_index, &features);
            let unit_id = probe_unit_id(&device, device_index, &features);
            let controls = control::probe_controls(&device, device_index, &features, dpi.as_ref());
            let summary = summary_for(info, device_index, features, dpi, lighting, controls, unit_id, None);
            if keys.insert(summary.key.clone()) {
                devices.push(summary);
            }
        }
    }

    let mut devices = collapse_receiver_mirrors(devices);
    devices.sort_by(|a, b| a.product.cmp(&b.product).then(a.key.cmp(&b.key)));
    Ok(devices)
}

pub(crate) fn open_summary_device(device: &DeviceSummary) -> Result<HidDevice, String> {
    let api = HidApi::new().map_err(|error| format!("failed to initialize HIDAPI: {error}"))?;
    let path = CString::new(device.path.as_bytes())
        .map_err(|_| "device path contains an embedded NUL byte".to_owned())?;
    api.open_path(&path)
        .map_err(|error| format!("cannot open HID device {}: {error}", device.path))
}

pub fn set_dpi_with_handle(handle: &HidDevice, device: &DeviceSummary, dpi: u16) -> Result<DpiInfo, String> {
    if !device.hidpp {
        return Err("selected device is not a HID++ device".into());
    }

    let dpi_index = root_feature(handle, device.device_index, ADJUSTABLE_DPI)?;
    let range_request = HidppRequest::new(device.device_index, dpi_index, 0x01, vec![0, 0, 0])
        .map_err(|error| error.to_string())?;
    let range_response = transact(handle, &range_request)?;
    let range = parse_dpi_range(&range_response).map_err(|error| error.to_string())?;
    let supported = parse_dpi_values(&range_response).map_err(|error| error.to_string())?;
    validate_dpi_value(&supported, dpi).map_err(|error| error.to_string())?;

    let set_request = build_set_dpi_request(device.device_index, dpi_index, range.sensor, dpi)
        .map_err(|error| error.to_string())?;
    transact(handle, &set_request)?;

    let verified = read_dpi(handle, device.device_index)?;
    if verified.current != dpi {
        return Err(format!(
            "DPI write did not verify: requested {dpi}, device reports {}",
            verified.current
        ));
    }
    Ok(verified)
}

pub fn set_dpi(device: &DeviceSummary, dpi: u16) -> Result<DpiInfo, String> {
    let handle = open_summary_device(device)?;
    set_dpi_with_handle(&handle, device, dpi)
}

fn effect_candidates(effect: LightingEffect) -> &'static [u16] {
    match effect {
        LightingEffect::Off => &[0x0000],
        LightingEffect::Static
        | LightingEffect::Gradient
        | LightingEffect::ScreenReactive
        | LightingEffect::AudioReactive => &[0x0001],
        LightingEffect::Breathing => &[0x000a, 0x0002],
        LightingEffect::ColorCycle => &[0x0015, 0x0003],
        LightingEffect::Wave => &[0x0016, 0x0004],
        LightingEffect::Ripple => &[0x0017, 0x000b],
    }
}

fn effect_parameters(state: &LightingState) -> LedEffectParameters {
    LedEffectParameters {
        color: state.primary.packed(),
        speed: (10_000_u32 / u32::from(state.period_ms.max(250))).clamp(1, 255) as u8,
        period_ms: state.period_ms.max(2),
        intensity: state.intensity.min(100),
        saturation: 255,
        direction: state.direction,
        ramp: 3,
        form: 0,
    }
}

fn claim_rgb_control(
    handle: &HidDevice,
    device: &DeviceSummary,
    enabled: bool,
) -> Result<(), String> {
    if let Some(onboard) = feature_index(&device.features, ONBOARD_PROFILES) {
        let request = onboard_profile_mode_request(device.device_index, onboard, enabled)
            .map_err(|error| error.to_string())?;
        transact_retry(handle, &request)?;
    }
    if let Some(rgb) = feature_index(&device.features, RGB_EFFECTS) {
        let request = rgb_sw_control_request(device.device_index, rgb, enabled)
            .map_err(|error| error.to_string())?;
        transact_retry(handle, &request)?;
    } else if let Some(color) = feature_index(&device.features, COLOR_LED_EFFECTS) {
        let request = color_led_control_request(device.device_index, color, enabled)
            .map_err(|error| error.to_string())?;
        transact_retry(handle, &request)?;
    }
    Ok(())
}

fn apply_brightness(
    handle: &HidDevice,
    device: &DeviceSummary,
    state: &LightingState,
) -> Result<(), String> {
    let value = state.brightness;
    if let Some(index) = feature_index(&device.features, BRIGHTNESS_CONTROL) {
        let Some(value) = value else {
            return Ok(());
        };
        let range = parse_brightness_range(&transact_retry(
            handle,
            &brightness_info_request(device.device_index, index).map_err(|error| error.to_string())?,
        )?)
        .map_err(|error| error.to_string())?;
        let valid = (value == 0 && range.can_switch_off) || (value >= range.min && value <= range.max);
        if !valid {
            return Err(format!(
                "brightness {value} is outside device range {}–{}",
                range.min, range.max
            ));
        }
        if range.can_switch_off {
            let power = brightness_power_request(device.device_index, index, value != 0)
                .map_err(|error| error.to_string())?;
            transact_retry(handle, &power)?;
            if value == 0 {
                return Ok(());
            }
        }
        let request = brightness_write_request(device.device_index, index, value)
            .map_err(|error| error.to_string())?;
        transact_retry(handle, &request)?;
        let read = brightness_read_request(device.device_index, index)
            .map_err(|error| error.to_string())?;
        let verified = parse_brightness_value(&transact_retry(handle, &read)?)
            .map_err(|error| error.to_string())?;
        if verified != value {
            return Err(format!(
                "brightness write did not verify: requested {value}, device reports {verified}"
            ));
        }
        return Ok(());
    }

    let Some(index) = feature_index(&device.features, BACKLIGHT2) else {
        return if value.is_some() {
            Err("device does not expose an adjustable backlight feature".into())
        } else {
            Ok(())
        };
    };
    let info_request = backlight2_info_request(device.device_index, index)
        .map_err(|error| error.to_string())?;
    let current = parse_backlight2_info(&transact_retry(handle, &info_request)?)
        .map_err(|error| error.to_string())?;
    let level = if let Some(value) = value {
        let range_request = backlight2_range_request(device.device_index, index)
            .map_err(|error| error.to_string())?;
        let range = parse_backlight2_range(&transact_retry(handle, &range_request)?)
            .map_err(|error| error.to_string())?;
        if value < range.min || value > range.max {
            return Err(format!(
                "backlight level {value} is outside device range {}–{}",
                range.min, range.max
            ));
        }
        Some(u8::try_from(value).map_err(|_| "Backlight2 level does not fit in one byte")?)
    } else {
        None
    };
    let write = backlight2_write_request(device.device_index, index, &current, state.enabled, level)
        .map_err(|error| error.to_string())?;
    transact_retry(handle, &write)?;
    Ok(())
}

fn apply_zone_effect_inner(
    handle: &HidDevice,
    device: &DeviceSummary,
    state: &LightingState,
    claim: bool,
) -> Result<(), String> {
    let caps = device
        .lighting
        .as_ref()
        .ok_or_else(|| "device has no lighting capabilities".to_owned())?;
    if caps.zones.is_empty() {
        return Ok(());
    }
    if claim {
        claim_rgb_control(handle, device, true)?;
    }
    let params = effect_parameters(state);
    let candidates = effect_candidates(if state.enabled {
        state.effect
    } else {
        LightingEffect::Off
    });
    for zone in &caps.zones {
        let (effect_index, effect_id) = candidates
            .iter()
            .find_map(|candidate| {
                zone.effect_ids
                    .iter()
                    .position(|id| id == candidate)
                    .map(|index| (index as u8, *candidate))
            })
            .or_else(|| {
                zone.effect_ids
                    .iter()
                    .position(|id| *id == 0x0001)
                    .map(|index| (index as u8, 0x0001))
            })
            .ok_or_else(|| format!("{} exposes no compatible lighting effect", zone.name))?;
        let mut zone_params = params.clone();
        if !state.enabled && effect_id == 0x0001 {
            zone_params.color = 0;
        }
        let wire = encode_effect_parameters(effect_id, &zone_params);
        let request = if let Some(rgb) = feature_index(&device.features, RGB_EFFECTS) {
            rgb_set_effect_request(device.device_index, rgb, zone.index, effect_index, wire)
        } else if let Some(color) = feature_index(&device.features, COLOR_LED_EFFECTS) {
            color_led_set_effect_request(device.device_index, color, zone.index, effect_index, wire)
        } else {
            return Err("device has no writable LED effects feature".into());
        }
        .map_err(|error| error.to_string())?;
        transact_retry(handle, &request)?;
    }
    Ok(())
}

fn apply_zone_effect(
    handle: &HidDevice,
    device: &DeviceSummary,
    state: &LightingState,
) -> Result<(), String> {
    apply_zone_effect_inner(handle, device, state, true)
}

fn write_per_key_frame_handle_inner(
    handle: &HidDevice,
    device: &DeviceSummary,
    values: &[(u8, RgbColor)],
    claim: bool,
) -> Result<(), String> {
    let index = feature_index(&device.features, PER_KEY_LIGHTING_V2)
        .ok_or_else(|| "device does not expose Per-Key Lighting v2 (0x8081)".to_owned())?;
    let supported: HashSet<u8> = device
        .lighting
        .as_ref()
        .map(|caps| caps.supported_keys.iter().copied().collect())
        .unwrap_or_default();
    if let Some((key, _)) = values.iter().find(|(key, _)| !supported.contains(key)) {
        return Err(format!("key zone {key:#04x} is not advertised by this device"));
    }

    if claim {
        claim_rgb_control(handle, device, true)?;
    }
    let mut by_color: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
    for (key, color) in values {
        by_color.entry(color.packed()).or_default().push(*key);
    }
    for (color, keys) in by_color {
        for chunk in keys.chunks(13) {
            let request = per_key_set_uniform_request(device.device_index, index, chunk, color)
                .map_err(|error| error.to_string())?;
            transact_retry(handle, &request)?;
        }
    }
    let commit = per_key_frame_end_request(device.device_index, index)
        .map_err(|error| error.to_string())?;
    transact_retry(handle, &commit)?;
    Ok(())
}

fn write_per_key_frame_handle(
    handle: &HidDevice,
    device: &DeviceSummary,
    values: &[(u8, RgbColor)],
) -> Result<(), String> {
    write_per_key_frame_handle_inner(handle, device, values, true)
}

pub struct HostFrameSession {
    device: DeviceSummary,
    handle: HidDevice,
}

impl HostFrameSession {
    pub fn open(device: &DeviceSummary) -> Result<Self, String> {
        let handle = open_summary_device(device)?;
        claim_rgb_control(&handle, device, true)?;
        Ok(Self {
            device: device.clone(),
            handle,
        })
    }

    pub fn write_frame(&mut self, values: &[(u8, RgbColor)]) -> Result<(), String> {
        let caps = self
            .device
            .lighting
            .as_ref()
            .ok_or_else(|| "device has no lighting capabilities".to_owned())?;
        if caps.per_key_v2 && !caps.supported_keys.is_empty() {
            return write_per_key_frame_handle_inner(&self.handle, &self.device, values, false);
        }
        if values.is_empty() {
            return Ok(());
        }
        let count = values.len() as u32;
        let sum = values.iter().fold((0_u32, 0_u32, 0_u32), |acc, (_, color)| {
            (
                acc.0 + u32::from(color.r),
                acc.1 + u32::from(color.g),
                acc.2 + u32::from(color.b),
            )
        });
        let state = LightingState {
            effect: LightingEffect::Static,
            primary: RgbColor::new(
                (sum.0 / count) as u8,
                (sum.1 / count) as u8,
                (sum.2 / count) as u8,
            ),
            ..Default::default()
        };
        apply_zone_effect_inner(&self.handle, &self.device, &state, false)
    }
}

pub fn set_per_key_frame_with_handle(
    handle: &HidDevice,
    device: &DeviceSummary,
    values: &[(u8, RgbColor)],
) -> Result<(), String> {
    write_per_key_frame_handle(handle, device, values)
}

pub fn set_per_key_frame(
    device: &DeviceSummary,
    values: &[(u8, RgbColor)],
) -> Result<(), String> {
    let handle = open_summary_device(device)?;
    set_per_key_frame_with_handle(&handle, device, values)
}

pub fn set_key_color_with_handle(handle: &HidDevice, device: &DeviceSummary, key: u8, color: RgbColor) -> Result<(), String> {
    set_per_key_frame_with_handle(handle, device, &[(key, color)])
}

pub fn set_key_color(device: &DeviceSummary, key: u8, color: RgbColor) -> Result<(), String> {
    set_per_key_frame(device, &[(key, color)])
}

pub fn set_host_frame_with_handle(handle: &HidDevice, device: &DeviceSummary, values: &[(u8, RgbColor)]) -> Result<(), String> {
    let caps = device
        .lighting
        .as_ref()
        .ok_or_else(|| "device has no lighting capabilities".to_owned())?;
    if caps.per_key_v2 && !caps.supported_keys.is_empty() {
        return set_per_key_frame_with_handle(handle, device, values);
    }
    if values.is_empty() {
        return Ok(());
    }
    let count = values.len() as u32;
    let sum = values.iter().fold((0_u32, 0_u32, 0_u32), |acc, (_, color)| {
        (
            acc.0 + u32::from(color.r),
            acc.1 + u32::from(color.g),
            acc.2 + u32::from(color.b),
        )
    });
    let state = LightingState {
        effect: LightingEffect::Static,
        primary: RgbColor::new(
            (sum.0 / count) as u8,
            (sum.1 / count) as u8,
            (sum.2 / count) as u8,
        ),
        ..Default::default()
    };
    apply_lighting_with_handle(handle, device, &state)
}


pub fn set_host_frame(device: &DeviceSummary, values: &[(u8, RgbColor)]) -> Result<(), String> {
    let handle = open_summary_device(device)?;
    set_host_frame_with_handle(&handle, device, values)
}

pub fn apply_lighting_with_handle(handle: &HidDevice, device: &DeviceSummary, state: &LightingState) -> Result<(), String> {
    if !device.hidpp {
        return Err("selected device is not a HID++ device".into());
    }
    let caps = device
        .lighting
        .as_ref()
        .ok_or_else(|| "selected device exposes no supported lighting controls".to_owned())?;
    apply_brightness(handle, device, state)?;

    if state.effect.is_host_driven() {
        if caps.per_key_v2 && !caps.supported_keys.is_empty() {
            let mut static_base = state.clone();
            static_base.effect = LightingEffect::Static;
            static_base.enabled = true;
            apply_zone_effect(handle, device, &static_base)?;
            let values: Vec<_> = caps
                .supported_keys
                .iter()
                .copied()
                .map(|key| (key, state.per_key.get(&key).copied().unwrap_or(state.primary)))
                .collect();
            return write_per_key_frame_handle_inner(handle, device, &values, false);
        }
        return apply_zone_effect(handle, device, state);
    }

    apply_zone_effect(handle, device, state)?;
    if state.effect == LightingEffect::Static && caps.per_key_v2 && !state.per_key.is_empty() {
        let values: Vec<_> = state.per_key.iter().map(|(key, color)| (*key, *color)).collect();
        write_per_key_frame_handle(handle, device, &values)?;
    }
    Ok(())
}

pub fn apply_lighting(device: &DeviceSummary, state: &LightingState) -> Result<(), String> {
    let handle = open_summary_device(device)?;
    apply_lighting_with_handle(&handle, device, state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_indices_include_reported_last_entry() {
        assert_eq!(feature_indices(3), vec![1, 2, 3]);
    }

    #[test]
    fn per_key_bitmap_collects_supported_zone_ids() {
        let mut page = vec![0_u8; 14];
        page[0] |= 1 << 4; // HID key 4
        page[7] |= 1 << 2; // HID key 58
        let keys = supported_keys_from_pages(&[page, vec![0; 14], vec![0; 14]]);
        assert!(keys.contains(&4));
        assert!(keys.contains(&58));
        assert!(!keys.contains(&5));
    }

    #[test]
    fn effect_candidates_prefer_newer_wave_id() {
        assert_eq!(effect_candidates(LightingEffect::Wave), &[0x16, 0x04]);
    }
}
