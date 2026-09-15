use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;

const VERSION: &str = "10.2.93";
const INPUT_SOURCE: &str = "AETHERFORGE_CONTROL_BRIDGE_READONLY_V1";
const OUTPUT_SOURCE: &str = "AETHERFORGE_CONTROL_CENTER_CONSUMER_V1";
const PROTOCOL_VERSION: u16 = 1;
const CONTROL_CENTER_ROLE: &str = "READ_ONLY_CLIENT";
const DISPLAY_OWNER: &str = "AETHERFORGE_DISPLAY_SERVICE";
const DSP_OWNER: &str = "AETHERSTREAM";
const DSP_PENDING: &str = "UNAVAILABLE_UNTIL_TYPED_SNAPSHOT_ADAPTER";
const REQUIRED_BRIDGE_VERSION: &str = "10.2.91";
const TYPED_ADAPTER_SOURCE: &str = "AETHERSTREAM_TYPED_CONTROL_CENTER_ADAPTER_V1";
const TYPED_ADAPTER_VERSION: &str = "10.2.88";
const TYPED_ADAPTER_CONTROL_MODE: &str = "READ_ONLY_GET_STATE";

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

struct JsonParser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            input: text.as_bytes(),
            pos: 0,
        }
    }

    fn parse(mut self) -> Result<JsonValue, String> {
        let value = self.parse_value()?;
        self.skip_ws();
        if self.pos != self.input.len() {
            return Err(format!("trailing JSON at byte {}", self.pos));
        }
        Ok(value)
    }

    fn skip_ws(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_ws();
        let Some(&byte) = self.input.get(self.pos) else {
            return Err("unexpected end of JSON".to_string());
        };

        match byte {
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b'"' => self.parse_string().map(JsonValue::String),
            b't' => {
                self.expect_bytes(b"true")?;
                Ok(JsonValue::Bool(true))
            }
            b'f' => {
                self.expect_bytes(b"false")?;
                Ok(JsonValue::Bool(false))
            }
            b'n' => {
                self.expect_bytes(b"null")?;
                Ok(JsonValue::Null)
            }
            b'-' | b'0'..=b'9' => self.parse_number().map(JsonValue::Number),
            _ => Err(format!("unexpected JSON byte {} at {}", byte, self.pos)),
        }
    }

    fn expect_bytes(&mut self, expected: &[u8]) -> Result<(), String> {
        if self.input.get(self.pos..self.pos + expected.len()) == Some(expected) {
            self.pos += expected.len();
            Ok(())
        } else {
            Err(format!("invalid JSON literal at byte {}", self.pos))
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, String> {
        self.pos += 1;
        let mut map = BTreeMap::new();

        loop {
            self.skip_ws();
            if self.input.get(self.pos) == Some(&b'}') {
                self.pos += 1;
                break;
            }

            let key = self.parse_string()?;
            self.skip_ws();
            if self.input.get(self.pos) != Some(&b':') {
                return Err(format!("expected ':' at byte {}", self.pos));
            }
            self.pos += 1;
            let value = self.parse_value()?;
            if map.insert(key.clone(), value).is_some() {
                return Err(format!("duplicate JSON key: {key}"));
            }

            self.skip_ws();
            match self.input.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(format!("expected ',' or '}}' at byte {}", self.pos)),
            }
        }

        Ok(JsonValue::Object(map))
    }

    fn parse_array(&mut self) -> Result<JsonValue, String> {
        self.pos += 1;
        let mut values = Vec::new();

        loop {
            self.skip_ws();
            if self.input.get(self.pos) == Some(&b']') {
                self.pos += 1;
                break;
            }

            values.push(self.parse_value()?);
            self.skip_ws();
            match self.input.get(self.pos) {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err(format!("expected ',' or ']' at byte {}", self.pos)),
            }
        }

        Ok(JsonValue::Array(values))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.skip_ws();
        if self.input.get(self.pos) != Some(&b'"') {
            return Err(format!("expected string at byte {}", self.pos));
        }
        self.pos += 1;

        let mut result = String::new();
        while self.pos < self.input.len() {
            let byte = self.input[self.pos];
            self.pos += 1;
            match byte {
                b'"' => return Ok(result),
                b'\\' => {
                    let Some(&escaped) = self.input.get(self.pos) else {
                        return Err("truncated JSON escape".to_string());
                    };
                    self.pos += 1;
                    match escaped {
                        b'"' => result.push('"'),
                        b'\\' => result.push('\\'),
                        b'/' => result.push('/'),
                        b'b' => result.push('\u{0008}'),
                        b'f' => result.push('\u{000c}'),
                        b'n' => result.push('\n'),
                        b'r' => result.push('\r'),
                        b't' => result.push('\t'),
                        b'u' => {
                            if self.pos + 4 > self.input.len() {
                                return Err("truncated Unicode escape".to_string());
                            }
                            let hex = std::str::from_utf8(&self.input[self.pos..self.pos + 4])
                                .map_err(|_| "invalid Unicode escape".to_string())?;
                            self.pos += 4;
                            let scalar = u32::from_str_radix(hex, 16)
                                .map_err(|_| "invalid Unicode escape".to_string())?;
                            result.push(
                                char::from_u32(scalar)
                                    .ok_or_else(|| "invalid Unicode scalar".to_string())?,
                            );
                        }
                        _ => return Err("unsupported JSON escape".to_string()),
                    }
                }
                0x00..=0x1f => return Err("control byte in JSON string".to_string()),
                0x20..=0x7f => result.push(byte as char),
                _ => {
                    let start = self.pos - 1;
                    let tail = std::str::from_utf8(&self.input[start..])
                        .map_err(|_| "invalid UTF-8 in JSON string".to_string())?;
                    let character = tail
                        .chars()
                        .next()
                        .ok_or_else(|| "invalid UTF-8".to_string())?;
                    result.push(character);
                    self.pos = start + character.len_utf8();
                }
            }
        }

        Err("unterminated JSON string".to_string())
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        let start = self.pos;
        while self.pos < self.input.len() {
            match self.input[self.pos] {
                b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E' => self.pos += 1,
                _ => break,
            }
        }

        let token = std::str::from_utf8(&self.input[start..self.pos])
            .map_err(|_| "invalid number encoding".to_string())?;
        let value = token
            .parse::<f64>()
            .map_err(|_| format!("invalid JSON number: {token}"))?;
        if !value.is_finite() {
            return Err("non-finite JSON number".to_string());
        }
        Ok(value)
    }
}

