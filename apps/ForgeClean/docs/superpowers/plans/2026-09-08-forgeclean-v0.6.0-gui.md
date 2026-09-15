# ForgeClean v0.6.0 GUI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a native DragonGlass ForgeClean frontend while preserving the verified CLI/daemon as the authoritative engine.

**Architecture:** Add a focused `gui_state` library module for settings, filesystem/service snapshots, and action execution; add `forgeclean-gui` as an eframe/egui native binary; keep destructive operations routed through existing ForgeClean commands/core safety gates. Persist GUI/runtime settings under the user config directory and let the daemon read organizer timing/maintenance defaults from that same file.

**Tech Stack:** Rust 2024, ForgeClean core library, serde/serde_json, eframe/egui 0.36.2 with Glow + X11 + Wayland, systemd --user.

**Spec:** `docs/superpowers/specs/2026-09-08-forgeclean-v0.6.0-gui-design.md`

## Global Constraints

- Version is exactly `0.6.0`.
- Preserve all v0.5.2 sorting, cleanup, direct-unlink, offload, ColdPack, GC, registry, and rerouting safety behavior.
- GUI transparency target is 90% transparent / 10% smoky-glassy.
- Closing the GUI never stops the persistent organizer.
- GUI must not duplicate destructive cleanup/GC logic.
- Incomplete downloads never become canonical build artifacts.
- Existing project/version build-aware reconciliation remains always active.

---

### Task 1: Shared GUI/runtime settings and state
**Files:** Create `src/gui_state.rs`; modify `src/lib.rs`; create `tests/gui_state.rs`.
**Interfaces:** Produce `GuiSettings`, `DashboardSnapshot`, `ProjectSnapshot`, `BuildSnapshot`, `load_settings`, `save_settings`, `collect_dashboard_snapshot`, and `run_core_action`.
- [ ] Write settings round-trip and project/build snapshot tests.
- [ ] Verify tests are RED before the module exists (host compile gate if Rust is unavailable locally).
- [ ] Implement JSON settings with fail-safe defaults and atomic write/rename.
- [ ] Implement read-only project/build/service/storage/ColdPack snapshot collection.
- [ ] Implement background-safe core action runner by invoking the installed/current sibling `forgeclean` binary, preserving its CLI safety gates.

### Task 2: Runtime settings integration
**Files:** Modify `src/main.rs`, `forgeclean-organizer.service.in`.
**Interfaces:** `watch` with no timing flags reads persisted `GuiSettings`; explicit CLI flags still override settings.
- [ ] Add tests/contract assertions for default 30s stability / 2s poll when no settings file exists.
- [ ] Wire settings-backed defaults without changing explicit CLI option behavior.
- [ ] Change service ExecStart to `forgeclean watch` so persisted runtime settings take effect after restart.

### Task 3: Native DragonGlass GUI
**Files:** Create `src/gui.rs`, `src/gui_main.rs`; modify `Cargo.toml`.
**Interfaces:** `forgeclean-gui --version`, `forgeclean-gui --self-test`, native app with Dashboard, Projects, Inbox, Cleanup, ColdPack, Storage, Activity, Settings, Diagnostics.
- [ ] Add compile/static contracts for the second binary and eframe dependency.
- [ ] Implement transparent undecorated native viewport and custom DragonGlass titlebar.
- [ ] Implement left nav + scrollable pages and responsive cards.
- [ ] Implement background refresh/actions through channels so render loop never blocks on long commands.
- [ ] Implement settings save/restart-service controls and destructive-action confirmations.

### Task 4: Desktop integration and release gates
**Files:** Create `forgeclean.desktop.in`; modify `install-local.sh`, `build-and-verify.sh`, `hit-it-template.sh`, `README.md`, version bindings; create `tests/regression_v0_6_0_gui.sh`.
**Interfaces:** Install both binaries, desktop entry, persistent service, GUI self-test; keep verification TXT guarantee.
- [ ] Write RED static regression for GUI binary/theme/settings/desktop install/version bindings.
- [ ] Update version bindings to 0.6.0 without rewriting historical regression semantics.
- [ ] Build-and-verify must run Clippy/tests/release build, CLI smoke, GUI `--self-test`, and all inherited E2E gates.
- [ ] Installer must install `forgeclean` and `forgeclean-gui`, desktop entry, restart organizer, and verify service/linger.
- [ ] Seal source ZIP, bind exact SHA256 into HIT-IT, fresh-extract static verification, and preserve early-failure VERIFY.txt.
