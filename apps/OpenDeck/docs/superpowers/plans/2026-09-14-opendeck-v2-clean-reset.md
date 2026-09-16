# OpenDeck v2 Clean Reset Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the retired OpenDeck v1.x tree with a clean, usable Windows-style v2 application centered on OBS, Twitch, and Elgato Marketplace connectivity.

**Architecture:** A single Tauri Studio process owns the first-baseline UI and service connections, eliminating background daemons until hardware runtime is reintroduced deliberately. React renders the Windows-style editor; Rust commands own credentials, OBS WebSocket 5.x, Twitch Device Code Flow, browser handoff, and Marketplace/local-content discovery.

**Tech Stack:** Rust 2024, Tauri 2, React 19, TypeScript 5.9, Vite/Vitest, obs-websocket 5.x JSON protocol, Twitch OAuth Device Code Flow.

**Spec:** `docs/superpowers/specs/2026-09-14-opendeck-v2-clean-reset-design.md`

## Global Constraints

- Do not copy OpenDeck v1.x source into v2.
- Windows Stream Deck 7.5.1.22901 is the visual/interaction benchmark.
- No DragonGlass in the first usable baseline.
- No OpenDeck daemon, background service, or autostart in the first baseline.
- Never ask for or store Twitch/Elgato account passwords.
- Never auto-start OBS or start a public stream during qualification.
- Preserve user-owned profiles/config and icon packs in the reset archive.

---

### Task 1: Clean Windows editor shell

**Files:**
- Create: `apps/opendeck-studio/src/App.tsx`
- Create: `apps/opendeck-studio/src/styles.css`
- Test: `apps/opendeck-studio/src/App.test.tsx`

**Interfaces:**
- Produces the 8-key/4-touch/4-dial editor hierarchy used by later integrations.

- [ ] Write the hierarchy test requiring device/profile selectors, Action List, Stream Deck Plus canvas, and bottom Property Inspector.
- [ ] Run the focused test and confirm RED before implementation.
- [ ] Implement the minimal shell and local key-assignment model.
- [ ] Re-run the focused test and require GREEN.

### Task 2: OBS WebSocket 5 connection

**Files:**
- Create/Modify: `apps/opendeck-studio/src-tauri/src/lib.rs`
- Modify: `apps/opendeck-studio/src/bridge.ts`

**Interfaces:**
- Produces `obs_status`, `obs_scene_names`, `obs_set_scene`, `obs_toggle_stream`, `obs_toggle_record`, and `obs_toggle_mute` Tauri commands.

- [ ] Add authentication derivation/unit contract.
- [ ] Implement Hello → Identify → Identified → Request/RequestResponse flow.
- [ ] Wire scene, stream, record, and mute actions to the editor.
- [ ] Require Rust tests plus frontend integration contract.

### Task 3: Twitch public-client sign-in

**Files:**
- Modify: `apps/opendeck-studio/src-tauri/src/lib.rs`
- Modify: `apps/opendeck-studio/src/App.tsx`

**Interfaces:**
- Produces `twitch_begin_auth`, `twitch_poll_auth`, and `twitch_status`.

- [ ] Implement Device Code Flow using the registered public OpenDeck client identity.
- [ ] Store access/refresh tokens in mode-0600 config under `~/.config/opendeck-v2`.
- [ ] Validate and refresh tokens before declaring connected.
- [ ] Render browser activation code and signed-in identity without password/client-secret fields.

### Task 4: Marketplace/local content

**Files:**
- Modify: `apps/opendeck-studio/src-tauri/src/lib.rs`
- Modify: `apps/opendeck-studio/src/App.tsx`

**Interfaces:**
- Produces allowlisted browser handoff and `scan_marketplace_downloads`.

- [ ] Open only official Twitch activation or Elgato Marketplace URLs.
- [ ] Discover supported `.streamDeck*` document families in Downloads and retained icon-pack storage.
- [ ] Render discovered items without claiming unsupported Linux plugin execution.

### Task 5: Build-first destructive cutover

**Files:**
- Create: release installer/reset wrapper in `~/Downloads`.

**Interfaces:**
- Consumes a fully verified v2 source archive.
- Produces a clean user-local v2 install and a dated user-data backup.

- [ ] Build frontend and Rust/Tauri release before touching the current install.
- [ ] Archive old config/profile/icon-pack data.
- [ ] Stop/disable old services and processes.
- [ ] Remove old user-local binaries, desktop entries, units/autostart, active v1.3.1 source tree, and safely identified system package installation.
- [ ] Install the new Studio binary and desktop entry only.
- [ ] Verify old binaries/services/tree are absent and the new binary checksum matches.
- [ ] Leave the new app stopped; emit one verification file.

### Regression requirement — persistence boundary

- [ ] Verify the Studio renders eight keys even when localStorage get/set throw.
- [ ] Keep local key persistence best-effort and never crash the editor on storage failure.
