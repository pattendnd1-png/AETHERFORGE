# AetherBrowser v2.1.18 OBS Control Design

## Scope

AetherBrowser v2.1.18 fixes the Rust 1.98.1 `field_reassign_with_default` preinstall blocker in `aether-engine-servo` and turns the existing OBS provider metadata into a real local OBS WebSocket 5.x control path. The browser remains the authoritative Stream Studio UI. AetherStream's supervisor socket remains the preferred AetherForge service boundary when present, while direct local OBS WebSocket control is the concrete OBS compatibility transport.

## OBS transport

`aether-stream-studio` owns a synchronous `ObsWebSocketClient` and protocol helpers. It connects to `ws://127.0.0.1:4455` by default, with `AETHER_OBS_WEBSOCKET_URL` as a local override. The OBS password is read from `AETHER_OBS_WEBSOCKET_PASSWORD` only at connection time and is never logged or copied into rendered browser HTML. The client implements OBS WebSocket 5.x JSON opcodes 0/1/2/5/6/7, negotiates RPC version 1, performs the official SHA-256/base64 challenge response, verifies request IDs, and exposes explicit request errors.

The client requests `GetVersion`, `GetSceneList`, `GetStreamStatus`, `GetRecordStatus`, and `GetReplayBufferStatus` for state. Control intents map to `SetCurrentProgramScene`, `StartStream`, `StopStream`, `StartRecord`, `StopRecord`, `SaveReplayBuffer`, and `SetInputVolume` using dB converted from the existing millidecibel intent.

## Browser wiring

`aether-native-pages` parses `aether://stream` query actions. The Stream Studio page connects to OBS, renders connection/version/current-scene/live/record/replay state, and exposes real `aether://stream?action=...` links/forms for refresh, scene selection, stream, record, replay, and mixer gain. Rendering never claims OBS is connected when the handshake or status request fails; it shows the transport error instead.

The existing AetherStream supervisor socket status remains visible. Direct OBS control does not replace the AetherStream socket contract; it makes the existing OBS compatibility provider operational.

## Servo blocker

The Servo runtime constructs `Opts` with a struct initializer rather than assigning fields after `Opts::default()`, satisfying Rust 1.98.1 Clippy with `-D warnings` without suppressing the lint.

## Security and failure behavior

OBS control defaults to loopback only. Authentication is supported and recommended. The password is not persisted by this release, is not logged, and is not rendered. Unknown Stream Studio actions are ignored and rendered as no-op status refreshes. Failed OBS requests leave AetherBrowser running and display a bounded error message; they do not falsify stream state or bypass package verification.

## Verification

Static regression tests require the Servo struct initializer, real OBS WebSocket opcode/auth/request handling, concrete OBS request names, loopback default, password redaction contract, Stream Studio action links, and v2.1.18 release identity. Host verification must additionally pass `cargo check`, Clippy with warnings denied, full Rust tests, release build, install verification, and the existing runtime/media/stability gates before promotion.