fn as_object(value: &JsonValue) -> Result<&BTreeMap<String, JsonValue>, String> {
    match value {
        JsonValue::Object(map) => Ok(map),
        _ => Err("expected JSON object".to_string()),
    }
}

fn as_array(value: &JsonValue) -> Result<&[JsonValue], String> {
    match value {
        JsonValue::Array(values) => Ok(values),
        _ => Err("expected JSON array".to_string()),
    }
}

fn string_value(map: &BTreeMap<String, JsonValue>, key: &str) -> Option<String> {
    match map.get(key) {
        Some(JsonValue::String(value)) => Some(value.clone()),
        _ => None,
    }
}

fn optional_string(map: &BTreeMap<String, JsonValue>, key: &str) -> Result<Option<String>, String> {
    match map.get(key) {
        Some(JsonValue::String(value)) => Ok(Some(value.clone())),
        Some(JsonValue::Null) => Ok(None),
        Some(_) => Err(format!("{key} must be string or null")),
        None => Err(format!("missing {key}")),
    }
}

fn bool_value(map: &BTreeMap<String, JsonValue>, key: &str) -> Option<bool> {
    match map.get(key) {
        Some(JsonValue::Bool(value)) => Some(*value),
        _ => None,
    }
}

fn u16_value(map: &BTreeMap<String, JsonValue>, key: &str) -> Option<u16> {
    match map.get(key) {
        Some(JsonValue::Number(value))
            if *value >= 0.0 && *value <= u16::MAX as f64 && value.fract() == 0.0 =>
        {
            Some(*value as u16)
        }
        _ => None,
    }
}

fn u64_value(map: &BTreeMap<String, JsonValue>, key: &str) -> Option<u64> {
    match map.get(key) {
        Some(JsonValue::Number(value))
            if *value >= 0.0 && *value <= u64::MAX as f64 && value.fract() == 0.0 =>
        {
            Some(*value as u64)
        }
        _ => None,
    }
}

fn optional_u64(map: &BTreeMap<String, JsonValue>, key: &str) -> Result<Option<u64>, String> {
    match map.get(key) {
        Some(JsonValue::Number(value))
            if *value >= 0.0 && *value <= u64::MAX as f64 && value.fract() == 0.0 =>
        {
            Ok(Some(*value as u64))
        }
        Some(JsonValue::Null) => Ok(None),
        Some(_) => Err(format!("{key} must be unsigned integer or null")),
        None => Err(format!("missing {key}")),
    }
}

