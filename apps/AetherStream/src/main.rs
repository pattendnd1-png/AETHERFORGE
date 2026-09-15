use aetherstream_control::{
    AetherState, AlertState, AudioBusState, AuthState, CreatorOpsState, DeckState, MediaState,
    ObsState, OutputDspState, OverlayState, RecordingState, StreamElementsOpsState, StreamState,
    StreamlabsOpsState,
};
use aetherstream_core::{ClientInstanceId, ServiceHealth, SessionId};
use aetherstream_ipc::{
    ClientHello, ControlRequest, ControlResponse, FrameCodec, IPC_PROTOCOL_VERSION, IpcEnvelope,
    ServiceSnapshot,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

const VERSION: &str = "10.2.94";
const SOURCE: &str = "AETHERSTREAM_TYPED_CONTROL_CENTER_ADAPTER_V1";
const CONTRACT_VERSION: &str = "10.2.50";
const SCHEMA_47: &str = "10.2.47";
const SCHEMA_50: &str = "10.2.50";
const MAX_FRAME: usize = 8 * 1024 * 1024;
const TIMEOUT_MS: u64 = 1500;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct LinkedOutputStateV47 {
    enabled: bool,
    hdmi_node_name: Option<String>,
    bluetooth_node_name: Option<String>,
    master_normalized_10k: u16,
    hdmi_trim_db_x100: i16,
    bluetooth_trim_db_x100: i16,
    latency_compensated: bool,
    status_detail: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct AudioStateV47 {
    buses: BTreeMap<String, AudioBusState>,
    output_dsp: OutputDspState,
    linked_output: LinkedOutputStateV47,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
struct AetherStateV47 {
    revision: u64,
    auth: AuthState,
    active_session: Option<SessionId>,
    stream: StreamState,
    creator: CreatorOpsState,
    obs: ObsState,
    audio: AudioStateV47,
    recording: RecordingState,
    overlays: OverlayState,
    streamelements_ops: StreamElementsOpsState,
    streamlabs_ops: StreamlabsOpsState,
    alerts: AlertState,
    deck: DeckState,
    media: MediaState,
    services: BTreeMap<String, ServiceHealth>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum ControlResponseV47 {
    StateSnapshot(Box<AetherStateV47>),
}

#[derive(Debug)]
struct DecodedState {
    schema_version: &'static str,
    revision: u64,
    state_json: Value,
}

fn socket_path() -> PathBuf {
    if let Some(dir) = env::var_os("XDG_RUNTIME_DIR") {
        PathBuf::from(dir).join("aetherstream/supervisor.sock")
    } else {
        PathBuf::from(format!(
            "/tmp/aetherstream-{}/supervisor.sock",
            env::var("UID").unwrap_or_else(|_| "user".into())
        ))
    }
}

fn state_path() -> PathBuf {
    let root = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from("."));
    root.join("aetherforge/aetherstream/typed-snapshot.json")
}

fn read_only_request() -> ControlRequest {
    ControlRequest::GetState
}

fn write_frame<T: serde::Serialize>(stream: &mut UnixStream, value: &T) -> Result<(), String> {
    let frame = FrameCodec::encode(value).map_err(|error| format!("encode: {error}"))?;
    stream
        .write_all(&frame)
        .map_err(|error| format!("socket write: {error}"))
}

fn read_raw_frame(stream: &mut UnixStream) -> Result<Vec<u8>, String> {
    let mut prefix = [0u8; 4];
    stream
        .read_exact(&mut prefix)
        .map_err(|error| format!("frame prefix read: {error}"))?;
    let payload_len = u32::from_be_bytes(prefix) as usize;
    if payload_len > MAX_FRAME {
        return Err(format!("frame too large: {payload_len}"));
    }

    let mut frame = Vec::with_capacity(payload_len + 4);
    frame.extend_from_slice(&prefix);
    frame.resize(payload_len + 4, 0);
    stream
        .read_exact(&mut frame[4..])
        .map_err(|error| format!("frame payload read: {error}"))?;
    Ok(frame)
}

fn read_frame<T: serde::de::DeserializeOwned>(stream: &mut UnixStream) -> Result<T, String> {
    let frame = read_raw_frame(stream)?;
    FrameCodec::decode(&frame).map_err(|error| format!("decode: {error}"))
}

fn decode_current(frame: &[u8]) -> Result<DecodedState, String> {
    let reply: IpcEnvelope<ControlResponse> =
        FrameCodec::decode(frame).map_err(|error| format!("10.2.50 decode: {error}"))?;
    if reply.protocol_version != IPC_PROTOCOL_VERSION {
        return Err(format!(
            "10.2.50 protocol mismatch: {}",
            reply.protocol_version
        ));
    }
    match reply.payload {
        ControlResponse::StateSnapshot(state) => {
            let revision = state.revision;
            let state_json =
                serde_json::to_value(*state).map_err(|error| format!("10.2.50 json: {error}"))?;
            Ok(DecodedState {
                schema_version: SCHEMA_50,
                revision,
                state_json,
            })
        }
        _ => Err("10.2.50 response was not StateSnapshot".into()),
    }
}

fn decode_legacy_47(frame: &[u8]) -> Result<DecodedState, String> {
    let reply: IpcEnvelope<ControlResponseV47> =
        FrameCodec::decode(frame).map_err(|error| format!("10.2.47 decode: {error}"))?;
    if reply.protocol_version != IPC_PROTOCOL_VERSION {
        return Err(format!(
            "10.2.47 protocol mismatch: {}",
            reply.protocol_version
        ));
    }
    match reply.payload {
        ControlResponseV47::StateSnapshot(state) => {
            let revision = state.revision;
            let state_json =
                serde_json::to_value(*state).map_err(|error| format!("10.2.47 json: {error}"))?;
            Ok(DecodedState {
                schema_version: SCHEMA_47,
                revision,
                state_json,
            })
        }
    }
}

fn decode_state_frame(frame: &[u8]) -> Result<DecodedState, String> {
    let current = decode_current(frame);
    let legacy = decode_legacy_47(frame);

    match (current, legacy) {
        (Ok(state), Err(_)) => Ok(state),
        (Err(_), Ok(state)) => Ok(state),
        (Ok(_), Ok(_)) => Err("ambiguous state frame: both 10.2.47 and 10.2.50 decoded".into()),
        (Err(current_error), Err(legacy_error)) => Err(format!(
            "state decode failed; current={current_error}; legacy={legacy_error}"
        )),
    }
}

fn connect_and_get_state() -> Result<(ServiceSnapshot, DecodedState), String> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path)
        .map_err(|error| format!("connect {}: {error}", path.display()))?;
    let timeout = Some(Duration::from_millis(TIMEOUT_MS));
    stream
        .set_read_timeout(timeout)
        .map_err(|error| format!("set read timeout: {error}"))?;
    stream
        .set_write_timeout(timeout)
        .map_err(|error| format!("set write timeout: {error}"))?;

    let hello = IpcEnvelope::new(ClientHello {
        client: ClientInstanceId::new(),
        pid: std::process::id(),
    });
    if hello.protocol_version != IPC_PROTOCOL_VERSION {
        return Err("constructed hello protocol version mismatch".into());
    }
    write_frame(&mut stream, &hello)?;

    let service_reply: IpcEnvelope<ServiceSnapshot> = read_frame(&mut stream)?;
    if service_reply.protocol_version != IPC_PROTOCOL_VERSION {
        return Err(format!(
            "service snapshot protocol mismatch: {}",
            service_reply.protocol_version
        ));
    }

    let request = IpcEnvelope::new(read_only_request());
    if request.protocol_version != IPC_PROTOCOL_VERSION {
        return Err("constructed request protocol version mismatch".into());
    }
    write_frame(&mut stream, &request)?;

    let frame = read_raw_frame(&mut stream)?;
    let state = decode_state_frame(&frame)?;
    Ok((service_reply.payload, state))
}

fn services_json(snapshot: &ServiceSnapshot) -> Value {
    let services = snapshot
        .services
        .iter()
        .map(|(name, health)| (name.clone(), Value::String(format!("{health:?}"))))
        .collect::<serde_json::Map<String, Value>>();

    json!({
        "connected_clients": snapshot.connected_clients,
        "services": services,
    })
}

fn snapshot_document(service_snapshot: &ServiceSnapshot, state: &DecodedState) -> Value {
    json!({
        "source": SOURCE,
        "adapter_version": VERSION,
        "aetherstream_version": CONTRACT_VERSION,
        "aetherstream_version_semantics": "ADAPTER_CONTRACT_SCHEMA",
        "decoded_state_schema_version": state.schema_version,
        "ipc_protocol_version": IPC_PROTOCOL_VERSION,
        "control_mode": "READ_ONLY_GET_STATE",
        "socket": socket_path(),
        "service_snapshot": services_json(service_snapshot),
        "state": state.state_json,
    })
}

fn write_atomic(value: &Value) -> Result<PathBuf, String> {
    let path = state_path();
    let parent = path
        .parent()
        .ok_or_else(|| "typed snapshot path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("create {}: {error}", parent.display()))?;
    let temp = parent.join(format!("typed-snapshot.json.tmp.{}", std::process::id()));
    let payload = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("serialize typed snapshot: {error}"))?;
    fs::write(&temp, payload).map_err(|error| format!("write {}: {error}", temp.display()))?;
    fs::rename(&temp, &path).map_err(|error| format!("replace {}: {error}", path.display()))?;
    Ok(path)
}

fn run_snapshot() -> i32 {
    match connect_and_get_state() {
        Ok((service_snapshot, state)) => {
            let document = snapshot_document(&service_snapshot, &state);
            match write_atomic(&document).and_then(|path| {
                let text = serde_json::to_string(&document)
                    .map_err(|error| format!("render snapshot json: {error}"))?;
                Ok((path, text))
            }) {
                Ok((path, text)) => {
                    println!("AETHERSTREAM_TYPED_ADAPTER_STATUS=ONLINE");
                    println!("AETHERSTREAM_TYPED_DECODED_SCHEMA={}", state.schema_version);
                    println!("AETHERSTREAM_TYPED_STATE_REVISION={}", state.revision);
                    println!("AETHERSTREAM_TYPED_SNAPSHOT_FILE={}", path.display());
                    println!("AETHERSTREAM_TYPED_SNAPSHOT_JSON={text}");
                    println!("AETHERSTREAM_TYPED_ADAPTER=PASS");
                    0
                }
                Err(error) => {
                    eprintln!("AETHERSTREAM_TYPED_ADAPTER=FAIL:{error}");
                    1
                }
            }
        }
        Err(error) => {
            println!("AETHERSTREAM_TYPED_ADAPTER_STATUS=OFFLINE");
            println!("AETHERSTREAM_TYPED_ADAPTER_REASON={error}");
            println!("AETHERSTREAM_TYPED_ADAPTER=OFFLINE");
            3
        }
    }
}

fn self_test() -> i32 {
    let request = IpcEnvelope::new(read_only_request());
    if request.protocol_version != IPC_PROTOCOL_VERSION {
        eprintln!("AETHERSTREAM_TYPED_ADAPTER_SELF_TEST=FAIL:PROTOCOL");
        return 1;
    }

    let frame = match FrameCodec::encode(&request) {
        Ok(frame) => frame,
        Err(error) => {
            eprintln!("AETHERSTREAM_TYPED_ADAPTER_SELF_TEST=FAIL:ENCODE:{error}");
            return 1;
        }
    };

    let decoded = match FrameCodec::decode::<IpcEnvelope<ControlRequest>>(&frame) {
        Ok(decoded) => decoded,
        Err(error) => {
            eprintln!("AETHERSTREAM_TYPED_ADAPTER_SELF_TEST=FAIL:DECODE:{error}");
            return 1;
        }
    };
    if decoded.protocol_version != IPC_PROTOCOL_VERSION
        || !matches!(decoded.payload, ControlRequest::GetState)
    {
        eprintln!("AETHERSTREAM_TYPED_ADAPTER_SELF_TEST=FAIL:ROUND_TRIP");
        return 1;
    }

    let current_state = AetherState {
        revision: 50,
        ..AetherState::default()
    };
    let current_frame = match FrameCodec::encode(&IpcEnvelope::new(ControlResponse::StateSnapshot(
        Box::new(current_state),
    ))) {
        Ok(frame) => frame,
        Err(error) => {
            eprintln!("AETHERSTREAM_TYPED_ADAPTER_SELF_TEST=FAIL:CURRENT_ENCODE:{error}");
            return 1;
        }
    };
    match decode_state_frame(&current_frame) {
        Ok(decoded) if decoded.schema_version == SCHEMA_50 && decoded.revision == 50 => {}
        Ok(decoded) => {
            eprintln!(
                "AETHERSTREAM_TYPED_ADAPTER_SELF_TEST=FAIL:CURRENT_SCHEMA:{}:{}",
                decoded.schema_version, decoded.revision
            );
            return 1;
        }
        Err(error) => {
            eprintln!("AETHERSTREAM_TYPED_ADAPTER_SELF_TEST=FAIL:CURRENT_DECODE:{error}");
            return 1;
        }
    }

    let legacy_state = AetherStateV47 {
        revision: 47,
        ..AetherStateV47::default()
    };
    let legacy_frame = match FrameCodec::encode(&IpcEnvelope::new(
        ControlResponseV47::StateSnapshot(Box::new(legacy_state)),
    )) {
        Ok(frame) => frame,
        Err(error) => {
            eprintln!("AETHERSTREAM_TYPED_ADAPTER_SELF_TEST=FAIL:LEGACY_ENCODE:{error}");
            return 1;
        }
    };
    match decode_state_frame(&legacy_frame) {
        Ok(decoded) if decoded.schema_version == SCHEMA_47 && decoded.revision == 47 => {}
        Ok(decoded) => {
            eprintln!(
                "AETHERSTREAM_TYPED_ADAPTER_SELF_TEST=FAIL:LEGACY_SCHEMA:{}:{}",
                decoded.schema_version, decoded.revision
            );
            return 1;
        }
        Err(error) => {
            eprintln!("AETHERSTREAM_TYPED_ADAPTER_SELF_TEST=FAIL:LEGACY_DECODE:{error}");
            return 1;
        }
    }

    if !socket_path()
        .to_string_lossy()
        .ends_with("aetherstream/supervisor.sock")
    {
        eprintln!("AETHERSTREAM_TYPED_ADAPTER_SELF_TEST=FAIL:SOCKET_PATH");
        return 1;
    }

    println!("AETHERSTREAM_TYPED_ADAPTER_SELF_TEST=PASS:{VERSION}");
    0
}

fn usage() {
    eprintln!("usage: aetherforge-aetherstream-typed-adapter --self-test | --snapshot");
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let code = match args.as_slice() {
        [flag] if flag == "--self-test" => self_test(),
        [flag] if flag == "--snapshot" => run_snapshot(),
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

    fn legacy_frame(revision: u64) -> Vec<u8> {
        let state = AetherStateV47 {
            revision,
            ..AetherStateV47::default()
        };
        FrameCodec::encode(&IpcEnvelope::new(ControlResponseV47::StateSnapshot(
            Box::new(state),
        )))
        .unwrap()
    }

    fn current_frame(revision: u64) -> Vec<u8> {
        let state = AetherState {
            revision,
            ..AetherState::default()
        };
        FrameCodec::encode(&IpcEnvelope::new(ControlResponse::StateSnapshot(Box::new(
            state,
        ))))
        .unwrap()
    }

    #[test]
    fn exact_ipc_get_state_round_trip() {
        let request = IpcEnvelope::new(read_only_request());
        let frame = FrameCodec::encode(&request).unwrap();
        let decoded: IpcEnvelope<ControlRequest> = FrameCodec::decode(&frame).unwrap();
        assert_eq!(decoded.protocol_version, IPC_PROTOCOL_VERSION);
        assert!(matches!(decoded.payload, ControlRequest::GetState));
    }

    #[test]
    fn v10_2_47_fixture_selects_legacy_schema() {
        let decoded = decode_state_frame(&legacy_frame(47)).unwrap();
        assert_eq!(decoded.schema_version, SCHEMA_47);
        assert_eq!(decoded.revision, 47);
    }

    #[test]
    fn v10_2_50_fixture_selects_current_schema() {
        let decoded = decode_state_frame(&current_frame(50)).unwrap();
        assert_eq!(decoded.schema_version, SCHEMA_50);
        assert_eq!(decoded.revision, 50);
    }

    #[test]
    fn old_adapter_current_only_decoder_rejects_v10_2_47_fixture() {
        let frame = legacy_frame(47);
        assert!(FrameCodec::decode::<IpcEnvelope<ControlResponse>>(&frame).is_err());
    }

    #[test]
    fn normalized_document_keeps_contract_version_and_exposes_real_schema() {
        let decoded = decode_state_frame(&legacy_frame(47)).unwrap();
        let doc = snapshot_document(&ServiceSnapshot::default(), &decoded);
        assert_eq!(doc["aetherstream_version"], CONTRACT_VERSION);
        assert_eq!(
            doc["aetherstream_version_semantics"],
            "ADAPTER_CONTRACT_SCHEMA"
        );
        assert_eq!(doc["decoded_state_schema_version"], SCHEMA_47);
        assert_eq!(doc["state"]["revision"], 47);
    }

    #[test]
    fn adapter_request_constructor_is_get_state_only() {
        assert!(matches!(read_only_request(), ControlRequest::GetState));
    }

    #[test]
    fn output_source_is_stable() {
        assert_eq!(SOURCE, "AETHERSTREAM_TYPED_CONTROL_CENTER_ADAPTER_V1");
    }
}
