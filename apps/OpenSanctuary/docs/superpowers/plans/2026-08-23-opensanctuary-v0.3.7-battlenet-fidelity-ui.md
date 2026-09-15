# OpenSanctuary v0.3.7 Battle.net-Fidelity UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the approved Battle.net-fidelity launcher shell, Diablo III page, integrated Battle.net/account surfaces, downloads/notifications experience, and progressive diagnostics while preserving v0.3.6 backend behavior.

**Architecture:** Keep `LauncherApp` as orchestration owner and move presentation into focused egui modules that communicate through view structs and action enums. Existing backend methods remain canonical; new modules never spawn processes, mutate bridge records, or capture credentials directly.

**Tech Stack:** Rust 2024, egui/eframe 0.35, existing OpenSanctuary launcher/battlenet crates.

**Spec:** `docs/superpowers/specs/2026-08-23-opensanctuary-v0.3.7-battlenet-fidelity-ui-design.md`

## Global Constraints

- Release version is exactly `0.3.7`; no r/RC/revision suffixes.
- Preserve existing Battle.net credential-safety boundary.
- Preserve X11 embedded host and Wayland companion behavior.
- Preserve canonical install/index/play orchestration in `LauncherApp`.
- Do not add proprietary Blizzard assets or fabricated social/download data.
- Strict Clippy with `-D warnings` remains a release gate on Arch.

---

### Task 1: Extract Battle.net-style launcher chrome

**Files:**
- Create: `apps/launcher/src/chrome.rs`
- Modify: `apps/launcher/src/main.rs`
- Test: `apps/launcher/src/chrome.rs`

**Interfaces:**
- Produces `TopBarAction`, `TopBarView`, `GameStripAction`, `GameStripView`, `render_top_bar`, `render_game_strip`.
- `main.rs` maps actions to existing `Page` and `UtilityDrawer` values.

- [ ] **Step 1: Write failing action tests**

```rust
#[test]
fn top_bar_actions_are_player_facing() {
    assert_eq!(TopBarAction::Home.label(), "HOME");
    assert_eq!(TopBarAction::BattleNet.label(), "BATTLE.NET");
    assert_eq!(TopBarAction::Downloads.label(), "DOWNLOADS");
}
```

- [ ] **Step 2: Verify the tests fail before the new module exists**

Run: `cargo test -p opensanctuary-launcher top_bar_actions_are_player_facing`
Expected: compile failure because `chrome`/`TopBarAction` do not exist.

- [ ] **Step 3: Implement chrome view/action types and rendering**

Render compact top navigation, notification/download/account controls, local search, favorites, and last-played. Do not mutate launcher state inside the module.

- [ ] **Step 4: Wire actions in `main.rs` and remove old `top_bar`/`game_strip` bodies**

Map each returned action to the same existing page/drawer state transitions.

- [ ] **Step 5: Verify**

Run: `cargo test -p opensanctuary-launcher chrome`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/launcher/src/chrome.rs apps/launcher/src/main.rs
git commit -m "refactor: extract Battle.net-style launcher chrome"
```

### Task 2: Build player-facing Diablo III hero and game status

**Files:**
- Create: `apps/launcher/src/game_page.rs`
- Modify: `apps/launcher/src/main.rs`
- Test: `apps/launcher/src/game_page.rs`

**Interfaces:**
- Produces `GamePageAction`, `GamePageView`, `GameReadiness`, `render_game_hero`, `render_game_status`.
- Consumes strings/booleans and existing `PrimaryAction`; no backend side effects.

- [ ] **Step 1: Write failing lifecycle tests**

```rust
#[test]
fn player_status_hides_advanced_bridge_details() {
    let labels = player_status_labels();
    assert!(labels.contains(&"Battle.net"));
    assert!(labels.contains(&"Installation"));
    assert!(labels.contains(&"Content"));
    assert!(!labels.contains(&"Runner"));
    assert!(!labels.contains(&"Prefix"));
}
```

- [ ] **Step 2: Verify RED**

Run: `cargo test -p opensanctuary-launcher player_status_hides_advanced_bridge_details`
Expected: compile failure because the new helper does not exist.

- [ ] **Step 3: Implement hero/status rendering**

Create a large dark-blue Diablo hero, title treatment, build/region caption, one primary action, settings/overflow buttons, and four-line compact GAME STATUS. Use semantic lifecycle text; no fabricated percentage.

- [ ] **Step 4: Replace `overview_page` control-column hierarchy**

Keep existing backend action resolution; route Details→Diagnostics, Settings→Settings, Open Battle.net→BattleNet, Locate/Repair/Re-index through existing methods.

- [ ] **Step 5: Verify**

Run: `cargo test -p opensanctuary-launcher game_page`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add apps/launcher/src/game_page.rs apps/launcher/src/main.rs
git commit -m "feat: redesign Diablo III game page"
```