#[derive(Debug, Clone, PartialEq)]
struct TypedAdapterView {
    source: String,
    adapter_version: String,
    control_mode: String,
    status: String,
    state_revision: Option<u64>,
    reason: Option<String>,
    snapshot: Option<JsonValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DisplayRow {
    id: String,
    name: String,
    current_physical: String,
    canonical_family: String,
    native_layout: String,
    semi_native_layout: String,
    orientation: String,
}

#[derive(Debug, Clone, PartialEq)]
struct DspView {
    owner: String,
    health: String,
    detail_status: String,
    input_device: Option<String>,
    output_device: Option<String>,
    profile: Option<String>,
    typed_adapter: TypedAdapterView,
}

#[derive(Debug, Clone, PartialEq)]
struct ConsumerView {
    bridge_version: String,
    protocol_version: u16,
    control_center_role: String,
    live_control: bool,
    display_owner: String,
    displays: Vec<DisplayRow>,
    dsp: DspView,
}

fn validate_embedded_typed_snapshot(
    snapshot: &JsonValue,
    expected_revision: u64,
) -> Result<(), String> {
    let root = as_object(snapshot)?;
    if string_value(root, "source").as_deref() != Some(TYPED_ADAPTER_SOURCE) {
        return Err("embedded typed snapshot source mismatch".to_string());
    }
    if string_value(root, "adapter_version").as_deref() != Some(TYPED_ADAPTER_VERSION) {
        return Err("embedded typed snapshot adapter version mismatch".to_string());
    }
    if string_value(root, "aetherstream_version").as_deref() != Some("10.2.50") {
        return Err("embedded typed snapshot AetherStream version mismatch".to_string());
    }
    if u16_value(root, "ipc_protocol_version") != Some(1) {
        return Err("embedded typed snapshot IPC protocol mismatch".to_string());
    }
    if string_value(root, "control_mode").as_deref() != Some(TYPED_ADAPTER_CONTROL_MODE) {
        return Err("embedded typed snapshot is not read-only GetState".to_string());
    }

    let state = as_object(
        root.get("state")
            .ok_or_else(|| "embedded typed snapshot missing state".to_string())?,
    )?;
    let revision = u64_value(state, "revision")
        .ok_or_else(|| "embedded typed snapshot missing state revision".to_string())?;
    if revision != expected_revision {
        return Err(format!(
            "typed state revision mismatch: outer {expected_revision}, embedded {revision}"
        ));
    }
    Ok(())
}

fn parse_typed_adapter(dsp: &BTreeMap<String, JsonValue>) -> Result<TypedAdapterView, String> {
    let typed = as_object(
        dsp.get("typed_adapter")
            .ok_or_else(|| "missing dsp.typed_adapter".to_string())?,
    )?;

    let source =
        string_value(typed, "source").ok_or_else(|| "typed adapter source missing".to_string())?;
    if source != TYPED_ADAPTER_SOURCE {
        return Err("typed adapter source mismatch".to_string());
    }

    let adapter_version = string_value(typed, "adapter_version")
        .ok_or_else(|| "typed adapter version missing".to_string())?;
    if adapter_version != TYPED_ADAPTER_VERSION {
        return Err("typed adapter version mismatch".to_string());
    }

    let control_mode = string_value(typed, "control_mode")
        .ok_or_else(|| "typed adapter control_mode missing".to_string())?;
    if control_mode != TYPED_ADAPTER_CONTROL_MODE {
        return Err("typed adapter is not read-only GetState".to_string());
    }

    let status =
        string_value(typed, "status").ok_or_else(|| "typed adapter status missing".to_string())?;
    if !matches!(
        status.as_str(),
        "ONLINE" | "OFFLINE" | "MISSING" | "INVALID" | "ERROR"
    ) {
        return Err(format!("unsupported typed adapter status: {status}"));
    }

    let state_revision = optional_u64(typed, "state_revision")?;
    let reason = optional_string(typed, "reason")?;
    let snapshot = match typed.get("snapshot") {
        Some(JsonValue::Object(_)) => typed.get("snapshot").cloned(),
        Some(JsonValue::Null) => None,
        Some(_) => return Err("typed adapter snapshot must be object or null".to_string()),
        None => return Err("missing typed adapter snapshot".to_string()),
    };

    if status == "ONLINE" {
        let revision = state_revision
            .ok_or_else(|| "ONLINE typed adapter requires state_revision".to_string())?;
        if reason.is_some() {
            return Err("ONLINE typed adapter reason must be null".to_string());
        }
        let snapshot_ref = snapshot
            .as_ref()
            .ok_or_else(|| "ONLINE typed adapter requires snapshot".to_string())?;
        validate_embedded_typed_snapshot(snapshot_ref, revision)?;
    } else {
        if state_revision.is_some() {
            return Err(format!(
                "{status} typed adapter must not claim a state revision"
            ));
        }
        if snapshot.is_some() {
            return Err(format!("{status} typed adapter must not expose a snapshot"));
        }
        if reason.as_deref().is_none_or(str::is_empty) {
            return Err(format!(
                "{status} typed adapter requires a diagnostic reason"
            ));
        }
    }

    Ok(TypedAdapterView {
        source,
        adapter_version,
        control_mode,
        status,
        state_revision,
        reason,
        snapshot,
    })
}

fn parse_consumer_view(text: &str) -> Result<ConsumerView, String> {
    let root_value = JsonParser::new(text).parse()?;
    let root = as_object(&root_value)?;

    if string_value(root, "source").as_deref() != Some(INPUT_SOURCE) {
        return Err("unexpected bridge source".to_string());
    }

    let bridge_version =
        string_value(root, "version").ok_or_else(|| "missing bridge version".to_string())?;
    if bridge_version != REQUIRED_BRIDGE_VERSION {
        return Err(format!(
            "bridge version mismatch: expected {REQUIRED_BRIDGE_VERSION}, got {bridge_version}"
        ));
    }

    let protocol_version = u16_value(root, "protocol_version")
        .ok_or_else(|| "missing protocol_version".to_string())?;
    if protocol_version != PROTOCOL_VERSION {
        return Err(format!(
            "protocol version mismatch: expected {PROTOCOL_VERSION}, got {protocol_version}"
        ));
    }

    let role = string_value(root, "control_center_role")
        .ok_or_else(|| "missing control_center_role".to_string())?;
    if role != CONTROL_CENTER_ROLE {
        return Err("Control Center role is not read-only".to_string());
    }

    let live_control =
        bool_value(root, "live_control").ok_or_else(|| "missing live_control".to_string())?;
    if live_control {
        return Err("live control is forbidden in v10.2.93".to_string());
    }

    let display = as_object(
        root.get("display")
            .ok_or_else(|| "missing display object".to_string())?,
    )?;
    let display_owner =
        string_value(display, "owner").ok_or_else(|| "missing display owner".to_string())?;
    if display_owner != DISPLAY_OWNER {
        return Err("display ownership mismatch".to_string());
    }

    let output_values = as_array(
        display
            .get("outputs")
            .ok_or_else(|| "missing display outputs".to_string())?,
    )?;
    let mut displays = Vec::new();
    for item in output_values {
        let output = as_object(item)?;
        displays.push(DisplayRow {
            id: string_value(output, "id").ok_or_else(|| "display id missing".to_string())?,
            name: string_value(output, "name").ok_or_else(|| "display name missing".to_string())?,
            current_physical: string_value(output, "current_physical")
                .ok_or_else(|| "current physical mode missing".to_string())?,
            canonical_family: string_value(output, "canonical_family")
                .ok_or_else(|| "canonical family missing".to_string())?,
            native_layout: string_value(output, "native_layout")
                .ok_or_else(|| "native layout missing".to_string())?,
            semi_native_layout: string_value(output, "semi_native_layout")
                .ok_or_else(|| "Semi-Native layout missing".to_string())?,
            orientation: string_value(output, "orientation")
                .ok_or_else(|| "orientation missing".to_string())?,
        });
    }

    let dsp = as_object(
        root.get("dsp")
            .ok_or_else(|| "missing dsp object".to_string())?,
    )?;
    let dsp_owner = string_value(dsp, "owner").ok_or_else(|| "missing DSP owner".to_string())?;
    if dsp_owner != DSP_OWNER {
        return Err("DSP ownership mismatch".to_string());
    }

    let detail_status = string_value(dsp, "detail_status")
        .ok_or_else(|| "missing DSP detail_status".to_string())?;
    let input_device = optional_string(dsp, "input_device")?;
    let output_device = optional_string(dsp, "output_device")?;
    let profile = optional_string(dsp, "profile")?;

    let typed_adapter = parse_typed_adapter(dsp)?;

    if detail_status != DSP_PENDING
        && (input_device.is_none() || output_device.is_none() || profile.is_none())
    {
        return Err(
            "DSP detail claims availability while required typed values are null".to_string(),
        );
    }

    Ok(ConsumerView {
        bridge_version,
        protocol_version,
        control_center_role: role,
        live_control,
        display_owner,
        displays,
        dsp: DspView {
            owner: dsp_owner,
            health: string_value(dsp, "health").ok_or_else(|| "missing DSP health".to_string())?,
            detail_status,
            input_device,
            output_device,
            profile,
            typed_adapter,
        },
    })
}

fn json_value_json(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "null".to_string(),
        JsonValue::Bool(value) => value.to_string(),
        JsonValue::Number(value) => {
            if value.fract() == 0.0 {
                format!("{value:.0}")
            } else {
                value.to_string()
            }
        }
        JsonValue::String(value) => format!("\"{}\"", escape_json(value)),
        JsonValue::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(json_value_json)
                .collect::<Vec<_>>()
                .join(",")
        ),
        JsonValue::Object(map) => {
            let fields = map
                .iter()
                .map(|(key, value)| format!("\"{}\":{}", escape_json(key), json_value_json(value)))
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{fields}}}")
        }
    }
}

