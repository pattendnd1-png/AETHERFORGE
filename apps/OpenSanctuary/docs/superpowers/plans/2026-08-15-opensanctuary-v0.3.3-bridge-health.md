# OpenSanctuary v0.3.3 Bridge Health & Self-Healing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add typed bridge health, safe self-healing rediscovery, crash-loop protection, and launcher health/recovery UI on top of corrected v0.3.2 source.

**Architecture:** `sanctuary-battlenet` owns all validation/recovery/cooldown logic and exposes pure testable types/functions. `sanctuary-launcher` transports typed events and state. `apps/launcher` performs bounded background recovery and renders the health surface.

**Tech Stack:** Rust 2024, serde/toml, eframe/egui 0.35, std `/proc` and filesystem APIs.

## Global Constraints

- Never store Battle.net credentials or automate login forms.
- Never delete Battle.net, the Wine prefix, or Diablo III as part of repair.
- Keep Wine-specific integration inside `crates/sanctuary-battlenet`.
- Preserve existing install/probe/index/native-engine behavior.
- Keep Clippy clean under Rust 1.97 with `-D warnings`.

---

### Task 1: Bridge health and recovery primitives

**Files:**
- Create: `crates/sanctuary-battlenet/src/health.rs`
- Modify: `crates/sanctuary-battlenet/src/lib.rs`
- Test: `crates/sanctuary-battlenet/tests/health.rs`

**Interfaces:**
- Produces `BridgeHealthState`, `BridgeHealthReport`, `RecoveryOutcome`, `evaluate_bridge_health`, `recover_bridge`, `quarantine_bridge_record`, `FailureTracker`.

- [ ] Add tests for healthy/degraded/broken component reports.
- [ ] Add tests showing a stale saved Diablo path recovers from the saved prefix.
- [ ] Add a test proving corrupt-record quarantine renames only the record.
- [ ] Add tests for three failures entering cooldown and success clearing the tracker.
- [ ] Implement the minimum health/recovery/cooldown code to satisfy those tests.
- [ ] Export the public health surface from `sanctuary-battlenet`.

### Task 2: Launcher health events and state

**Files:**
- Modify: `crates/sanctuary-launcher/src/lib.rs`

**Interfaces:**
- Consumes `BridgeHealthReport`.
- Produces `LauncherEvent::BridgeHealth`, `OfficialBridgeEvent::RecoveryStarted`, `RecoveryCompleted`, `RecoveryFailed`, and model fields `bridge_health`, `last_bridge_recovery`.

- [ ] Add launcher tests showing recovery start/result transitions do not overwrite Running/Updating.
- [ ] Add model fields with non-broken defaults.
- [ ] Apply health/recovery events to BridgeState, detail, and Activity without altering install/index data directly.

### Task 3: Background recovery orchestration and UI

**Files:**
- Modify: `apps/launcher/src/main.rs`

**Interfaces:**
- Uses `recover_bridge`, bridge record load/save/quarantine helpers, and launcher health events.

- [ ] Evaluate bridge health during startup.
- [ ] Trigger at most one automatic recovery for degraded saved state.
- [ ] Persist only complete recovered records and route recovered Diablo installs into the existing probe flow.
- [ ] Replace Repair Bridge with `REPAIR & REDISCOVER` and make it invoke safe recovery.
- [ ] Add the Bridge Health block to the Diablo details/status surface.
- [ ] Apply launch cooldown before automatic Battle.net/Diablo retry loops.

### Task 4: Release hardening and packaging

**Files:**
- Modify: `Cargo.toml`, `README.md`, `VERIFICATION.md`, `BUILD-ON-ARCH.sh`, `scripts/verify.sh`, `packaging/arch/make-source.sh`

**Interfaces:**
- Produces v0.3.3 source ZIP/TAR.GZ and corrected v0.3.2 source -> v0.3.3 patch.

- [ ] Set workspace/package references to `0.3.3`.
- [ ] Document health/self-healing behavior and safe repair semantics.
- [ ] Extend static verifier to require health/recovery symbols and preserve compatibility-layer isolation.
- [ ] Run all available static checks in the sandbox.
- [ ] Package exactly the committed source tree and validate both archives.