### Task 3: Add persistent downloads/activity experience

**Files:**
- Create: `apps/launcher/src/downloads.rs`
- Modify: `apps/launcher/src/main.rs`
- Test: `apps/launcher/src/downloads.rs`

**Interfaces:**
- Produces `DownloadsAction`, `DownloadsView`, `downloads_summary`, `render_downloads_tray`.
- Consumes `BridgeState`, `InventoryState`, and a borrowed activity slice.

- [ ] **Step 1: Write failing summary tests**

```rust
#[test]
fn active_bridge_work_beats_idle_summary() {
    let summary = downloads_summary(BridgeState::Updating, InventoryState::Indexed, &[]);
    assert_eq!(summary, "Diablo III • Updating");
}
```

- [ ] **Step 2: Verify RED**

Run: `cargo test -p opensanctuary-launcher active_bridge_work_beats_idle_summary`
Expected: compile failure before `downloads_summary` exists.

- [ ] **Step 3: Implement tray and expanded drawer**

Show active task count, semantic lifecycle, recent local tasks, and View All/Details actions. Numeric transfer rate/bytes are omitted unless actually available.

- [ ] **Step 4: Replace old `activity_tray` rendering while preserving `activity_open` state**

Map actions to Activity/Diagnostics pages.

- [ ] **Step 5: Verify and commit**

```bash
cargo test -p opensanctuary-launcher downloads
git add apps/launcher/src/downloads.rs apps/launcher/src/main.rs
git commit -m "feat: add launcher downloads surface"
```

### Task 4: Redesign notifications and account drawer

**Files:**
- Create: `apps/launcher/src/notifications.rs`
- Modify: `apps/launcher/src/main.rs`
- Modify: `apps/launcher/src/account_page.rs`
- Test: `apps/launcher/src/notifications.rs`

**Interfaces:**
- Produces `NotificationEntry`, `notification_entries`, `render_notifications`.
- Account page keeps existing `AccountPageAction` contract.

- [ ] **Step 1: Write failing notification grouping test**

```rust
#[test]
fn notification_entries_include_active_game_work() {
    let entries = notification_entries(BridgeState::Indexing, &[]);
    assert!(entries.iter().any(|entry| entry.title.contains("Diablo III")));
}
```

- [ ] **Step 2: Verify RED**

Run: `cargo test -p opensanctuary-launcher notification_entries_include_active_game_work`
Expected: compile failure before the helper exists.

- [ ] **Step 3: Implement grouped notification rendering**

Use existing bridge/activity state only. Do not fabricate Battle.net service or social data.

- [ ] **Step 4: Simplify Account page hierarchy**

Make BattleTag, region, desktop session, and Diablo access primary. Route advanced bridge/session details to Diagnostics. Preserve the no-password-capture test.

- [ ] **Step 5: Verify and commit**

```bash
cargo test -p opensanctuary-launcher notifications
cargo test -p opensanctuary-launcher account_page_never_enables_password_capture
git add apps/launcher/src/notifications.rs apps/launcher/src/account_page.rs apps/launcher/src/main.rs
git commit -m "feat: redesign notifications and account UX"
```

### Task 5: Simplify the integrated Battle.net page

**Files:**
- Modify: `apps/launcher/src/battlenet_page.rs`
- Modify: `apps/launcher/src/main.rs`
- Test: `apps/launcher/src/battlenet_page.rs`

**Interfaces:**
- Preserve existing `BattleNetPageAction`, `BattleNetPageView`, and `BattleNetPageOutput` names where practical.
- Add only presentation helpers; host rectangle remains returned to `LauncherApp` for X11 embedding.

