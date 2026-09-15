# Stream Deck Windows-Parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship AetherBrowser v2.1.37 with a standalone native Rust Stream Deck editor that matches the Windows Stream Deck application's core layout and workflows, while focusing runtime verification on Stream Deck/Elgato and Velora.tv.

**Architecture:** Extend `aether-deck` with a serializable editor/profile/action model and a standalone `aether-streamdeck-studio` eframe binary. Keep `aether-deck-daemon` as the single hardware owner and retain `aether-deck-monitor` as a separate diagnostic surface. Add focused v2.1.37 verification and packaging without touching known-good Twitch/YouTube/YouTube Music runtime paths.

**Tech Stack:** Rust 2024, eframe/egui 0.34.3, hidapi 2.6.7, serde/serde_json, image JPEG encoding, Bash verification/packaging.

**Spec:** `docs/superpowers/specs/2026-09-11-streamdeck-windows-parity-design.md`

## Global Constraints

- Canonical release version is exactly `2.1.37`; no RC/revision suffixes.
- Default Stream Deck Studio visual layout follows Windows Stream Deck software.
- Primary acceptance hardware is VID `0x0fd9`, PID `0x0084`, 8 keys, 4 dials, 4 touch sections.
- Direct HID remains single-writer ownership through Aether Deck.
- Twitch, YouTube, and YouTube Music runtime tests are excluded from this patch cycle.
- Velora.tv and Stream Deck/Elgato are the focused runtime targets.
- Firmware writes require a verified official package and explicit confirmation.

---

### Task 1: Version and focused release gates

**Files:**
- Modify: `Cargo.toml`
- Modify: `scripts/verify.sh`
- Modify: `scripts/package-release.sh`
- Modify: `scripts/install-host.sh`
- Modify: `scripts/host-build.sh`
- Test: `tests/current-release-identity.sh`
- Create: `tests/current-v2-1-37-focused-scope.sh`

**Interfaces:**
- Consumes: existing v2.1.36 verification framework.
- Produces: v2.1.37 identity and a focused runtime/test contract that omits Twitch/YouTube/YouTube Music probes.

- [ ] Add a failing contract asserting version 2.1.37 and explicit focused-scope markers.
- [ ] Run the shell contract and confirm RED on the v2.1.36 tree.
- [ ] Change version/tag/path literals to 2.1.37 and add focused-scope markers.
- [ ] Run the shell contract and confirm PASS.

### Task 2: Persistent Windows-parity editor model

**Files:**
- Create: `crates/aether-deck/src/studio_model.rs`
- Modify: `crates/aether-deck/src/lib.rs`
- Modify: `crates/aether-deck/Cargo.toml`
- Test: `crates/aether-deck/tests/studio_model.rs`

**Interfaces:**
- Produces: `StudioDocument`, `StudioProfile`, `StudioPage`, `ActionDefinition`, `ActionInstance`, `SlotAddress`, `ControllerKind`, `ProfileStore`, and canonical built-in action catalog.

- [ ] Write tests for default Stream Deck+ 4×2+4 encoder document, profile/page CRUD, action assignment compatibility, JSON round-trip, atomic store path, and app-profile binding.
- [ ] Confirm the tests fail because the model does not exist.
- [ ] Implement the serializable model and profile store.
- [ ] Re-run model tests and keep all existing `aether-deck` tests green.

### Task 3: Standalone Windows-parity editor UI

**Files:**
- Create: `crates/aether-deck/src/bin/aether-streamdeck-studio.rs`
- Create: `crates/aether-deck/tests/studio_ui_contract.rs`
- Create: `packaging/wrappers/aether-streamdeck-studio`

**Interfaces:**
- Consumes: Task 2 editor model.
- Produces: standalone native editor binary with device/profile header, key/encoder canvas, action library, property inspector, pages/folders, and status bar.

- [ ] Add source-contract tests for Windows-parity regions, action search, 4×2 key geometry, four encoder regions, profile controls, property inspector, page navigation, drag/drop assignment hooks, and neutral dark default theme.
- [ ] Confirm RED before the binary exists.
- [ ] Implement the eframe editor with direct model updates and persistence.
- [ ] Confirm UI contract PASS.

### Task 4: Hardware synchronization and interaction

