# OpenSanctuary v0.3.2 Session Supervisor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reconcile Battle.net, Agent, Diablo III, and install-update state automatically so the single OpenSanctuary button remains correct across restarts and updates.

**Architecture:** Add a pure snapshot/supervisor module to `sanctuary-battlenet`, expose typed supervisor events through `sanctuary-launcher`, and poll the supervisor from the egui app on a bounded cadence. Update completion feeds the existing probe/index pipeline rather than duplicating install validation.

**Tech Stack:** Rust 2024, std `/proc` inspection, serde/toml bridge persistence, existing mpsc launcher event bus, egui/eframe 0.35.

## Global Constraints

- Battle.net/Wine handling remains isolated to `sanctuary-battlenet` and launcher orchestration.
- Do not capture credentials or automate Blizzard login.
- Do not modify Blizzard installation files.
- Do not add Wine/Proton dependencies to native engine/render/assets crates.
- Rust warnings remain denied by the existing verifier.

---

### Task 1: Process and build snapshot primitives

**Files:**
- Create: `crates/sanctuary-battlenet/src/supervisor.rs`
- Modify: `crates/sanctuary-battlenet/src/lib.rs`
- Test: `crates/sanctuary-battlenet/src/supervisor.rs`

**Interfaces:**
- Produces: `ProcessSnapshot`, `BuildFingerprint`, `SessionSnapshot`, `SupervisorState`, `SessionSupervisor`.

- [ ] Write tests for synthetic `/proc` classification and build fingerprint changes.
- [ ] Attempt the focused test command to establish the RED environment result.
- [ ] Implement filesystem/process snapshot primitives using only `std`.
- [ ] Add update-settle state transitions requiring consecutive stable samples.
- [ ] Export the module API from `lib.rs`.

### Task 2: Launcher supervisor events and state reconciliation

**Files:**
- Modify: `crates/sanctuary-launcher/src/lib.rs`
- Test: `crates/sanctuary-launcher/src/lib.rs`

**Interfaces:**
- Consumes: `SessionSnapshot`, `SupervisorState`.
- Produces: `OfficialBridgeEvent::SessionObserved`, `UpdateSettled`, and model reconciliation behavior.

- [ ] Add model tests for Running, Updating, update-settled, and ready restoration transitions.
- [ ] Add typed bridge events carrying supervisor state.
- [ ] Ensure update settling reuses probe/index behavior instead of directly claiming readiness.

### Task 3: App polling, duplicate-launch prevention, and repair action

**Files:**
- Modify: `apps/launcher/src/main.rs`

**Interfaces:**
- Consumes: `SessionSupervisor`, launcher event bus.
- Produces: periodic session reconciliation and `Repair Bridge` action.

- [ ] Initialize supervisor from persisted discovery/install state.
- [ ] Poll no faster than once per second and emit events only on meaningful snapshot/state changes.
- [ ] Prevent official-game duplicate launch when supervisor already sees Diablo III running.
- [ ] Avoid launching another Battle.net client when the supervisor already sees it running.
- [ ] On `UpdateSettled`, run the existing automatic probe/index pipeline.
- [ ] Add `Repair Bridge` that deletes only the OpenSanctuary bridge record and rediscoveries.

### Task 4: Release metadata and verification

**Files:**
- Modify: `Cargo.toml`
- Modify: `README.md`
- Modify: `VERIFICATION.md`
- Modify: `scripts/verify.sh`

**Interfaces:**
- Produces: v0.3.2 source package with supervisor boundary checks.

- [ ] Set workspace version to `0.3.2`.
- [ ] Document session supervision and bridge repair.
- [ ] Extend static verifier checks for supervisor symbols and compatibility boundary.
- [ ] Run manifest/shell/diff/static package checks.
- [ ] Produce ZIP, TAR.GZ, and v0.3.1 -> v0.3.2 patch.
