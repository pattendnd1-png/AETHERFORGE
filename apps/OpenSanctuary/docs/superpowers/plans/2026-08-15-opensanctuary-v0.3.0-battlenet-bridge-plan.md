# OpenSanctuary v0.3.0 Battle.net Bootstrap Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Battle.net bootstrap/install bridge and managed Diablo III launch path while preserving OpenSanctuary's native engine and v0.2.5 content workflow.

**Architecture:** A new `sanctuary-battlenet` crate owns discovery and external-process commands. `sanctuary-launcher` wraps it in asynchronous events. The egui app only initiates workflows, renders status, and keeps native PLAY preferred.

**Tech Stack:** Rust 2024, std::process/std::fs/std::sync::mpsc, thiserror, egui/eframe 0.35.

## Global Constraints

- Do not bundle or download Blizzard binaries/game data.
- Do not capture or automate Battle.net credentials or 2FA.
- Do not recursively scan the user's entire home directory.
- Keep compatibility-runner code isolated to `sanctuary-battlenet`; the native engine remains runner-independent.
- Maintain Rust 1.92 workspace floor and Clippy `-D warnings` compatibility with the user's Rust 1.97 toolchain.
- Preserve existing v0.2.5 content/index/native-engine behavior.

---

### Task 1: External Battle.net bridge crate

**Files:**
- Create: `crates/sanctuary-battlenet/Cargo.toml`
- Create: `crates/sanctuary-battlenet/src/lib.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Produces `BridgeDiscovery`, `BridgeError`, `OFFICIAL_DOWNLOAD_URL`, `discover`, `launch_battlenet`, `open_official_download`, `launch_diablo`, and `find_diablo_candidates`.

- [ ] Add failing fixture tests for prefix/runner/launcher/game discovery and command preconditions.
- [ ] Implement bounded common-prefix discovery and PATH runner lookup.
- [ ] Implement Battle.net spawn, official-download handoff, and Diablo spawn using one prefix/runner.
- [ ] Run `cargo test -p sanctuary-battlenet` on an environment with Rust; if unavailable, preserve tests and run static source checks.
- [ ] Commit the crate.

### Task 2: Launcher orchestration events

**Files:**
- Modify: `crates/sanctuary-launcher/Cargo.toml`
- Modify: `crates/sanctuary-launcher/src/lib.rs`

**Interfaces:**
- Adds `PrimaryAction::Install`.
- Adds `OfficialBridgeEvent::{ClientLaunched, DownloadPageOpened, InstallDetected, Failed, WatchFinished}` inside `LauncherEvent`.
- Adds `spawn_official_install_bridge(home, sender)` and `spawn_official_game(discovery, install_path, sender)`.

- [ ] Add tests asserting an unconfigured model selects INSTALL and bridge events update model state/message.
- [ ] Implement event variants and model application.
- [ ] Implement install watcher with bounded candidate polling and Task events.
- [ ] Implement official game worker that starts Battle.net, waits briefly, starts Diablo III, and reports task completion/failure.
- [ ] Run `cargo test -p sanctuary-launcher` when Rust is available; otherwise run structural checks.
- [ ] Commit orchestration.

### Task 3: Launcher UI/install flow

**Files:**
- Modify: `apps/launcher/Cargo.toml`
- Modify: `apps/launcher/src/main.rs`

**Interfaces:**
- Consumes the new bridge events and discovery state.
- Adds local `official_install_active`, `official_game_child/status` state only if required by process polling.

- [ ] Update primary-action rendering to support INSTALL DIABLO III.
- [ ] Make INSTALL launch/watch Battle.net or open the official download page.
- [ ] On `InstallDetected`, save the path and automatically run the existing probe.
- [ ] Add Battle.net status lines and a manual `Open Battle.net` secondary action without credential UI.
- [ ] Keep native PLAY first; add official fallback only when native launch is unavailable and bridge discovery has an executable game client.
- [ ] Run formatting/Clippy/build on Rust-capable environment; run static egui/API/delimiter checks here.
- [ ] Commit UI integration.

### Task 4: Release/package v0.3.0

**Files:**
- Modify: `Cargo.toml`
- Modify: `README.md`
- Modify: `BUILD-ON-ARCH.sh`
- Modify: `packaging/arch/PKGBUILD`
- Modify: `packaging/arch/make-source.sh`
- Modify: `scripts/verify.sh`
- Modify: `VERIFICATION.md`

**Interfaces:**
- Produces `OpenSanctuary-0.3.0-source.zip`, `.tar.gz`, and a v0.2.5→v0.3.0 patch.

- [ ] Bump workspace/package metadata to 0.3.0.
- [ ] Update verifier for 13 workspace crates plus launcher/game apps and allow compatibility-runner references only inside the bridge crate/docs.
- [ ] Update README with Battle.net bootstrap limitations and environment overrides.
- [ ] Run manifest, shell, workspace, compatibility-boundary, version, archive-integrity, and diff checks.
- [ ] Generate clean release archives and upgrade patch.
- [ ] Commit the release metadata.
