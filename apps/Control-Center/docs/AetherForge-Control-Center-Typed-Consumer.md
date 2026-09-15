# AetherForge v10.2.93 — Control Center Typed Bridge Consumer

## Scope

This is a **consumer-only** release. The installed v10.2.91 aggregation bridge,
v10.2.88 typed AetherStream adapter, Display service, AetherStream, protocol
binary, Plasma configuration, and `/usr/bin/aether-control-center` are
read-only protected inputs.

## Required producer contract

The consumer requires:

- bridge source `AETHERFORGE_CONTROL_BRIDGE_READONLY_V1`
- bridge version `10.2.91`
- bridge protocol `1`
- `live_control=false`
- Display owner `AETHERFORGE_DISPLAY_SERVICE`
- DSP owner `AETHERSTREAM`
- nested `dsp.typed_adapter`

The nested typed adapter must contain:

- source `AETHERSTREAM_TYPED_CONTROL_CENTER_ADAPTER_V1`
- adapter version `10.2.88`
- control mode `READ_ONLY_GET_STATE`
- status `ONLINE`, `OFFLINE`, `MISSING`, `INVALID`, or `ERROR`

For `ONLINE`, the consumer requires a numeric state revision, null diagnostic
reason, a non-null snapshot, exact AetherStream v10.2.50/protocol-v1 identity,
and the embedded `state.revision` must equal the outer typed-adapter revision.

For every non-online status, revision and snapshot must be null and a
non-empty diagnostic reason is required. This prevents stale typed state from
being normalized as current data.

## Normalized consumer output

The existing output identity stays:
`AETHERFORGE_CONTROL_CENTER_CONSUMER_V1`.

The existing Display and legacy DSP fields remain intact. A new normalized
`dsp.typed_adapter` object carries source/version/control mode/status/revision,
diagnostic reason, and the validated complete typed snapshot when online.

Summary output adds:

- `DSP_TYPED_SOURCE`
- `DSP_TYPED_ADAPTER_VERSION`
- `DSP_TYPED_CONTROL_MODE`
- `DSP_TYPED_STATUS`
- `DSP_TYPED_STATE_REVISION`
- `DSP_TYPED_REASON`
- `DSP_TYPED_SNAPSHOT`

While AetherStream is offline, the existing compatibility fields remain
`DSP_INPUT=N/A`, `DSP_OUTPUT=N/A`, and `DSP_PROFILE=N/A`.

## Install boundary

The launcher backs up and replaces only:

`~/.local/lib/aetherforge/aetherforge-control-center-bridge-consumer`

Before installation, the newly built consumer must parse the live v10.2.91
bridge successfully in both `--summary` and `--json` modes. The installed
consumer is tested again after replacement. On any post-install failure, the
exact v10.2.80 consumer binary is restored.

There is no sudo, no systemctl, no service restart, no KScreen, no PipeWire,
no audio-routing change, no Plasma restart, and no reboot.


## v10.2.93 local Rust qualification correction

The newly installed sandbox Rust 1.98.1 toolchain exposed two packaging/source hygiene defects before host handoff:

1. v10.2.92 source had not been passed through `cargo fmt`; `cargo fmt --check` failed.
2. After formatting, strict Clippy (`-D warnings`) found `online_fixture` compiled in non-test builds even though it is test-only.

v10.2.93 keeps the typed-consumer runtime behavior and ownership boundaries unchanged. It applies canonical rustfmt output and marks the helper `#[cfg(test)]`.

The canonical v10.2.93 source has been locally qualified with Rust 1.98.1:

- `cargo fmt --check` — PASS
- `cargo clippy --all-targets -- -D warnings` — PASS
- `RUSTFLAGS="-D warnings" cargo test --release --all-targets` — PASS, 22/22
- `RUSTFLAGS="-D warnings" cargo build --release` — PASS
- built binary `--self-test` — PASS

Host live-summary/live-JSON/install/rollback gates remain pending until the launcher is run on AetherForge.
