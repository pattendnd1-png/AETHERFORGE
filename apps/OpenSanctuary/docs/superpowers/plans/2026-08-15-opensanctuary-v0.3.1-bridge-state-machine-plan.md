# OpenSanctuary v0.3.1 Bridge State Machine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist and revalidate the successful Battle.net/Diablo III bridge and expose a reliable explicit lifecycle through OpenSanctuary.

**Architecture:** `sanctuary-battlenet` owns versioned bridge persistence, validation, discovery merge, and retry policy. `sanctuary-launcher` owns lifecycle events/model state. `apps/launcher` loads/revalidates the bridge on startup and renders/initiates actions without handling credentials or Windows process internals.

**Tech Stack:** Rust 2024, serde/TOML, std::fs/std::process/std::thread/std::sync::mpsc, egui/eframe 0.35.

## Global Constraints

- No credential or 2FA storage/automation.
- No bundled Blizzard executables or game data.
- Wine/Windows process code remains isolated to `sanctuary-battlenet`.
- Persist only bridge filesystem metadata.
- Revalidate saved paths before use.
- Preserve v0.3.0 native engine, probe, and index behavior.
- Keep Clippy `-D warnings` clean on Rust 1.97.

---

### Task 1: Persisted bridge record and validation

**Files:**
- Modify: `crates/sanctuary-battlenet/Cargo.toml`
- Modify: `crates/sanctuary-battlenet/src/lib.rs`
- Test: `crates/sanctuary-battlenet/tests/discovery.rs`

**Interfaces:**
- Produces `BridgeRecord`, `BridgeRecord::from_discovery`, `BridgeRecord::validated`, `load_bridge_record`, `save_bridge_record`, `default_bridge_record_path`, `merge_saved_discovery`, and `RetryPolicy`.

- [ ] Add tests for record round-trip, missing record, stale validation, XDG/default path, and retry attempts.
- [ ] Attempt `cargo test -p sanctuary-battlenet` to establish RED; record toolchain absence if unavailable.
- [ ] Add serde/toml dependencies and implement versioned atomic persistence plus path validation.
- [ ] Implement merge so valid saved fields are preferred and missing/stale fields fall back to normal discovery.
- [ ] Implement bounded retry policy helper with exactly 3 default attempts.
- [ ] Attempt focused tests again; run static manifest/source checks if Cargo is unavailable.
- [ ] Commit bridge persistence.

### Task 2: Explicit launcher bridge lifecycle

**Files:**
- Modify: `crates/sanctuary-launcher/src/lib.rs`

**Interfaces:**
- Produces `BridgeState` and stores it in `LauncherModel` as `bridge_state` plus `bridge_detail`.
- Extends official bridge events with `BridgeValidated`, `BridgeInvalidated`, and `LaunchRetry` where needed.

- [ ] Add model tests for MissingClient → Installing → Verifying → Indexing → Ready and Ready → Starting → Running → Ready/Error.
- [ ] Attempt focused launcher tests for RED.
- [ ] Implement typed lifecycle transitions in `LauncherModel::apply_event`.
- [ ] Make probe/index completion synchronize bridge state without changing native install-state semantics.
- [ ] Add bounded Battle.net/Diablo spawn retries to the official-game worker using `RetryPolicy`.
- [ ] Attempt tests again and commit lifecycle changes.

### Task 3: Startup revalidation and UI behavior

**Files:**
- Modify: `apps/launcher/Cargo.toml`
- Modify: `apps/launcher/src/main.rs`

**Interfaces:**
- Loads and validates the persisted bridge before normal discovery.
- Saves refreshed bridge records after detection/discovery.
- Renders bridge state rows and disables PLAY while Starting/Running.

- [ ] Wire startup record load/revalidation and saved-install probing.
- [ ] Persist successful bridge discovery/install detection.
- [ ] Replace loose official status text with `BridgeState`-based Game/Battle.net/Install/Content/Runtime rows.
- [ ] Render STARTING/RUNNING/VERIFYING/INDEXING labels and action disable rules.
- [ ] On official-game exit, revalidate and return to Ready or BrokenInstall.
- [ ] Run static egui/exhaustive-match/delimiter checks and commit UI integration.

### Task 4: Release v0.3.1

**Files:**
- Modify: `Cargo.toml`
- Modify: `README.md`
- Modify: `VERIFICATION.md`
- Modify: `packaging/arch/PKGBUILD`
- Modify: `packaging/arch/make-source.sh`

**Interfaces:**
- Produces `OpenSanctuary-0.3.1-source.zip`, `.tar.gz`, and v0.3.0→v0.3.1 patch.

- [ ] Bump all workspace/package release metadata to 0.3.1.
- [ ] Document persisted bridge record and reset/override behavior.
- [ ] Run manifests/workspace/shell/version/compatibility-boundary/diff/static source gates.
- [ ] Generate ZIP/TAR and upgrade patch from the exact v0.3.0 baseline.
- [ ] Validate archive integrity and required contents.
- [ ] Commit release metadata.