- [ ] **Step 1: Add a failing presentation test**

```rust
#[test]
fn primary_surface_labels_do_not_expose_runner_or_prefix() {
    let labels = primary_surface_labels();
    assert!(!labels.contains(&"RUNNER"));
    assert!(!labels.contains(&"PREFIX"));
}
```

- [ ] **Step 2: Verify RED**

Run: `cargo test -p opensanctuary-launcher primary_surface_labels_do_not_expose_runner_or_prefix`
Expected: compile failure before the helper exists.

- [ ] **Step 3: Redesign the page**

Make the integrated client host the dominant canvas. Show three compact status summaries and primary/open/sign-in/account controls. Move repair/deep status to Details/Diagnostics.

- [ ] **Step 4: Verify and commit**

```bash
cargo test -p opensanctuary-launcher battlenet_page
git add apps/launcher/src/battlenet_page.rs apps/launcher/src/main.rs
git commit -m "feat: simplify integrated Battle.net surface"
```

### Task 6: Add diagnostics presentation helpers and finish module split

**Files:**
- Create: `apps/launcher/src/diagnostics.rs`
- Modify: `apps/launcher/src/main.rs`
- Modify: `apps/launcher/src/widgets.rs`
- Test: `apps/launcher/src/diagnostics.rs`

**Interfaces:**
- Produces focused advanced-status render helpers only.
- Existing diagnostics export and repair methods stay in `LauncherApp`.

- [ ] **Step 1: Add advanced-label tests**

```rust
#[test]
fn advanced_diagnostics_keeps_bridge_fields() {
    let labels = advanced_labels();
    assert!(labels.contains(&"Bridge"));
    assert!(labels.contains(&"Runner"));
    assert!(labels.contains(&"Content index"));
}
```

- [ ] **Step 2: Verify RED, implement helpers, and rewire diagnostics page**

Run: `cargo test -p opensanctuary-launcher advanced_diagnostics_keeps_bridge_fields`
Expected before implementation: compile failure. Expected after implementation: PASS.

- [ ] **Step 3: Remove imports/helpers from `main.rs` that are no longer directly used**

Use symbol scans so test-only imports remain under `#[cfg(test)]` and avoid another strict-Clippy unused-import regression.

- [ ] **Step 4: Commit**

```bash
git add apps/launcher/src/diagnostics.rs apps/launcher/src/main.rs apps/launcher/src/widgets.rs
git commit -m "refactor: isolate advanced diagnostics presentation"
```

### Task 7: Release v0.3.7 and harden verification

**Files:**
- Modify: `Cargo.toml`
- Modify: `PKGBUILD`
- Modify: `README.md`
- Modify: `VERIFICATION.md`
- Modify: `BUILD-ON-ARCH.sh`
- Modify: `scripts/verify.sh`
- Modify: relevant v0.3.7 spec/plan text if needed for final identity

**Interfaces:**
- Produces canonical release identity `0.3.7` and verifier checks for module boundaries/security policy.

- [ ] **Step 1: Bump every canonical release identity from 0.3.6 to 0.3.7**

Use an exact version sweep and ensure no revision/RC suffix appears.

- [ ] **Step 2: Extend verifier checks**

Require `chrome.rs`, `game_page.rs`, `downloads.rs`, `notifications.rs`, and `diagnostics.rs`; preserve boxed-X11 and credential-boundary checks; scan for known Clippy regression shapes.

- [ ] **Step 3: Run full host verification**

Run: `cargo fmt --all -- --check && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo build --workspace --release`
Expected on Arch: all commands exit 0.

- [ ] **Step 4: Run package/static verification in the build environment**

Parse all Cargo manifests, resolve workspace members, run shell syntax checks, version sweep, compatibility-boundary scan, credential scan, revision-marker scan, and archive integrity checks.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "release: OpenSanctuary v0.3.7"
```

- [ ] **Step 6: Package**

Create exactly:
- `OpenSanctuary-0.3.7-source.zip`
- `OpenSanctuary-0.3.7-source.tar.gz`
- `OpenSanctuary-v0.3.6-to-v0.3.7.patch`
