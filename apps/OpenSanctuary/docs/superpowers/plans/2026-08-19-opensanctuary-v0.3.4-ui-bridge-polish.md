# OpenSanctuary v0.3.4 UI + Bridge Integration Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor stateless launcher presentation into focused modules and polish the Diablo III/bridge lifecycle UI without changing v0.3.3 bridge behavior.

**Architecture:** `LauncherApp` and all bridge/session orchestration remain in `main.rs`. `theme.rs` owns palette/style and `widgets.rs` owns reusable stateless egui rendering helpers. Existing `ui_motion.rs` remains the motion/math unit.

**Tech Stack:** Rust 1.97, eframe/egui 0.35, existing OpenSanctuary workspace.

**Spec:** `docs/superpowers/specs/2026-08-19-opensanctuary-v0.3.4-ui-bridge-polish-design.md`

## Global Constraints
- Preserve v0.3.3 bridge health, self-healing, session supervision, install/update/index/play behavior.
- No CASC/native-engine behavior changes.
- No Blizzard credentials/assets/executables in source packages.
- Wine/compatibility-specific production code stays under `crates/sanctuary-battlenet`.
- Strict Clippy `-D warnings` remains a release gate.

---

### Task 1: Extract theme configuration

**Files:**
- Create: `apps/launcher/src/theme.rs`
- Modify: `apps/launcher/src/main.rs`

**Interfaces:**
- Produces: `pub(super) fn configure_style(ctx: &egui::Context)` and shared palette constants.

- [ ] Move `configure_style` and palette constants to `theme.rs`.
- [ ] Import and call `theme::configure_style` from `LauncherApp::new`.
- [ ] Run source/static checks and `cargo fmt --all` where available.
- [ ] Commit `refactor: extract launcher theme`.

### Task 2: Extract reusable stateless widgets

**Files:**
- Create: `apps/launcher/src/widgets.rs`
- Modify: `apps/launcher/src/main.rs`

**Interfaces:**
- Produces stateless `pub(super)` widget/render helpers with the same inputs/behavior as the existing functions.
- Consumes existing egui types plus simple launcher enums/data passed by value/reference.

- [ ] Move painter-only/navigation/card/status helpers that do not mutate `LauncherApp` into `widgets.rs`.
- [ ] Keep helpers that depend on private page/model state in `main.rs` unless moving their types is required.
- [ ] Import helpers with explicit names; do not use wildcard imports.
- [ ] Run static duplicate-symbol and delimiter checks.
- [ ] Commit `refactor: extract launcher widgets`.

### Task 3: Add cohesive bridge-health presentation

**Files:**
- Modify: `apps/launcher/src/main.rs`
- Modify: `apps/launcher/src/widgets.rs`

**Interfaces:**
- Consumes existing `LauncherModel`, `BridgeDiscovery`, supervisor observation, and health fields already present in v0.3.3.
- Produces display-only Bridge Health stack and lifecycle banner; no new bridge side effects.

- [ ] Add a reusable status-row widget with label, state text, and semantic status class.
- [ ] Render Bridge Health, Battle.net, Agent, Runner, Game, and Content rows in the Diablo control surface.
- [ ] Show last recovery text only when present.
- [ ] Make lifecycle headline/button copy consistent for Installing/Updating/Verifying/Indexing/Ready/Starting/Running/Error.
- [ ] Commit `feat: polish bridge health presentation`.

### Task 4: Integrate Downloads & Activity presentation

**Files:**
- Modify: `apps/launcher/src/main.rs`
- Modify: `apps/launcher/src/widgets.rs`

**Interfaces:**
- Consumes existing activity items and bridge/session state only.
- Produces denser active-task rows and compact lifecycle summary.

- [ ] Highlight active bridge/install/index/update tasks in the bottom tray.
- [ ] Preserve completed/failed history and reduced-motion behavior.
- [ ] Do not create new workers or polling loops.
- [ ] Commit `feat: unify launcher activity presentation`.

### Task 5: Release hardening and packaging

**Files:**
- Modify: workspace/package version metadata to `0.3.4`.
- Modify: `README.md`, `VERIFICATION.md`, and `scripts/verify.sh` only as required for the new modules/release guarantees.

**Interfaces:**
- Produces source ZIP, TAR.GZ, and v0.3.3→v0.3.4 patch.

- [ ] Update version metadata consistently to `0.3.4`.
- [ ] Extend verifier to require `theme.rs` and `widgets.rs` and keep compatibility-boundary checks.
- [ ] Scan for the prior strict-Clippy warning patterns (`collapsible_if`, unnecessary lazy `Option` fallback, immediate field reassignment after `Default`).
- [ ] Run all available static checks and package-integrity checks.
- [ ] Generate source archives directly from committed Git tree.
- [ ] Commit `release: prepare OpenSanctuary v0.3.4`.
