# Build-Aware Sorting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Route related project/version artifacts into live canonical build bundles while preserving legacy Downloads paths and generic fallback sorting.

**Architecture:** Add a focused build-identity model to `organizer.rs`. `organize_once` chooses Active-project activation, live build-bundle routing, or existing generic sort/ColdPack behavior in that order. Build artifacts are moved into `Projects/<Project>/Builds/<Version>/` and receive guarded compatibility symlinks at their original Downloads paths.

**Tech Stack:** Rust 2024, std filesystem/symlink APIs, existing ForgeClean organizer/registry/ColdPack modules.

**Spec:** `docs/superpowers/specs/2026-09-08-build-aware-sorting-design.md`

## Global Constraints

- Canonical release remains v0.5.2 because this release has not completed its host/install gate.
- Project/build identity wins over generic type classification.
- Recognized build artifacts stay live and are not immediately ColdPacked.
- Existing real paths and conflicting symlinks are never overwritten.
- Generic unassociated files retain current type-based ColdPack behavior.
- Active project source trees remain live under `Projects/<Project>/Active/`.

---

### Task 1: Build artifact identity

**Files:**
- Modify: `src/organizer.rs`
- Test: `tests/organizer.rs`

**Interfaces:**
- Produces: `BuildIdentity { project: String, version: String }`
- Produces: `build_identity_from_entry(path: &Path) -> Option<BuildIdentity>`
- Produces: `ForgeLayout::project_builds(project)` and `ForgeLayout::project_build(project, version)`

- [ ] Add failing tests for `ForgeClean-v0.5.2-HIT-IT.sh`, `ForgeHX-10.0.30-VERIFY.txt`, and `AetherForge-Control-Center-V10_2_66-AETHERDISPLAY-SHA256SUMS.txt`.
- [ ] Verify the new tests are RED (host Cargo; sandbox static regression when Cargo is unavailable).
- [ ] Implement deterministic version-token parsing and sanitized project/version components.
- [ ] Re-run identity tests to GREEN on the host.

### Task 2: Live build-bundle routing and compatibility aliases

**Files:**
- Modify: `src/organizer.rs`
- Test: `tests/organizer.rs`

**Interfaces:**
- Consumes: `build_identity_from_entry`
- Produces: build destination `Projects/<Project>/Builds/<Version>/<filename>`
- Produces: guarded original-path symlink to canonical file

- [ ] Add a failing organizer test that creates multiple ForgeClean v0.5.2 artifacts and asserts they converge into one Build directory, remain live, and leave correct top-level symlinks.
- [ ] Verify RED.
- [ ] Implement build routing before generic sort/archive.
- [ ] Verify the build bundle test is GREEN on the host.

### Task 3: Fallback scripts and host E2E

**Files:**
- Modify: `src/organizer.rs`
- Modify: `build-and-verify.sh`
- Modify: `README.md`
- Create: `tests/regression_v0_5_2_build_aware.sh`

**Interfaces:**
- Consumes: organizer build routing
- Produces: `FORGECLEAN_BUILD_AWARE_SORT_TEST=PASS`
- Produces: `FORGECLEAN_BUILD_BUNDLE_E2E=PASS`

- [ ] Add `.sh`, `.bash`, `.fish`, and `.run` fallback installer classification without overriding build identity.
- [ ] Add a static regression requiring Build layout, identity parser, live build routing, and compatibility alias behavior.
- [ ] Extend host verification with one complete ForgeClean build bundle plus a ForgeHX artifact and verify generic notes still ColdPack.
- [ ] Update README canonical layout and behavior.
- [ ] Run all static regressions, then host fmt/clippy/tests/release/E2E through HIT-IT.
