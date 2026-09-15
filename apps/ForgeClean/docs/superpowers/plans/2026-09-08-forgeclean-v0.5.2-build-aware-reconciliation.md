# ForgeClean v0.5.2 Build-Aware Reconciliation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Continuously group old and newly downloaded project artifacts into canonical project/version build folders without breaking legacy paths.

**Architecture:** Add filename-based `BuildIdentity`, route it before generic categories, and run a migration/reconciliation pass over legacy live staging roots on every organizer cycle. Preserve previous paths through guarded symlinks and exclude transient download suffixes.

**Tech Stack:** Rust 2024, std filesystem/symlink APIs, existing ForgeClean organizer/registry/ColdPack modules, Bash host gates.

**Spec:** `docs/superpowers/specs/2026-09-08-forgeclean-v0.5.2-build-aware-reconciliation-design.md`

## Global Constraints
- Canonical release: `0.5.2`.
- Project/build identity precedes generic file type.
- Canonical build path: `Projects/<Project>/Builds/<Version>/`.
- Existing and future live artifacts are reconciled continuously.
- Old paths remain guarded symlinks; real paths are never overwritten.
- `.part`, `.partial`, `.tmp` are never promoted into build bundles.
- Existing ColdPack/GC/offload/package-cleanup behavior is unchanged.

---

### Task 1: Build identity and destination
**Files:** Modify `src/organizer.rs`; test `tests/organizer.rs`.
- [x] Add `BuildIdentity { project, version }`.
- [x] Parse `-vX.Y.Z-`, `-X.Y.Z-`, and `-VX_Y_Z-` naming forms.
- [x] Reject incomplete download suffixes.
- [x] Add `ForgeLayout::project_builds` and `project_build`.
- [x] Route build identity before generic categories.

### Task 2: Build-bundle routing and compatibility
**Files:** Modify `src/organizer.rs`; test `tests/organizer.rs`.
- [x] Move recognized artifacts to the canonical build directory.
- [x] Create guarded legacy aliases after the move.
- [x] Roll back the move if alias creation fails.
- [x] Keep build artifacts live instead of immediately ColdPacking them.

### Task 3: Old + future reconciliation
**Files:** Modify `src/organizer.rs`; test `tests/organizer.rs`.
- [x] Scan legacy generic live buckets.
- [x] Scan each project `Downloads/` and `Releases/` staging root.
- [x] Skip symlinks and hidden entries.
- [x] Reconcile stable old artifacts before top-level processing.
- [x] Repeat on every organizer/watch cycle so later arrivals are captured.

### Task 4: Host release gates
**Files:** Modify `build-and-verify.sh`, `hit-it-template.sh`, `tests/regression_v0_5_2_build_aware.sh`, `README.md`.
- [x] Add static build-aware regression.
- [x] Add focused organizer unit gates.
- [x] Add existing-file and later-arriving-file E2E gates.
- [x] Keep v0.5.2 Clippy host-gate fix.
- [x] Keep verification-TXT failure-path guarantee.
- [ ] Run host Cargo fmt/Clippy/test/release and persistent install verification.
