# Stream Deck 7.5.1 Package Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Import user-owned Elgato Stream Deck 7.5.1 package metadata into AetherBrowser's native embedded Stream Deck Studio without executing or redistributing macOS binaries.

**Architecture:** Add a focused package-import/normalization module to `aether-deck`, persist normalized metadata in the existing Stream Deck data root, merge imported catalog data into Studio UI/profile creation, and keep the native daemon/HID action execution path authoritative.

**Tech Stack:** Rust/serde_json, Linux filesystem/process APIs, existing `aether-deck`/egui/ProfileStore, shell release regressions.

**Spec:** `docs/superpowers/specs/2026-09-13-streamdeck-751-package-integration-design.md`

## Global Constraints
- Canonical version: `2.1.48`.
- Exact preferred package path: `/home/benji/Downloads/Stream_Deck_7.5.1.22901.pkg`.
- Reference SHA-256: `8cc1f0b875839e2d50618a37cad2f46b689cad1d3fe1df1af1e373303515ffe8`.
- Never execute Mach-O/dylib vendor code on Linux.
- Never redistribute the `.pkg`, app bundle, or proprietary asset payload inside AetherBrowser artifacts.
- Preserve existing embedded-window/reintegration behavior and existing no-recursion capture safety.

---

### Task 1: Importer model and package discovery
**Files:**
- Create: `crates/aether-deck/src/elgato_package.rs`
- Modify: `crates/aether-deck/src/lib.rs`
- Test: `crates/aether-deck/tests/elgato_package.rs`

**Interfaces:**
- Produces: `ElgatoPackageImporter`, `ElgatoImportSummary`, `ElgatoPluginRecord`, `ElgatoProfileRecord`, `default_elgato_pkg_path()`.

- [ ] Write failing discovery/hash/model tests.
- [ ] Run focused test to confirm RED.
- [ ] Implement discovery + safe normalized model.
- [ ] Run focused test to GREEN.

### Task 2: Archive extraction and resource normalization
**Files:**
- Modify: `crates/aether-deck/src/elgato_package.rs`
- Test: `crates/aether-deck/tests/elgato_package.rs`

**Interfaces:**
- Produces: `inspect_package(path) -> Result<ElgatoImportSummary, String>` and `import_package(path) -> Result<ElgatoImportSummary, String>`.

- [ ] Add failing tests for safe archive member validation and fixture normalization.
- [ ] Run RED.
- [ ] Implement bounded XAR/payload extraction strategy and JSON normalization; reject traversal/executable runtime use.
- [ ] Run GREEN.

### Task 3: Stream Deck+ editable profile import
**Files:**
- Modify: `crates/aether-deck/src/studio_model.rs`
- Modify: `crates/aether-deck/src/elgato_package.rs`
- Test: `crates/aether-deck/tests/elgato_package.rs`

**Interfaces:**
- Produces: `create_editable_stream_deck_plus_profile(document, imported_profile)`.

- [ ] Add failing profile conversion test for model `20GBD9901`, schema `2.0`, two pages, Keypad + Encoder.
- [ ] Run RED.
- [ ] Implement native editable profile conversion without vendor execution.
- [ ] Run GREEN.

### Task 4: Studio UI integration
**Files:**
- Modify: `crates/aether-deck/src/bin/aether-streamdeck-studio.rs`
- Test: `tests/current-v2-1-48-elgato-package-integration.sh`

**Interfaces:**
- Consumes normalized import summary/catalog/profile.
- Produces package status, Import/Refresh, editable-copy action, catalog visibility in embedded Studio.

- [ ] Add failing structural UI/release contract test.
- [ ] Run RED.
- [ ] Implement UI and startup refresh that never blocks Studio startup.
- [ ] Run GREEN.

### Task 5: Release gates and canonical 2.1.48 package
**Files:**
- Modify: `Cargo.toml`, crate versions via workspace identity, README/CHANGELOG.
- Modify: `scripts/verify.sh`, `scripts/package-release.sh`, `tests/current-clean-break.sh` as needed.
- Test: all existing release gates plus `current-v2-1-48-elgato-package-integration.sh`.

- [ ] Wire new gate into verify/package/clean-break.
- [ ] Promote all active release identity to 2.1.48.
- [ ] Run full local static/package suite.
- [ ] Build canonical `.run`, source ZIP, SHA256SUMS, static verify, package verify.
- [ ] Verify consolidated installer payload and checksums.