**Files:**
- Create: `crates/aether-deck/src/studio_hardware.rs`
- Modify: `crates/aether-deck/src/lib.rs`
- Modify: `crates/aether-deck/src/bin/aether-deck-daemon.rs`
- Test: `crates/aether-deck/tests/studio_hardware.rs`

**Interfaces:**
- Consumes: `StudioDocument`, active page assignments, existing Stream Deck+ HID JPEG/LCD/input functions.
- Produces: render plan for eight key JPEGs and four 200×100 encoder regions; input-to-slot dispatch; reconnect restore plan.

- [ ] Write failing tests for eight-key render plan, four encoder regions, input dispatch, reconnect restore, and controller compatibility.
- [ ] Implement pure hardware synchronization planning and daemon commands.
- [ ] Keep actual HID writes behind existing direct-HID functions.
- [ ] Run tests and existing Stream Deck contracts.

### Task 5: Plugin/action compatibility model

**Files:**
- Create: `crates/aether-deck/src/plugin_manifest.rs`
- Test: `crates/aether-deck/tests/plugin_manifest.rs`

**Interfaces:**
- Produces: safe parser for public manifest concepts: UUID/name/version, Keypad/Encoder controllers, states, property inspector path, profiles, and settings metadata.

- [ ] Add failing fixtures for Keypad, Encoder, mixed controller, property inspector, and profile bundle metadata.
- [ ] Implement serde parser that rejects invalid controller/action identifiers and never executes plugin code.
- [ ] Verify parser tests PASS.

### Task 5A: Twitch account and action integration

**Files:**
- Create: `crates/aether-deck/src/twitch_oauth.rs`
- Modify: `crates/aether-deck/src/lib.rs`
- Modify: `crates/aether-deck/src/studio_model.rs`
- Modify: `crates/aether-deck/src/bin/aether-streamdeck-studio.rs`
- Create: `tests/current-v2-1-37-twitch-account.sh`

**Interfaces:**
- Produces: public Device Code OAuth controller, Secret Service token storage/validation/refresh, Preferences → Accounts → Twitch UI, and built-in Twitch actions.

- [ ] Add a failing contract proving Twitch OAuth is available in the full Stream Deck Studio rather than only the diagnostic monitor.
- [ ] Implement public-client Device Code OAuth, token validation/refresh, disconnect, and activation-page handling.
- [ ] Add Twitch actions to the editor catalog without re-running browser Twitch playback tests.
- [ ] Verify the focused Twitch-account source contract passes.

### Task 6: Focused Stream Deck+ and Velora host acceptance

**Files:**
- Modify: `scripts/streamdeck-plus-runtime-test.sh`
- Create: `scripts/velora-runtime-test.sh`
- Modify: `scripts/host-gate.sh` or equivalent host gate script referenced by packaging.
- Create: `tests/current-v2-1-37-streamdeck-windows-parity.sh`
- Create: `tests/current-v2-1-37-velora-focus.sh`

**Interfaces:**
- Produces: `Aether-Browser-v2.1.37-STREAMDECKPLUS-VERIFY.txt` and `Aether-Browser-v2.1.37-VELORA-VERIFY.txt`.

- [ ] Make the focused contracts fail on v2.1.36 due to missing studio binary/model markers.
- [ ] Update Stream Deck+ acceptance to launch the full editor first and keep diagnostic monitor as optional confirmation.
- [ ] Add Velora-only provider/session smoke without Twitch/YouTube/YouTube Music runtime calls.
- [ ] Verify focused shell contracts PASS statically.

### Task 7: Install/package integration

**Files:**
- Modify: `scripts/install-current-tree.sh`
- Modify: `scripts/package-release.sh`
- Modify: `packaging/desktop/org.aetherforge.AetherBrowser.desktop.in`
- Create: `packaging/desktop/org.aetherforge.StreamDeckStudio.desktop`
- Modify: `README.md`
- Modify: `CHANGELOG.md`

**Interfaces:**
- Produces: installed `/usr/bin/aether-streamdeck-studio`, desktop launcher, source ZIP, installer, SHA256SUMS, static verify, package verify.

- [ ] Add packaging contract requiring new binary wrapper, desktop launcher, model/UI/hardware/plugin modules, and focused tests.
- [ ] Add install mapping for studio binary and launcher.
- [ ] Update documentation and v2.1.37 release notes.
- [ ] Run static/package verification and create canonical artifacts.
