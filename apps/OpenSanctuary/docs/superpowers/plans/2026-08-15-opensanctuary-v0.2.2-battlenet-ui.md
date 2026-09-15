# OpenSanctuary v0.2.2 Battle.net-Style Launcher UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refine the native launcher into a closer Battle.net-style shell without changing the working v0.2.x backend behavior.

**Architecture:** Keep launcher state and backend events in the existing `LauncherApp`, but replace the old vertical game rail/full-width hero presentation with two top chrome rows and a selected-game split layout. New local-only drawers are presentation state only and consume existing activity/diagnostics data.

**Tech Stack:** Rust 2024, eframe/egui 0.35.0, wgpu, existing OpenSanctuary workspace crates.

## Global Constraints

- Rust MSRV: 1.92.
- eframe/egui: 0.35.0.
- No Wine, Proton, DXVK, VKD3D, Windows DLLs, or compatibility-layer dependencies.
- Do not ship Blizzard logos, copyrighted artwork, or game assets.
- Keep Diablo III installation access read-only.
- Preserve existing install discovery, inventory indexing/cache, diagnostics, settings, and native engine launch paths.
- Startup opens Diablo III Overview.

---

### Task 1: Global launcher navigation state

**Files:**
- Modify: `apps/launcher/src/main.rs`
- Test: `apps/launcher/src/main.rs::ui_state_tests`

**Interfaces:**
- Consumes: existing `Page` and `LauncherModel`.
- Produces: `Page::Home`, `Page::Shop`, `Page::is_launcher_page()`, `activity_notification_count(&LauncherModel) -> usize`.

- [ ] **Step 1: Write failing state tests** for Home/Games/Shop classification and notification count.
- [ ] **Step 2: Run** `cargo test -p opensanctuary-launcher ui_state_tests` and confirm the new tests fail because the variants/helpers do not exist.
- [ ] **Step 3: Add the page variants/helpers** with startup still set to `Page::Overview`.
- [ ] **Step 4: Re-run** `cargo test -p opensanctuary-launcher ui_state_tests` and confirm they pass.
- [ ] **Step 5: Commit** `feat: add launcher chrome navigation state`.

### Task 2: Battle.net-style chrome and local drawers

**Files:**
- Modify: `apps/launcher/src/main.rs`

**Interfaces:**
- Consumes: `Page`, `LauncherModel`, `activity_notification_count`.
- Produces: `top_bar`, `game_strip`, `notification_drawer`, `social_drawer` and presentation state `notifications_open`, `social_open`.

- [ ] **Step 1: Replace the old vertical game rail** with a persistent horizontal favorites/game strip beneath the global top bar.
- [ ] **Step 2: Add HOME/GAMES/SHOP navigation** and right-side notification/social/profile controls.
- [ ] **Step 3: Add local-only notification and social drawers** without external services or credential fields.
- [ ] **Step 4: Remove unused vertical-rail helpers** so Clippy does not report dead code.
- [ ] **Step 5: Commit** `feat: add Battle.net-style launcher chrome`.

### Task 3: Selected Diablo III game page layout

**Files:**
- Modify: `apps/launcher/src/main.rs`

**Interfaces:**
- Consumes: existing `PrimaryAction`, inventory model, settings navigation, `paint_hero`.
- Produces: `game_control_column`, `featured_update_panel`, `latest_content_cards`.

- [ ] **Step 1: Replace the Overview full-width hero** with a fixed-width left game control column and flexible content surface.
- [ ] **Step 2: Put game identity, quick links, version, dominant primary action, options, and status in the control column.**
- [ ] **Step 3: Add an original procedural featured update card** and smaller latest-content cards on the right.
- [ ] **Step 4: Keep build metadata/storage metrics below the selected-game surface** and preserve Re-index behavior.
- [ ] **Step 5: Commit** `feat: reshape Diablo page around Battle.net game controls`.

### Task 4: Home and Shop visual surfaces

**Files:**
- Modify: `apps/launcher/src/main.rs`

**Interfaces:**
- Consumes: existing library card helpers and model state.
- Produces: `home_page`, `shop_page`.

- [ ] **Step 1: Add Home page** with featured Diablo III and latest OpenSanctuary/runtime cards.
- [ ] **Step 2: Keep Games as the existing library view.**
- [ ] **Step 3: Add Shop page** as a clearly non-Blizzard, non-commerce placeholder explaining no storefront is connected.
- [ ] **Step 4: Commit** `feat: add Battle.net-style home and shop surfaces`.

### Task 5: Activity chrome polish and v0.2.2 release metadata

**Files:**
- Modify: `apps/launcher/src/main.rs`
- Modify: `Cargo.toml`
- Modify: `README.md`
- Modify: `BUILD-ON-ARCH.sh`
- Modify: `scripts/verify.sh`
- Modify: `packaging/arch/PKGBUILD`
- Modify: `packaging/arch/make-source.sh`

**Interfaces:**
- Produces: workspace/package version `0.2.2` and updated source archive naming.

- [ ] **Step 1: Tighten the bottom downloads/activity strip** with active-count badge and subtle progress chrome.
- [ ] **Step 2: Bump release strings/package version to `0.2.2`.**
- [ ] **Step 3: Run** `./BUILD-ON-ARCH.sh` on the Rust-equipped target environment.
- [ ] **Step 4: Generate Arch/source archives** using `packaging/arch/make-source.sh` and release packaging scripts.
- [ ] **Step 5: Commit** `release: prepare OpenSanctuary v0.2.2 UI update`.