fn option_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn typed_adapter_json(view: &TypedAdapterView) -> String {
    let reason = option_json(&view.reason);
    let snapshot = view
        .snapshot
        .as_ref()
        .map(json_value_json)
        .unwrap_or_else(|| "null".to_string());

    format!(
        concat!(
            "{{\"source\":\"{}\",\"adapter_version\":\"{}\",",
            "\"control_mode\":\"{}\",\"status\":\"{}\",",
            "\"state_revision\":{},\"reason\":{},\"snapshot\":{}}}"
        ),
        escape_json(&view.source),
        escape_json(&view.adapter_version),
        escape_json(&view.control_mode),
        escape_json(&view.status),
        option_u64_json(view.state_revision),
        reason,
        snapshot,
    )
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn option_json(value: &Option<String>) -> String {
    match value {
        Some(value) => format!("\"{}\"", escape_json(value)),
        None => "null".to_string(),
    }
}

fn consumer_json(view: &ConsumerView) -> String {
    let displays = view
        .displays
        .iter()
        .map(|display| {
            format!(
                concat!(
                    "{{\"id\":\"{}\",\"name\":\"{}\",",
                    "\"current_physical\":\"{}\",\"canonical_family\":\"{}\",",
                    "\"native_layout\":\"{}\",\"semi_native_layout\":\"{}\",",
                    "\"orientation\":\"{}\"}}"
                ),
                escape_json(&display.id),
                escape_json(&display.name),
                escape_json(&display.current_physical),
                escape_json(&display.canonical_family),
                escape_json(&display.native_layout),
                escape_json(&display.semi_native_layout),
                escape_json(&display.orientation),
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    let typed_adapter = typed_adapter_json(&view.dsp.typed_adapter);

    format!(
        concat!(
            "{{\"source\":\"{}\",\"version\":\"{}\",\"bridge_version\":\"{}\",",
            "\"protocol_version\":{},\"read_only\":true,\"control_actions\":\"DISABLED\",",
            "\"display_owner\":\"{}\",\"displays\":[{}],",
            "\"dsp\":{{\"owner\":\"{}\",\"health\":\"{}\",",
            "\"detail_status\":\"{}\",\"input_device\":{},",
            "\"output_device\":{},\"profile\":{},\"typed_adapter\":{}}}}}"
        ),
        OUTPUT_SOURCE,
        VERSION,
        escape_json(&view.bridge_version),
        view.protocol_version,
        escape_json(&view.display_owner),
        displays,
        escape_json(&view.dsp.owner),
        escape_json(&view.dsp.health),
        escape_json(&view.dsp.detail_status),
        option_json(&view.dsp.input_device),
        option_json(&view.dsp.output_device),
        option_json(&view.dsp.profile),
        typed_adapter,
    )
}

fn summary(view: &ConsumerView) -> String {
    let mut lines = vec![
        format!("AETHERFORGE_CONTROL_CENTER_CONSUMER_VERSION={VERSION}"),
        "AETHERFORGE_CONTROL_CENTER_BRIDGE=READ_ONLY".to_string(),
        format!(
            "AETHERFORGE_CONTROL_CENTER_BRIDGE_VERSION={}",
            view.bridge_version
        ),
        format!("CONTROL_CENTER_ROLE={}", view.control_center_role),
        format!("DISPLAY_OWNER={}", view.display_owner),
        format!("DISPLAY_COUNT={}", view.displays.len()),
    ];

    for (index, display) in view.displays.iter().enumerate() {
        let n = index + 1;
        lines.push(format!("DISPLAY_{n}_NAME={}", display.name));
        lines.push(format!("DISPLAY_{n}_CURRENT={}", display.current_physical));
        lines.push(format!(
            "DISPLAY_{n}_CANONICAL={}",
            display.canonical_family
        ));
        lines.push(format!("DISPLAY_{n}_NATIVE={}", display.native_layout));
        lines.push(format!(
            "DISPLAY_{n}_SEMI_NATIVE={}",
            display.semi_native_layout
        ));
        lines.push(format!("DISPLAY_{n}_ORIENTATION={}", display.orientation));
    }

    lines.push(format!("DSP_OWNER={}", view.dsp.owner));
    lines.push(format!("DSP_HEALTH={}", view.dsp.health));
    lines.push(format!("DSP_DETAIL_STATUS={}", view.dsp.detail_status));
    lines.push(format!(
        "DSP_INPUT={}",
        view.dsp.input_device.as_deref().unwrap_or("N/A")
    ));
    lines.push(format!(
        "DSP_OUTPUT={}",
        view.dsp.output_device.as_deref().unwrap_or("N/A")
    ));
    lines.push(format!(
        "DSP_PROFILE={}",
        view.dsp.profile.as_deref().unwrap_or("N/A")
    ));
    lines.push(format!(
        "DSP_TYPED_SOURCE={}",
        view.dsp.typed_adapter.source
    ));
    lines.push(format!(
        "DSP_TYPED_ADAPTER_VERSION={}",
        view.dsp.typed_adapter.adapter_version
    ));
    lines.push(format!(
        "DSP_TYPED_CONTROL_MODE={}",
        view.dsp.typed_adapter.control_mode
    ));
    lines.push(format!(
        "DSP_TYPED_STATUS={}",
        view.dsp.typed_adapter.status
    ));
    lines.push(format!(
        "DSP_TYPED_STATE_REVISION={}",
        view.dsp
            .typed_adapter
            .state_revision
            .map(|value| value.to_string())
            .unwrap_or_else(|| "N/A".to_string())
    ));
    lines.push(format!(
        "DSP_TYPED_REASON={}",
        view.dsp.typed_adapter.reason.as_deref().unwrap_or("N/A")
    ));
    lines.push(format!(
        "DSP_TYPED_SNAPSHOT={}",
        if view.dsp.typed_adapter.snapshot.is_some() {
            "AVAILABLE"
        } else {
            "N/A"
        }
    ));
    lines.push("CONTROL_ACTIONS=DISABLED".to_string());
    lines.push("AETHERFORGE_CONTROL_CENTER_CONSUMER=PASS".to_string());

    lines.join("\n")
}

fn state_path() -> PathBuf {
    let state_home = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("."));
    state_home.join("aetherforge/control-bridge/snapshot.json")
}

fn load_live() -> Result<ConsumerView, String> {
    let path = state_path();
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    parse_consumer_view(&text)
}

fn run_summary() -> i32 {
    match load_live() {
        Ok(view) => {
            println!("{}", summary(&view));
            0
        }
        Err(error) => {
            eprintln!("AETHERFORGE_CONTROL_CENTER_CONSUMER=FAIL:{error}");
            1
        }
    }
}

fn run_json() -> i32 {
    match load_live() {
        Ok(view) => {
            println!("{}", consumer_json(&view));
            println!("AETHERFORGE_CONTROL_CENTER_CONSUMER=PASS");
            0
        }
        Err(error) => {
            eprintln!("AETHERFORGE_CONTROL_CENTER_CONSUMER=FAIL:{error}");
            1
        }
    }
}

fn fixture() -> &'static str {
    r#"{
      "source":"AETHERFORGE_CONTROL_BRIDGE_READONLY_V1",
      "version":"10.2.91",
      "protocol_version":1,
      "control_center_role":"READ_ONLY_CLIENT",
      "live_control":false,
      "display":{
        "owner":"AETHERFORGE_DISPLAY_SERVICE",
        "health":"HEALTHY",
        "state_source":"AETHERFORGE_DISPLAY_AUTOSCAN_NORMALIZED_V1",
        "outputs":[{
          "id":"1",
          "name":"HDMI-A-1",
          "current_physical":"1920x1080",
          "canonical_family":"1920x1080",
          "native_layout":"1914x1080",
          "semi_native_layout":"2554x1440",
          "orientation":"landscape",
          "priority":1
        }]
      },
      "dsp":{
        "owner":"AETHERSTREAM",
        "health":"OFFLINE",
        "audiod_active":false,
        "supervisor_active":false,
        "supervisor_socket_present":true,
        "detail_status":"UNAVAILABLE_UNTIL_TYPED_SNAPSHOT_ADAPTER",
        "input_device":null,
        "output_device":null,
        "profile":null,
        "bass_boost_db":null,
        "clarity_boost_db":null,
        "low_pass_hz":null,
        "high_pass_hz":null,
        "typed_adapter":{
          "source":"AETHERSTREAM_TYPED_CONTROL_CENTER_ADAPTER_V1",
          "adapter_version":"10.2.88",
          "control_mode":"READ_ONLY_GET_STATE",
          "status":"OFFLINE",
          "state_revision":null,
          "reason":"fixture supervisor offline",
          "snapshot":null
        }
      }
    }"#
}

#[cfg(test)]
fn online_fixture() -> String {
    fixture().replace(
        r#""status":"OFFLINE",
          "state_revision":null,
          "reason":"fixture supervisor offline",
          "snapshot":null"#,
        r#""status":"ONLINE",
          "state_revision":77,
          "reason":null,
          "snapshot":{
            "source":"AETHERSTREAM_TYPED_CONTROL_CENTER_ADAPTER_V1",
            "adapter_version":"10.2.88",
            "aetherstream_version":"10.2.50",
            "ipc_protocol_version":1,
            "control_mode":"READ_ONLY_GET_STATE",
            "service_snapshot":{"connected_clients":2,"services":{"audiod":"Healthy"}},
            "state":{"revision":77,"audio":{"selected_input":"fixture"}}
          }"#,
    )
}

fn self_test() -> i32 {
    let view = match parse_consumer_view(fixture()) {
        Ok(view) => view,
        Err(error) => {
            eprintln!("AETHERFORGE_CONTROL_CENTER_CONSUMER_SELF_TEST=FAIL:{error}");
            return 1;
        }
    };

    let text = summary(&view);
    for required in [
        "AETHERFORGE_CONTROL_CENTER_BRIDGE=READ_ONLY",
        "DISPLAY_1_CURRENT=1920x1080",
        "DISPLAY_1_NATIVE=1914x1080",
        "DISPLAY_1_SEMI_NATIVE=2554x1440",
        "DSP_OWNER=AETHERSTREAM",
        "DSP_HEALTH=OFFLINE",
        "DSP_DETAIL_STATUS=UNAVAILABLE_UNTIL_TYPED_SNAPSHOT_ADAPTER",
        "DSP_TYPED_SOURCE=AETHERSTREAM_TYPED_CONTROL_CENTER_ADAPTER_V1",
        "DSP_TYPED_ADAPTER_VERSION=10.2.88",
        "DSP_TYPED_CONTROL_MODE=READ_ONLY_GET_STATE",
        "DSP_TYPED_STATUS=OFFLINE",
        "DSP_TYPED_STATE_REVISION=N/A",
        "DSP_TYPED_SNAPSHOT=N/A",
        "CONTROL_ACTIONS=DISABLED",
    ] {
        if !text.contains(required) {
            eprintln!("AETHERFORGE_CONTROL_CENTER_CONSUMER_SELF_TEST=FAIL:MISSING:{required}");
            return 1;
        }
    }

    println!("AETHERFORGE_CONTROL_CENTER_CONSUMER_SELF_TEST=PASS:{VERSION}");
    0
}

fn usage() {
    eprintln!("usage: aetherforge-control-center-bridge-consumer --self-test | --summary | --json");
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let code = match args.as_slice() {
        [flag] if flag == "--self-test" => self_test(),
        [flag] if flag == "--summary" => run_summary(),
        [flag] if flag == "--json" => run_json(),
        _ => {
            usage();
            2
        }
    };
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wrong_bridge_source() {
        let input = fixture().replace(INPUT_SOURCE, "WRONG_SOURCE");
        assert!(parse_consumer_view(&input).is_err());
    }

    #[test]
    fn rejects_wrong_protocol_version() {
        let input = fixture().replace("\"protocol_version\":1", "\"protocol_version\":2");
        assert!(parse_consumer_view(&input).is_err());
    }

    #[test]
    fn rejects_live_control_true() {
        let input = fixture().replace("\"live_control\":false", "\"live_control\":true");
        assert!(parse_consumer_view(&input).is_err());
    }

    #[test]
    fn rejects_wrong_display_owner() {
        let input = fixture().replace(DISPLAY_OWNER, "CONTROL_CENTER");
        assert!(parse_consumer_view(&input).is_err());
    }

    #[test]
    fn rejects_wrong_dsp_owner() {
        let input = fixture().replace(DSP_OWNER, "CONTROL_CENTER");
        assert!(parse_consumer_view(&input).is_err());
    }

    #[test]
    fn preserves_1080p_native_and_semi_native_mapping() {
        let view = parse_consumer_view(fixture()).unwrap();
        let display = &view.displays[0];
        assert_eq!(display.current_physical, "1920x1080");
        assert_eq!(display.native_layout, "1914x1080");
        assert_eq!(display.semi_native_layout, "2554x1440");
    }

    #[test]
    fn canonical_768p_mapping_is_supported() {
        let input = fixture()
            .replace("1920x1080", "1366x768")
            .replace("1914x1080", "1360x768")
            .replace("2554x1440", "1914x1080");
        let view = parse_consumer_view(&input).unwrap();
        let display = &view.displays[0];
        assert_eq!(display.current_physical, "1366x768");
        assert_eq!(display.native_layout, "1360x768");
        assert_eq!(display.semi_native_layout, "1914x1080");
    }

    #[test]
    fn offline_dsp_with_pending_null_values_is_truthful_and_valid() {
        let view = parse_consumer_view(fixture()).unwrap();
        assert_eq!(view.dsp.health, "OFFLINE");
        assert_eq!(view.dsp.detail_status, DSP_PENDING);
        assert_eq!(view.dsp.input_device, None);
        assert_eq!(view.dsp.output_device, None);
        assert_eq!(view.dsp.profile, None);
        assert_eq!(view.dsp.typed_adapter.status, "OFFLINE");
        assert_eq!(view.dsp.typed_adapter.state_revision, None);
        assert_eq!(view.dsp.typed_adapter.snapshot, None);
    }

    #[test]
    fn ready_detail_status_cannot_hide_null_typed_values() {
        let input = fixture().replace(DSP_PENDING, "TYPED_SNAPSHOT_READY");
        assert!(parse_consumer_view(&input).is_err());
    }

    #[test]
    fn summary_is_ui_ready_and_read_only() {
        let view = parse_consumer_view(fixture()).unwrap();
        let output = summary(&view);
        assert!(output.contains("DISPLAY_1_NAME=HDMI-A-1"));
        assert!(output.contains("DSP_HEALTH=OFFLINE"));
        assert!(output.contains("CONTROL_ACTIONS=DISABLED"));
    }

    #[test]
    fn normalized_json_is_read_only() {
        let view = parse_consumer_view(fixture()).unwrap();
        let output = consumer_json(&view);
        assert!(output.contains(r#""source":"AETHERFORGE_CONTROL_CENTER_CONSUMER_V1""#));
        assert!(output.contains(r#""read_only":true"#));
        assert!(output.contains(r#""control_actions":"DISABLED""#));
    }

    #[test]
    fn control_center_role_must_be_read_only_client() {
        let input = fixture().replace(CONTROL_CENTER_ROLE, "OWNER");
        assert!(parse_consumer_view(&input).is_err());
    }
    #[test]
    fn rejects_wrong_bridge_version_for_typed_consumer() {
        let input = fixture().replace("\"version\":\"10.2.91\"", "\"version\":\"10.2.79\"");
        assert!(parse_consumer_view(&input).is_err());
    }

    #[test]
    fn rejects_wrong_typed_adapter_source() {
        let input = fixture().replace(TYPED_ADAPTER_SOURCE, "WRONG_TYPED_SOURCE");
        assert!(parse_consumer_view(&input).is_err());
    }

    #[test]
    fn rejects_wrong_typed_adapter_version() {
        let input = fixture().replace(
            "\"adapter_version\":\"10.2.88\"",
            "\"adapter_version\":\"10.2.87\"",
        );
        assert!(parse_consumer_view(&input).is_err());
    }

    #[test]
    fn rejects_non_read_only_typed_control_mode() {
        let input = fixture().replace(TYPED_ADAPTER_CONTROL_MODE, "LIVE_CONTROL");
        assert!(parse_consumer_view(&input).is_err());
    }

    #[test]
    fn rejects_unknown_typed_status() {
        let input = fixture().replace("\"status\":\"OFFLINE\"", "\"status\":\"MYSTERY\"");
        assert!(parse_consumer_view(&input).is_err());
    }

    #[test]
    fn offline_typed_adapter_rejects_non_null_snapshot() {
        let input = fixture().replace(
            "\"snapshot\":null",
            r#""snapshot":{"source":"AETHERSTREAM_TYPED_CONTROL_CENTER_ADAPTER_V1"}"#,
        );
        assert!(parse_consumer_view(&input).is_err());
    }

    #[test]
    fn online_typed_snapshot_is_validated_and_preserved() {
        let input = online_fixture();
        let view = parse_consumer_view(&input).unwrap();
        assert_eq!(view.dsp.typed_adapter.status, "ONLINE");
        assert_eq!(view.dsp.typed_adapter.state_revision, Some(77));
        assert!(view.dsp.typed_adapter.snapshot.is_some());

        let normalized = consumer_json(&view);
        assert!(normalized.contains(
            r#""typed_adapter":{"source":"AETHERSTREAM_TYPED_CONTROL_CENTER_ADAPTER_V1""#
        ));
        assert!(normalized.contains(r#""state_revision":77"#));
        assert!(
            normalized.contains(r#""state":{"audio":{"selected_input":"fixture"},"revision":77}"#)
                || normalized
                    .contains(r#""state":{"revision":77,"audio":{"selected_input":"fixture"}}"#)
        );
    }

    #[test]
    fn online_typed_snapshot_rejects_revision_mismatch() {
        let input =
            online_fixture().replace(r#""state":{"revision":77"#, r#""state":{"revision":78"#);
        assert!(parse_consumer_view(&input).is_err());
    }

    #[test]
    fn typed_summary_exposes_source_status_and_revision() {
        let offline = summary(&parse_consumer_view(fixture()).unwrap());
        assert!(offline.contains("DSP_TYPED_STATUS=OFFLINE"));
        assert!(offline.contains("DSP_TYPED_STATE_REVISION=N/A"));
        assert!(offline.contains("DSP_TYPED_SNAPSHOT=N/A"));

        let online = summary(&parse_consumer_view(&online_fixture()).unwrap());
        assert!(online.contains("DSP_TYPED_STATUS=ONLINE"));
        assert!(online.contains("DSP_TYPED_STATE_REVISION=77"));
        assert!(online.contains("DSP_TYPED_SNAPSHOT=AVAILABLE"));
    }

    #[test]
    fn legacy_dsp_fields_stay_null_while_typed_adapter_is_offline() {
        let view = parse_consumer_view(fixture()).unwrap();
        assert_eq!(view.dsp.detail_status, DSP_PENDING);
        assert_eq!(view.dsp.input_device, None);
        assert_eq!(view.dsp.output_device, None);
        assert_eq!(view.dsp.profile, None);
    }
}
