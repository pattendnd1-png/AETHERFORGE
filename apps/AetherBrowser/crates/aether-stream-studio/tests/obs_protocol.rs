use aether_stream_studio::{
    ObsWebSocketClient, ObsWebSocketConfig, StudioControlIntent, obs_authentication,
    verify_obs_control_plane,
};
use serde_json::{Value, json};
use std::net::TcpListener;
use std::thread;
use tungstenite::{Message, accept};

#[test]
fn obs_defaults_to_local_websocket_v5_port() {
    let config = ObsWebSocketConfig::default();
    assert_eq!(config.url, "ws://127.0.0.1:4455");
    assert_eq!(config.password, None);
}

#[test]
fn obs_authentication_matches_official_protocol_example() {
    let authentication = obs_authentication(
        "supersecretpassword",
        "lM1GncleQOaCu9lT1yeUZhFYnqhsLLP1G5lAGo3ixaI=",
        "+IxH4CnCiqpX1rM9scsNynZzbOe4KhDeYcTNS3PDaeY=",
    );
    assert_eq!(
        authentication,
        "1Ct943GAT+6YQUUX47Ia/ncufilbe6+oD6lY+5kaCu4="
    );
}

#[test]
fn obs_client_completes_v5_handshake_status_and_control_request() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock OBS server");
    let address = listener.local_addr().expect("mock OBS address");
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept mock OBS client");
        let mut socket = accept(stream).expect("upgrade mock OBS websocket");
        socket
            .send(Message::text(
                json!({
                    "op": 0,
                    "d": {
                        "obsStudioVersion": "31.0.0",
                        "obsWebSocketVersion": "5.6.0",
                        "rpcVersion": 1
                    }
                })
                .to_string(),
            ))
            .expect("send Hello");

        let identify = read_json(&mut socket);
        assert_eq!(identify.get("op").and_then(Value::as_u64), Some(1));
        assert_eq!(
            identify.pointer("/d/rpcVersion").and_then(Value::as_u64),
            Some(1)
        );
        socket
            .send(Message::text(
                json!({"op": 2, "d": {"negotiatedRpcVersion": 1}}).to_string(),
            ))
            .expect("send Identified");

        for expected in [
            "GetVersion",
            "GetSceneList",
            "GetStreamStatus",
            "GetRecordStatus",
            "GetReplayBufferStatus",
            "StartStream",
        ] {
            let request = read_json(&mut socket);
            assert_eq!(request.get("op").and_then(Value::as_u64), Some(6));
            let data = request.get("d").expect("request data");
            assert_eq!(
                data.get("requestType").and_then(Value::as_str),
                Some(expected)
            );
            let request_id = data
                .get("requestId")
                .and_then(Value::as_str)
                .expect("request id");
            let response_data = match expected {
                "GetVersion" => json!({
                    "obsVersion": "31.0.0",
                    "obsWebSocketVersion": "5.6.0",
                    "rpcVersion": 1
                }),
                "GetSceneList" => json!({
                    "currentProgramSceneName": "Gaming",
                    "scenes": [{"sceneName": "Gaming"}, {"sceneName": "BRB"}]
                }),
                "GetStreamStatus" => json!({"outputActive": false}),
                "GetRecordStatus" => json!({"outputActive": true}),
                "GetReplayBufferStatus" => json!({"outputActive": true}),
                _ => Value::Null,
            };
            socket
                .send(Message::text(
                    json!({
                        "op": 7,
                        "d": {
                            "requestType": expected,
                            "requestId": request_id,
                            "requestStatus": {"result": true, "code": 100},
                            "responseData": response_data
                        }
                    })
                    .to_string(),
                ))
                .expect("send RequestResponse");
        }
    });

    let mut client = ObsWebSocketClient::connect(ObsWebSocketConfig {
        url: format!("ws://{address}"),
        password: None,
    })
    .expect("connect OBS client");
    let status = client.status().expect("read OBS status");
    assert!(status.connected);
    assert_eq!(status.obs_version, "31.0.0");
    assert_eq!(status.websocket_version, "5.6.0");
    assert_eq!(status.current_scene.as_deref(), Some("Gaming"));
    assert_eq!(status.scenes, vec!["Gaming".to_owned(), "BRB".to_owned()]);
    assert!(!status.streaming);
    assert!(status.recording);
    assert!(status.replay_buffer);
    client
        .send_intent(StudioControlIntent::StartStream)
        .expect("send StartStream");
    server.join().expect("mock OBS server thread");
}

