# Aether Browser v2.1.0 Native Chrome Reset Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Replace the dual-Servo/offscreen chrome runtime with a native Rust shell and one Servo web-content surface.

**Architecture:** `winit` owns the top-level window, Servo owns only web content, and `egui`/`egui_glow` paint browser chrome and Home directly onto Servo's parent OpenGL context. Home has zero dependency on any chrome/content WebView.

**Tech Stack:** Rust 2024, Servo 0.5.0, winit 0.30, egui 0.34.3, egui_glow 0.34.3, glow 0.17, image 0.25.

**Spec:** `docs/superpowers/specs/2026-09-09-native-chrome-reset-design.md`

## Global Constraints

- Canonical version is `2.1.0`.
- No old DragonGlass panel/compositor subsystem.
- No offscreen chrome WebView or browser HTML chrome document.
- Servo renders actual web content only.
- `aether://home` is fully native Rust UI.
- AetherAI remains the existing 0.3.7 Unix-socket integration.
- One canonical installer artifact and version-specific VERIFY/INSTALL-VERIFY handoff files.
- Visual completion requires host live screenshot evidence.

---

### Task 1: Native UI renderer

**Files:**
- Modify: `crates/aether-ui/Cargo.toml`
- Replace: `crates/aether-ui/src/lib.rs`
- Replace: `crates/aether-ui/tests/chrome.rs`
- Delete: `crates/aether-ui/assets/browser.html`

**Interfaces:**
- Produces `ChromeLayout`, `ChromeModel`, `ChromeTab`, `ChromeHitTarget`, `NativeChromeRenderer::paint`, and existing `DragonGlassTokens`.

- [x] Write tests asserting native layout geometry, Home display URL, and hit targets without HTML/DOM rendering.
- [x] Run the tests against the old UI and confirm the native-renderer contract fails.
- [x] Implement the native renderer with egui drawing primitives and embedded ordinary image assets.
- [x] Run UI tests and static no-HTML checks.

### Task 2: Single-content Servo runtime

**Files:**
- Replace: `crates/aether-engine-servo/src/live.rs`
- Modify: `crates/aether-engine-servo/Cargo.toml`

**Interfaces:**
- Consumes `NativeChromeRenderer` and `ChromeModel`.
- Produces `run_live_browser(initial_url: &str)` with native shell ownership and one active Servo content surface.

- [x] Add a failing architecture test that rejects chrome WebView/offscreen chrome symbols and requires native GL shell rendering.
- [x] Remove `ChromeDelegate`, `chrome_context`, `chrome_webview`, DOM patch refresh, and chrome frame-ready startup dependency.
- [x] Draw native shell directly to parent GL surface after optional web-content composite.
- [x] Route Home input only to native actions and web input only to the content WebView.
- [x] Run architecture/unit tests.

### Task 3: v2.1.0 status and verifier reset

**Files:**
- Modify: `crates/aether-browser/src/main.rs`
- Replace/update: `tests/current-shell-contract.sh`
- Replace/update: `tests/current-render-contract.sh`
- Update: `tests/current-no-flicker.sh`
- Update: `scripts/verify.sh`
- Update packaging/install scripts as needed for `2.1.0`.

**Interfaces:**
- Status reports `AETHER_BROWSER_UI_RENDERER=NATIVE_EGUI_GLOW` and `AETHER_BROWSER_SERVO_SURFACES=WEB_CONTENT_ONLY`.

- [x] Make old current contracts fail against v2.0.x assumptions.
- [x] Replace them with architecture assertions that test ownership rather than source formatting.
- [x] Run every current static gate.

### Task 4: Package and handoff

**Files:**
- Update: `README.md`, `CHANGELOG.md`, current architecture docs.
- Produce: `/mnt/data/aether-browser-v2.1.0-release/Aether-Browser-v2.1.0-INSTALL.run`

- [x] Package one canonical source archive and installer.
- [x] Run exact installer `--verify` and static/package checks in the container.
- [x] Hash the final `.run`.
- [x] Handoff one `~/Downloads` command and require host VERIFY + INSTALL-VERIFY + live screenshot.
