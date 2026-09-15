# OpenSanctuary v0.2.1 Hybrid Launcher UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a Battle.net-inspired hybrid launcher shell that opens directly to Diablo III while adding a launcher-wide Games/Home view and denser game-platform chrome.

**Architecture:** Keep all v0.2 backend crates unchanged. Confine the feature to `apps/launcher/src/main.rs`, release metadata, docs, and packaging. Add only pure helper behavior that can be unit-tested without opening a GUI.

**Tech Stack:** Rust 2024, eframe/egui 0.35.0, wgpu, existing OpenSanctuary launcher model.

## Global Constraints

- Preserve all v0.2 install discovery, content index/cache, diagnostics, settings, and native engine lifecycle behavior.
- Use egui 0.35 APIs only.
- Startup must land directly on Diablo III Overview.
- `GAMES` must expose a launcher-wide Games/Home page.
- Bottom activity tray must default collapsed.
- Use only original OpenSanctuary/procedural graphics; no Blizzard logos or copied artwork.
- No Wine, Proton, DXVK, VKD3D, Lutris, Bottles, or Winetricks dependencies.
- No writes to the Diablo III installation.

---

### Task 1: Navigation and activity-state helpers

**Files:**
- Modify: `apps/launcher/src/main.rs`

**Interfaces:**
- Produces: `Page::is_game_page() -> bool`
- Produces: `Page::games_nav_active() -> bool`
- Produces: `activity_tray_summary(model: &LauncherModel) -> String`

- [ ] **Step 1: Add failing unit tests** for game-page classification, Games nav state, and activity summary.
- [ ] **Step 2: Run** `cargo test -p opensanctuary-launcher --bin opensanctuary-launcher`; expect failures because helpers do not exist yet.
- [ ] **Step 3: Implement the minimal helper methods/functions** and add `Page::Games`.
- [ ] **Step 4: Run the same test command** and expect PASS.
- [ ] **Step 5: Commit** with `feat: add hybrid launcher navigation state`.

### Task 2: Hybrid launcher shell and Games/Home surface

**Files:**
- Modify: `apps/launcher/src/main.rs`

**Interfaces:**
- Consumes: `Page::Games`, `Page::is_game_page()`, existing `LauncherModel`.
- Produces: `games_page(&mut self, ui: &mut egui::Ui)` and revised top/rail navigation.

- [ ] **Step 1: Route top `GAMES` to `Page::Games` and keep startup at `Page::Overview`.**
- [ ] **Step 2: Replace the rail selection predicate with `Page::is_game_page()`.**
- [ ] **Step 3: Implement `games_page` with one featured Diablo III card, current install/index state, primary action, and inactive future-title placeholders.**
- [ ] **Step 4: Tighten top-bar/rail dimensions and paint selected rail accent.**
- [ ] **Step 5: Run format/check/clippy commands and commit** `feat: add Battle.net-inspired games shell`.

### Task 3: Selected-game hero and compact activity tray

**Files:**
- Modify: `apps/launcher/src/main.rs`

**Interfaces:**
- Consumes: existing `on_primary_action`, inventory model, activity list.
- Produces: revised selected-game header/hero and compact activity tray.

- [ ] **Step 1: Add selected-game title/subnav header with Overview, Content, Activity, Settings.**
- [ ] **Step 2: Increase hero hierarchy, paint a dedicated blue primary-action surface, add compact options/re-index controls, and move technical metadata below the primary action.**
- [ ] **Step 3: Default `activity_open` to false and use `activity_tray_summary` in collapsed mode; expanded mode shows at most three recent tasks.**
- [ ] **Step 4: Refine global colors/spacing using only egui 0.35-supported APIs.**
- [ ] **Step 5: Run format/check/clippy commands and commit** `feat: refine selected game and activity chrome`.

### Task 4: v0.2.1 release metadata and package

**Files:**
- Modify: `Cargo.toml`
- Modify: `scripts/verify.sh`
- Modify: `packaging/arch/make-source.sh`
- Modify: `packaging/arch/PKGBUILD`
- Modify: `README.md`

**Interfaces:**
- Produces: source version `0.2.1` and Arch package `pkgver=0.2.1`.

- [ ] **Step 1: Bump workspace/package/release strings to `0.2.1`.**
- [ ] **Step 2: Document the hybrid Games/Home shell and direct-to-Diablo startup.**
- [ ] **Step 3: Run `./scripts/verify.sh`.**
- [ ] **Step 4: Run `packaging/arch/make-source.sh` and inspect archive contents.**
- [ ] **Step 5: Commit** `release: prepare OpenSanctuary v0.2.1 UI update`.
