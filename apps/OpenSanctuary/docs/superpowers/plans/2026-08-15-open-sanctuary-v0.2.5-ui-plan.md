# OpenSanctuary v0.2.5 Launcher UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a v0.2.5 UI-only release that further aligns OpenSanctuary's launcher hierarchy and interactions with familiar Battle.net desktop launcher patterns without changing backend behavior.

**Architecture:** Keep launcher backend/model crates unchanged. Add small pure presentation helpers to `apps/launcher/src/ui_motion.rs` and keep rendering changes in `apps/launcher/src/main.rs`; no network or service integration is added.

**Tech Stack:** Rust 1.97, egui/eframe 0.35.0, existing OpenSanctuary workspace.

## Global Constraints
- Original OpenSanctuary branding/assets only.
- No Blizzard authentication, commerce, remote social, or update protocols.
- No compatibility-layer dependencies.
- Read-only user-owned Diablo III content remains unchanged.
- `cargo clippy --workspace --all-targets -- -D warnings` must pass on Arch.

---

### Task 1: Presentation behavior helpers

**Files:**
- Modify: `apps/launcher/src/ui_motion.rs`

**Interfaces:**
- Produces: deterministic queue progress and selection emphasis helpers used by launcher rendering.

- [ ] Add tests for active/completed/failed queue progress and reduced-motion selection emphasis.
- [ ] Implement the minimal pure helpers.
- [ ] Run `cargo test -p opensanctuary-launcher` on a Rust-equipped host.

### Task 2: Favorites / Last Played and launcher menu

**Files:**
- Modify: `apps/launcher/src/main.rs`

**Interfaces:**
- Consumes existing `Page`, `LauncherModel`, and local settings/diagnostics routes.
- Produces no backend API changes.

- [ ] Add a brand launcher-menu state and render Home/Settings/Diagnostics/About-local actions.
- [ ] Add a Last Played group beside Favorites with a Diablo III quick-launch/open entry.
- [ ] Refine selected-game strip rendering and reduced-motion behavior.

### Task 3: Selected-game and stories surface

**Files:**
- Modify: `apps/launcher/src/main.rs`

- [ ] Refine game-control column spacing, state grouping, primary action hierarchy, and version/status chrome.
- [ ] Replace the Latest layout with one lead story, two stacked secondary stories, and a lower compact story row.
- [ ] Keep all story targets local (Content, Diagnostics, Activity, Settings).

### Task 4: Downloads / utility chrome

**Files:**
- Modify: `apps/launcher/src/main.rs`

- [ ] Render queue summary and progress-style rows using the presentation helpers.
- [ ] Harmonize Notifications, Friends, Account, and launcher-menu panel chrome.
- [ ] Preserve existing activity/event model semantics.

### Task 5: Release and packaging

**Files:**
- Modify: `Cargo.toml`
- Modify: `apps/launcher/Cargo.toml`
- Modify: `apps/engine/Cargo.toml`
- Modify: all workspace crate package versions inherited from workspace as applicable
- Modify: `packaging/arch/PKGBUILD`
- Modify: `README.md`
- Modify: `VERIFICATION.md`

- [ ] Bump release metadata to `0.2.5`.
- [ ] Run static manifest/shell/API/dependency scans in the sandbox.
- [ ] On Arch run `./BUILD-ON-ARCH.sh` for fmt, tests, Clippy, and release build.
- [ ] Generate ZIP, TAR.GZ, and corrected v0.2.4 source→v0.2.5 patch artifacts.
