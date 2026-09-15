# Authoritative Vendor Package Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the five uploaded vendor installers authoritative local compatibility inputs for AetherBrowser v2.1.48 so embedded creator integrations stop drifting from vendor versions/layouts/capabilities.

**Architecture:** Extend the existing v2.1.48 Elgato importer with a typed cross-provider package registry in `aether-creator-integrations`, a data-only package identity normalizer, and browser-native status/baseline surfaces. Keep all execution on existing Linux-native Rust/browser/OBS-WebSocket/HID paths; vendor installers are never executed or redistributed.

**Tech Stack:** Rust std APIs, existing `aether-creator-integrations` and `aether-native-pages`, Python 3 data-only inspection helper, shell release regressions.

**Spec:** `docs/superpowers/specs/2026-09-13-authoritative-vendor-package-integration-design.md`

## Global Constraints
- Canonical release remains `2.1.48`.
- Never execute vendor `.pkg`, `.exe`, `.msi`, Mach-O, PE, DLL or plugin runtime code.
- Never include vendor installers in release/source artifacts.
- Preserve no-recursion/no-overflow capture safety and embedded-app containment.
- Missing/opaque metadata is reported honestly rather than inferred.

### Task 1: RED cross-provider package registry contract
**Files:**
- Create: `tests/current-v2-1-48-authoritative-vendor-packages.sh`
- Test: the new shell test

- [ ] Assert all five canonical package identities, exact inspected hashes and known versions.
- [ ] Assert browser target/execution-policy declarations and Ground Control action labels.
- [ ] Assert browser-native page consumes the typed registry.
- [ ] Assert no vendor installers appear in the source/release tree.
- [ ] Run and confirm RED before production changes.

### Task 2: Typed registry + discovery
**Files:**
- Create: `crates/aether-creator-integrations/src/vendor_packages.rs`
- Modify: `crates/aether-creator-integrations/src/lib.rs`
- Create: `crates/aether-creator-integrations/tests/vendor_packages.rs`

- [ ] Implement package descriptors, duplicate-suffix discovery and strict reference-only execution policy.
- [ ] Add Ground Control vendor action labels from inspected MSI evidence.
- [ ] Run structural gate GREEN; host Rust tests remain for the user's Cargo 1.98.1 machine.

### Task 3: Data-only normalizer
**Files:**
- Create: `scripts/vendor-package-normalize.py`
- Modify: release/install script copy lists and clean-break allowlists.

- [ ] Compute SHA-256 and safe package identity without executing packages.
- [ ] Support PE version-resource evidence, MSI textual metadata evidence, and Elgato XAR identity delegation/reference.
- [ ] Emit `vendor_code_executed=false` and fail closed for unsupported input.

### Task 4: Browser surfaces
**Files:**
- Modify: `crates/aether-native-pages/src/lib.rs`

- [ ] Add Authoritative Vendor Baselines section to creator/deck surfaces.
- [ ] Show OBS 32.2.2, Streamlabs 1.21.9, StreamElements package hash/channel, Ground Control 2.1.20 and Elgato 7.5.1 baseline.
- [ ] Keep runtime links pointed at existing embedded browser/native Linux surfaces.

### Task 5: Release integration
**Files:**
- Modify: `scripts/verify.sh`, `scripts/package-release.sh`, `tests/current-clean-break.sh`, README/CHANGELOG/version identity.

- [ ] Wire the new gate into all canonical release verification.
- [ ] Bump source identity from 2.1.47 to 2.1.48 only after feature gates are GREEN.
- [ ] Run full static/package verification and consolidated installer verification once.
