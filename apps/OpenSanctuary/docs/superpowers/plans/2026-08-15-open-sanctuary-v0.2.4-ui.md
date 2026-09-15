# OpenSanctuary v0.2.4 Launcher UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refine the v0.2.3 native launcher into a denser Battle.net-like shell with local search, stronger selected-game chrome, richer content hierarchy, and a functional downloads drawer.

**Architecture:** Keep launcher backend behavior unchanged and implement all new behavior inside the launcher binary. Add pure helper state for search routing and deterministic progress display so behavior remains testable independently from egui painting.

**Tech Stack:** Rust 2021, eframe/egui 0.35, existing OpenSanctuary workspace crates.

## Global Constraints
- Native Linux only; no Wine, Proton, DXVK, VKD3D, Bottles, Lutris, or Winetricks dependencies.
- Do not copy Blizzard logos, proprietary launcher artwork, credentials UI, or commerce/account behavior.
- Preserve v0.2.3 indexing, Content, diagnostics, settings, and native engine lifecycle behavior.
- Reduced motion disables decorative animation.

---

### Task 1: Local launcher search routing
**Files:** Modify `apps/launcher/src/main.rs`; Test existing `ui_state_tests` module.
**Produces:** deterministic local page routing from a launcher search string.
- [ ] Add tests for Content, Settings, Diagnostics, Activity, Games, and unknown search terms.
- [ ] Implement `search_route` and wire a compact top-bar search field without network behavior.
- [ ] Format and verify on Arch.

### Task 2: Selected-game and global chrome refinement
**Files:** Modify `apps/launcher/src/main.rs`.
**Produces:** denser global header and larger selected-game favorite treatment.
- [ ] Refine top bar spacing/status/account controls.
- [ ] Increase selected game tile identity and add hover/selected accents while preserving reduced-motion behavior.
- [ ] Keep HOME/GAMES/SHOP navigation behavior unchanged.

### Task 3: Content hierarchy refinement
**Files:** Modify `apps/launcher/src/main.rs`.
**Produces:** one featured Latest card plus two compact secondary cards and stronger game-page section hierarchy.
- [ ] Rework Latest card composition.
- [ ] Keep Content/Diagnostics/Activity navigation behavior unchanged.
- [ ] Move technical storage/build surfaces visually below launcher content.

### Task 4: Downloads drawer status rows
**Files:** Modify `apps/launcher/src/main.rs`; Test `ui_state_tests`.
**Produces:** deterministic visual progress fraction and status labels for activity rows.
- [ ] Add tests for active, complete, and failed activity progress/status behavior.
- [ ] Render compact progress/status rows in tray and Activity page.
- [ ] Preserve Activity model semantics.

### Task 5: Release packaging
**Files:** Modify `Cargo.toml`, `README.md`, `BUILD-ON-ARCH.sh`, `scripts/verify.sh`, `packaging/arch/PKGBUILD`, `packaging/arch/make-source.sh`.
**Produces:** OpenSanctuary v0.2.4 source release.
- [ ] Bump release metadata from 0.2.3 to 0.2.4.
- [ ] Run static integrity checks in the sandbox.
- [ ] Generate ZIP, TAR.GZ, and v0.2.3-to-v0.2.4 patch.
