# OpenSanctuary v0.3.11 Network Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an independent Battle.net network-health bridge with in-app repair, diagnostics, and safe retry behavior.

**Architecture:** `sanctuary-battlenet` owns DNS/TCP/TLS probing and a typed `NetworkBridgeReport`. `sanctuary-launcher` carries that report as launcher state/events, while `apps/launcher` runs probes asynchronously and exposes status/repair from the Battle.net page. Network repair never changes system DNS, installs proxies, bypasses TLS, or modifies Blizzard security settings.

**Tech Stack:** Rust 2024, std networking, rustls 0.23, webpki-roots 1, egui/eframe.

**Spec:** Approved in chat: system connectivity → DNS → outbound TCP/443 → Blizzard TLS reachability → ONLINE/DEGRADED/OFFLINE, with an in-app `REPAIR NETWORK BRIDGE` action.

## Global Constraints

- Full semantic version only: `0.3.11`; no revision or RC suffixes.
- Blizzard authentication remains Blizzard-controlled; no password capture or credential injection.
- Network bridge is diagnostic/recovery orchestration only; no proxying, traffic interception, DNS rewriting, TLS weakening, or security bypass.
- Network work must not block the egui render thread.

---

### Task 1: Network probe core

**Files:**
- Create: `crates/sanctuary-battlenet/src/network.rs`
- Modify: `crates/sanctuary-battlenet/src/lib.rs`
- Modify: `crates/sanctuary-battlenet/Cargo.toml`
- Test: `crates/sanctuary-battlenet/tests/network.rs`

**Interfaces:**
- Produces: `NetworkBridgeState`, `NetworkCheck`, `NetworkBridgeReport`, `probe_network_bridge()`.

- [ ] Add tests for DNS failure classification, aggregate state classification, and endpoint configuration.
- [ ] Implement bounded DNS resolution, TCP/443 connect and TLS handshake checks against official Battle.net domains.
- [ ] Export the typed network API from `sanctuary-battlenet`.

### Task 2: Launcher state/event integration

**Files:**
- Modify: `crates/sanctuary-launcher/src/lib.rs`

**Interfaces:**
- Consumes: `NetworkBridgeReport`.
- Produces: `LauncherEvent::NetworkBridgeChecked(NetworkBridgeReport)` and `LauncherModel::network_bridge`.

- [ ] Add a model test proving a failed network check does not overwrite RUNNING/UPDATING bridge lifecycle state.
- [ ] Add event/model state handling and activity messaging.

### Task 3: Asynchronous app repair flow

**Files:**
- Modify: `apps/launcher/src/main.rs`

**Interfaces:**
- Consumes: `probe_network_bridge()` and launcher network event.
- Produces: startup/background probes and `repair_network_bridge()`.

- [ ] Add app-side timestamp/in-flight state.
- [ ] Run probes on worker threads and send reports through `TaskBus`.
- [ ] On repair: clear only OpenSanctuary transient network state, reprobe, then refresh software/session observation on success.

### Task 4: Battle.net UI and diagnostics

**Files:**
- Modify: `apps/launcher/src/battlenet_page.rs`
- Modify: `apps/launcher/src/diagnostics.rs`

**Interfaces:**
- Consumes: network report summary/state/checks.
- Produces: `BattleNetPageAction::RepairNetworkBridge` and player-facing status.

- [ ] Add NETWORK compact status and repair button for degraded/offline states.
- [ ] Add advanced DNS/TCP/TLS check rows to diagnostics.

### Task 5: Version, verifier, package, activation string

**Files:**
- Modify: workspace version metadata, README/VERIFICATION/build scripts/verifier.
- Create canonical v0.3.11 ZIP/TAR and v0.3.10→v0.3.11 patch.

- [ ] Set canonical version to 0.3.11 everywhere.
- [ ] Extend verifier for network module/action/version/revision policy.
- [ ] Validate package integrity and produce one install/activation command using the canonical ZIP.
