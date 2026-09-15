# AetherBrowser v2.1.44 Integrated Studio Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship AetherBrowser v2.1.44 with full native OBS embedded in-browser, hosted Twitch login UX, editable omnibox caret behavior, restart reintegration, and last-used Stream Deck Marketplace/theme restoration.

**Architecture:** Reuse the existing X11 external-app host to embed the real OBS GUI instead of rebuilding OBS docks. Preserve OBS WebSocket v5 as the control plane. Keep Twitch credentials inside Twitch's hosted page and add a local Stream Deck theme-package registry for editable local copies and last-used restoration.

**Tech Stack:** Rust 2024, Servo/Aether native chrome, X11/x11rb reparenting, OBS Studio + obs-websocket v5, egui/eframe, reqwest blocking OAuth client, Secret Service, shell regression gates.

**Spec:** `docs/superpowers/specs/2026-09-12-aetherbrowser-v2.1.44-integrated-studio-design.md`

## Global Constraints
- Canonical version is exactly 2.1.44; no RC/revision suffixes.
- Twitch/YouTube/YouTube Music browser runtime tests remain excluded as previously user-confirmed known-good.
- External apps embed inside AetherBrowser by default; detached state does not persist across restart.
- Never capture or store the user's Twitch password.
- Never bypass Elgato Marketplace licensing/DRM or modify protected vendor plugin code.

---

### Task 1: Omnibox editing
**Files:** Modify `crates/aether-engine-servo/src/live.rs`, `crates/aether-ui/src/lib.rs`; test `tests/current-v2-1-44-omnibox-editing.sh`.
- [ ] Run the existing regression and confirm PASS or fix only missing caret behavior.
- [ ] Ensure UTF-8-safe Left/Right/Home/End, Backspace/Delete, Ctrl+A, insert-at-caret, and caret painting.
- [ ] Re-run the regression.

### Task 2: Hosted Twitch login UX
**Files:** Modify `crates/aether-deck/src/twitch_oauth.rs`, `crates/aether-deck/src/bin/aether-streamdeck-studio.rs`, `crates/aether-deck/src/bin/aether-deck-monitor.rs`, `crates/aether-engine-servo/src/live.rs`; test `tests/current-v2-1-44-twitch-hosted-login.sh`.
- [ ] Run regression RED.
- [ ] Hide manual Client ID UI and route Connect Twitch through `begin_twitch_account_login`.
- [ ] Keep Twitch username/password/2FA exclusively on Twitch's hosted page opened through the parent browser request path.
- [ ] Surface app-not-configured and Twitch JSON errors explicitly.
- [ ] Re-run regression GREEN.

### Task 3: Full OBS workspace embedded in browser
**Files:** Modify `crates/aether-engine-servo/src/live.rs`, `crates/aether-native-pages/src/lib.rs`; create test `tests/current-v2-1-44-obs-in-window-workspace.sh`.
- [ ] Write RED structural test requiring OBS external-app target, X11 Qt backend, restart adoption, title, and native page CTA.
- [ ] Register `obs` target and add it to known orphan-adoption targets.
- [ ] Add full-workspace CTA in the Stream Studio page and preserve OBS WebSocket dashboard/control plane.
- [ ] Re-run containment/reintegration and OBS contracts GREEN.

### Task 4: Stream Deck Marketplace/theme restore
**Files:** Modify `crates/aether-deck/src/studio_model.rs`, `crates/aether-deck/src/bin/aether-streamdeck-studio.rs`; create test `tests/current-v2-1-44-streamdeck-theme-restore.sh`.
- [ ] Write RED test for package registry, last-used metadata, device compatibility, editable-copy activation, and CONNECTED-triggered restore.
- [ ] Implement local package metadata registry and last-used record under the existing Stream Deck data root.
- [ ] Add Marketplace UI to index/import compatible local packages and activate editable Aether copies.
- [ ] Detect Twitch transition to CONNECTED and restore the last compatible package/profile once per connection.
- [ ] Re-run Studio model/UI tests and new regression GREEN.

### Task 5: Version, verification, and package
**Files:** Modify workspace/package/version files, `scripts/verify.sh`, `tests/current-clean-break.sh`, package scripts, changelog/readme; include all four v2.1.44 regressions.
- [ ] Bump every canonical 2.1.43 identity to 2.1.44.
- [ ] Wire new tests into verify/static/package/clean-break gates.
- [ ] Run all relevant structural/static regressions plus existing containment/Clippy/reintegration/host-summary gates.
- [ ] Build source ZIP, `.run` installer, SHA256SUMS, STATIC-VERIFY, PACKAGE-VERIFY.
- [ ] Verify embedded consolidated payload and checksum manifest before handoff.
