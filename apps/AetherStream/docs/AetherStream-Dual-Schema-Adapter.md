# AetherForge v10.2.94 — AetherStream Dual-Schema Typed Adapter

## Root cause

The installed v10.2.88 adapter was compiled only against the AetherStream
10.2.50 state schema. The protected live AetherStream audio lineage is the
10.2.47 ForgeHX recovery binary. Exact source comparison shows the IPC envelope,
ClientHello -> ServiceSnapshot -> GetState -> StateSnapshot sequence, and
protocol version remain unchanged.

The serialized state changed inside `audio.linked_output` between 10.2.47 and
10.2.50. 10.2.50 inserted `topology_exact` and eight topology counters before
`status_detail`. Postcard is positional, so decoding a 10.2.47 state as the
10.2.50 type shifts subsequent bytes and can report an invalid bool.

## Repair

The adapter remains a Rust-only, read-only client. It performs the exact same
GetState request, reads the response frame once, and attempts two exact schemas:

1. AetherStream 10.2.50 current schema.
2. AetherStream 10.2.47 compatibility schema.

Exactly one decode must succeed. Both-success is rejected as ambiguous and
both-fail returns the two decode diagnostics. A successful state is normalized
to the existing `AETHERSTREAM_TYPED_CONTROL_CENTER_ADAPTER_V1` JSON contract.

For compatibility with the already-installed v10.2.93 consumer,
`aetherstream_version` remains `10.2.50` and is explicitly labeled with
`aetherstream_version_semantics=ADAPTER_CONTRACT_SCHEMA`.
`decoded_state_schema_version` truthfully records the actual matched wire
schema (`10.2.47` or `10.2.50`).

## Release boundary

Only the typed adapter binary is replaced:

`~/.local/lib/aetherforge/aetherstream-control-center-typed-adapter`

The launcher requires the built v10.2.94 binary to obtain a live ONLINE
snapshot **before** installation. If live decode does not succeed, the installed
v10.2.88 adapter remains untouched.

No AetherStream binary, ForgeHX routing, PipeWire graph, bridge, consumer,
Control Center, Display service, protocol helper, Plasma configuration, service,
or audio route is changed.