fn read_json<S>(socket: &mut tungstenite::WebSocket<S>) -> Value
where
    S: std::io::Read + std::io::Write,
{
    loop {
        match socket.read().expect("read websocket message") {
            Message::Text(text) => {
                return serde_json::from_str(text.as_str()).expect("parse text JSON");
            }
            Message::Binary(bytes) => {
                return serde_json::from_slice(&bytes).expect("parse binary JSON");
            }
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => continue,
            Message::Close(frame) => panic!("websocket closed unexpectedly: {frame:?}"),
        }
    }
}

#[test]
fn obs_control_plane_verifier_exercises_scratch_scene_and_restores_original() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind verifier mock OBS server");
    let address = listener.local_addr().expect("verifier mock OBS address");
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept verifier client");
        let mut socket = accept(stream).expect("upgrade verifier websocket");
        socket.send(Message::text(json!({"op":0,"d":{"obsStudioVersion":"31.0.0","obsWebSocketVersion":"5.6.0","rpcVersion":1}}).to_string())).expect("hello");
        let identify = read_json(&mut socket);
        assert_eq!(identify.get("op").and_then(Value::as_u64), Some(1));
        socket
            .send(Message::text(
                json!({"op":2,"d":{"negotiatedRpcVersion":1}}).to_string(),
            ))
            .expect("identified");

        let expected = [
            "GetVersion",
            "GetSceneList",
            "GetStreamStatus",
            "GetRecordStatus",
            "GetReplayBufferStatus",
            "GetInputList",
            "GetSceneTransitionList",
            "GetOutputList",
            "GetStats",
            "GetProfileList",
            "GetSceneCollectionList",
            "GetVirtualCamStatus",
            "GetSceneItemList",
            "GetSourceFilterList",
            "CreateScene",
            "SetCurrentProgramScene",
            "GetCurrentProgramScene",
            "SetCurrentProgramScene",
            "GetCurrentProgramScene",
            "RemoveScene",
        ];
        let mut scratch = String::new();
        let mut current = "Gaming".to_owned();
        for request_type in expected {
            let request = read_json(&mut socket);
            let data = request.get("d").expect("verifier request data");
            assert_eq!(
                data.get("requestType").and_then(Value::as_str),
                Some(request_type)
            );
            let request_id = data
                .get("requestId")
                .and_then(Value::as_str)
                .expect("request id");
            if request_type == "CreateScene" {
                scratch = data
                    .pointer("/requestData/sceneName")
                    .and_then(Value::as_str)
                    .expect("scratch name")
                    .to_owned();
            }
            if request_type == "SetCurrentProgramScene" {
                current = data
                    .pointer("/requestData/sceneName")
                    .and_then(Value::as_str)
                    .expect("scene name")
                    .to_owned();
            }
            let response_data = match request_type {
                "GetVersion" => {
                    json!({"obsVersion":"31.0.0","obsWebSocketVersion":"5.6.0","rpcVersion":1})
                }
                "GetSceneList" => {
                    json!({"currentProgramSceneName":"Gaming","scenes":[{"sceneName":"Gaming"}]})
                }
                "GetStreamStatus"
                | "GetRecordStatus"
                | "GetReplayBufferStatus"
                | "GetVirtualCamStatus" => json!({"outputActive":false}),
                "GetCurrentProgramScene" => json!({"currentProgramSceneName":current}),
                "GetInputList" => json!({"inputs":[]}),
                "GetSceneTransitionList" => json!({"transitions":[]}),
                "GetOutputList" => json!({"outputs":[]}),
                "GetStats" => json!({"cpuUsage":0.1}),
                "GetProfileList" => json!({"profiles":["Untitled"]}),
                "GetSceneCollectionList" => json!({"sceneCollections":["Untitled"]}),
                "GetSceneItemList" => json!({"sceneItems":[]}),
                "GetSourceFilterList" => json!({"filters":[]}),
                _ => Value::Null,
            };
            socket.send(Message::text(json!({"op":7,"d":{"requestType":request_type,"requestId":request_id,"requestStatus":{"result":true,"code":100},"responseData":response_data}}).to_string())).expect("verifier response");
        }
        assert!(!scratch.is_empty());
        assert_eq!(current, "Gaming");
    });

    let report = verify_obs_control_plane(ObsWebSocketConfig {
        url: format!("ws://{address}"),
        password: None,
    })
    .expect("verify control plane");
    assert!(report.scratch_scene_ok);
    assert!(report.scene_restore_ok);
    assert!(report.sources_ok && report.inputs_ok && report.filters_ok && report.transitions_ok);
    assert!(
        report.outputs_ok
            && report.stats_ok
            && report.profiles_ok
            && report.scene_collections_ok
            && report.virtual_camera_ok
    );
    server.join().expect("verifier mock server thread");
}
