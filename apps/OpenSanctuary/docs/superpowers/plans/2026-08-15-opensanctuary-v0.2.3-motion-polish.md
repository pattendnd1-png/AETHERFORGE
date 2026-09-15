# OpenSanctuary v0.2.3 Battle.net-Style Motion & Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a richer Battle.net-style visual/motion pass to the v0.2.2 launcher while preserving every working backend path.

**Architecture:** Keep backend state in `LauncherApp`; add a small pure `ui_motion` module for carousel and interpolation behavior, then consume it from egui presentation helpers. Flyouts and carousel selection remain local UI state only.

**Tech Stack:** Rust 2024, eframe/egui 0.35.0, wgpu, existing OpenSanctuary workspace crates.

## Global Constraints

- Rust MSRV: 1.92.
- eframe/egui: 0.35.0.
- No Wine, Proton, DXVK, VKD3D, Windows DLLs, or compatibility-layer dependencies.
- Do not ship Blizzard logos, copyrighted artwork, copied icons, or game assets.
- Keep Diablo III installation access read-only.
- Preserve install discovery, inventory indexing/cache, diagnostics, settings, and native engine launch behavior.
- Startup opens Diablo III Overview.
- Reduced-motion disables decorative time-based motion.

---

### Task 1: Pure carousel and motion helpers

**Files:**
- Create: `apps/launcher/src/ui_motion.rs`
- Modify: `apps/launcher/src/main.rs`
- Test: `apps/launcher/src/ui_motion.rs`

**Interfaces:**
- Produces: `CarouselState::{new,current,next,previous,select}`, `normalized_phase(time, speed) -> f32`, `ease_out_cubic(t) -> f32`, `motion_amount(reduced_motion, time, speed) -> f32`.

- [ ] **Step 1: Write failing unit tests** for carousel wrap, selection clamping, easing endpoints, and reduced-motion returning zero.
- [ ] **Step 2: Run** `cargo test -p opensanctuary-launcher ui_motion` and confirm RED on the Arch verification machine.
- [ ] **Step 3: Implement minimal pure helpers** in `ui_motion.rs` and import the module from `main.rs`.
- [ ] **Step 4: Re-run** the focused tests and confirm GREEN on the Arch machine.
- [ ] **Step 5: Commit** `feat: add launcher motion and carousel helpers`.

### Task 2: Rich favorites/game strip

**Files:**
- Modify: `apps/launcher/src/main.rs`

**Interfaces:**
- Consumes: `motion_amount`, existing `Page` state.
- Produces: richer `game_strip_tile` selected/hover rendering and original procedural Diablo tile art.

- [ ] **Step 1: Replace text-only Diablo tile art** with procedural ember/blue emblem rendering inside the selected tile.
- [ ] **Step 2: Add hover/selected glow and subtle lift** derived only from egui response/time.
- [ ] **Step 3: Keep reduced-motion visually static** while retaining hover color changes.
- [ ] **Step 4: Remove any superseded helper code** to avoid dead-code Clippy failures.
- [ ] **Step 5: Commit** `feat: enrich launcher favorites strip`.

### Task 3: Featured update carousel

**Files:**
- Modify: `apps/launcher/src/main.rs`

**Interfaces:**
- Consumes: `CarouselState`, existing Overview content surface.
- Produces: three original featured cards, arrow/dot navigation, immediate transitions under reduced-motion.

- [ ] **Step 1: Add `featured_carousel: CarouselState` to `LauncherApp`** initialized with three cards.
- [ ] **Step 2: Replace the single featured update panel** with a three-card procedural carousel.
- [ ] **Step 3: Add left/right arrow and dot navigation** without timers or network content.
- [ ] **Step 4: Apply subtle slide/glow interpolation only when reduced-motion is disabled.**
- [ ] **Step 5: Commit** `feat: add featured launcher carousel`.

### Task 4: Notification badges and local account flyout

**Files:**
- Modify: `apps/launcher/src/main.rs`
- Test: `apps/launcher/src/main.rs::ui_state_tests`

**Interfaces:**
- Consumes: existing activity count, diagnostics, page navigation.
- Produces: `account_open: bool`, mutually exclusive notification/social/account flyouts, compact badge rendering.

- [ ] **Step 1: Write state test** proving notification/social/account drawers are represented as mutually exclusive UI state transitions.
- [ ] **Step 2: Add account flyout state and top-bar account control** without credential fields or external connectivity.
- [ ] **Step 3: Add compact notification badge** when unfinished activity exists.
- [ ] **Step 4: Add account actions** for Settings and Diagnostics and local native-session status.
- [ ] **Step 5: Commit** `feat: polish launcher utility flyouts`.

### Task 5: Bottom downloads polish and v0.2.3 release

**Files:**
- Modify: `apps/launcher/src/main.rs`
- Modify: `Cargo.toml`
- Modify: `README.md`
- Modify: `BUILD-ON-ARCH.sh`
- Modify: `scripts/verify.sh`
- Modify: `packaging/arch/PKGBUILD`
- Modify: `packaging/arch/make-source.sh`
- Create: `docs/verification-status-v0.2.3.md`

**Interfaces:**
- Produces: v0.2.3 version/package metadata and updated downloads strip presentation.

- [ ] **Step 1: Add a compact animated activity pulse/progress accent** that becomes static under reduced-motion.
- [ ] **Step 2: Bump workspace/package/release strings to `0.2.3`.**
- [ ] **Step 3: Run static verification locally**: TOML parse, workspace membership, shell syntax, forbidden dependency/API scans, delimiter check, and `git diff --check`.
- [ ] **Step 4: Run `./BUILD-ON-ARCH.sh` on the Rust-equipped Arch system** for fmt/test/Clippy/release build.
- [ ] **Step 5: Commit** `release: prepare OpenSanctuary v0.2.3 UI update`.
