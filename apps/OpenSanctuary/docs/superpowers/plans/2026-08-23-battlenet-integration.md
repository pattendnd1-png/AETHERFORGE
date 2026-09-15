# OpenSanctuary v0.3.6 Battle.net Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make OpenSanctuary own the visible Battle.net account/install/play workflow while preserving Blizzard-controlled authentication and the existing bridge supervisor.

**Architecture:** Extend `sanctuary-battlenet` with non-secret account metadata and a platform window-host abstraction. Keep process/install orchestration in the existing bridge. Add Battle.net and Account launcher pages that consume those APIs without moving Wine-specific behavior into the UI/native engine.

**Tech Stack:** Rust 2024, egui/eframe 0.35, serde/toml, x11rb on Linux/X11, existing OpenSanctuary task bus and bridge supervisor.

**Spec:** `docs/superpowers/specs/2026-08-23-battlenet-integration-design.md`

## Global Constraints

- Never store or inject raw Battle.net passwords, 2FA codes, recovery codes, or captchas.
- Blizzard-controlled UI remains authoritative for desktop login and install interaction.
- Wine/Windows-specific process/window integration stays in `sanctuary-battlenet`.
- Wayland uses managed companion mode; do not claim arbitrary embedding.
- Existing v0.3.4 bridge health, install detection, update supervision, and one-button launch behavior must remain intact.
- No Blizzard binaries/assets are bundled.

---

### Task 1: Safe Battle.net Account Metadata

**Files:**
- Create: `crates/sanctuary-battlenet/src/account.rs`
- Modify: `crates/sanctuary-battlenet/src/lib.rs`

**Interfaces:**
- Produces `AccountProfile`, `DesktopSessionState`, `default_account_profile_path`, `load_account_profile`, `save_account_profile`, `account_state_from_processes`, `official_account_url`.

- [ ] Write tests for XDG path selection, TOML round-trip, corrupt-file handling, session mapping, and absence of secret fields.
- [ ] Attempt `cargo test -p sanctuary-battlenet account` to establish RED/green when toolchain is available.
- [ ] Implement the minimal account module.
- [ ] Re-run targeted tests when toolchain is available.
- [ ] Commit.

### Task 2: Platform Battle.net Window Host

**Files:**
- Create: `crates/sanctuary-battlenet/src/window_host/mod.rs`
- Create: `crates/sanctuary-battlenet/src/window_host/x11.rs`
- Create: `crates/sanctuary-battlenet/src/window_host/wayland.rs`
- Modify: `crates/sanctuary-battlenet/src/lib.rs`
- Modify: `crates/sanctuary-battlenet/Cargo.toml`
- Modify: root `Cargo.toml`

**Interfaces:**
- Produces `WindowHostMode`, `HostRect`, `HostStatus`, `BattleNetWindowHost::detect`, `sync`, and `release`.

- [ ] Write pure tests for session-mode detection, Battle.net window-title/class matching, and target-rect validation.
- [ ] Add x11rb dependency.
- [ ] Implement X11 discovery/reparent/move/resize with recoverable fallback.
- [ ] Implement Wayland companion-mode adapter.
- [ ] Commit.

### Task 3: Integrated Launcher Pages and Navigation

**Files:**
- Create: `apps/launcher/src/battlenet_page.rs`
- Create: `apps/launcher/src/account_page.rs`
- Modify: `apps/launcher/src/main.rs`
- Modify: `apps/launcher/src/widgets.rs` only for small reusable status card helpers if necessary.

**Interfaces:**
- Adds `Page::BattleNet` and `Page::Account`.
- `LauncherApp` owns `AccountProfile` and `BattleNetWindowHost`.

- [ ] Add navigation/search tests for Battle.net and Account.
- [ ] Add top-level BATTLE.NET nav and Account-page route.
- [ ] Add integrated Battle.net status/host page with INSTALL/OPEN/PLAY actions delegated to existing `LauncherApp` methods.
- [ ] Add account page with non-secret account hint, SIGN IN, MANAGE ACCOUNT, and bridge/session state.
- [ ] Ensure no password input/widget is present.
- [ ] Commit.

### Task 4: Install/Login/Play Integration

**Files:**
- Modify: `apps/launcher/src/main.rs`
- Modify: `crates/sanctuary-launcher/src/lib.rs` only if an additional typed event/state is required.

**Interfaces:**
- Existing install, open Battle.net, and play functions remain canonical.

- [ ] Route INSTALL to the integrated Battle.net page before/while bridge supervision runs.
- [ ] Route SIGN IN to launch/reuse Battle.net and surface host/companion state.
- [ ] Preserve automatic install detection -> verify -> index -> ready.
- [ ] Preserve PLAY native-first + official fallback.
- [ ] Commit.

### Task 5: Release Hardening and Packaging

**Files:**
- Modify: workspace/package version metadata to `0.3.6`.
- Modify: `README.md`, `VERIFICATION.md`, `scripts/verify.sh`, `packaging/arch/PKGBUILD` as applicable.

**Interfaces:** release artifacts only.

- [ ] Add verifier assertions for account/window-host modules and secret-boundary scans.
- [ ] Run manifest parsing, workspace-member resolution, shell syntax, Git whitespace, known Clippy regression scans, and compatibility-boundary scans.
- [ ] Run `cargo fmt --all` and full project verifier when a Rust toolchain is available.
- [ ] Generate ZIP/TAR directly from committed Git tree and a v0.3.4 -> v0.3.6 patch.
- [ ] Validate archive integrity and required contents.
