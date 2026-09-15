# AetherBrowser v2.1.49 Delta Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add only the missing visual-readiness, AetherForge/Opera-GX shell delta, and full safe browser-takeover wiring to v2.1.48.

**Architecture:** Keep the existing Rust/egui browser shell, X11 child embedding, Stream Deck runtime, vendor registry, and takeover scripts. Add sequencing/readiness markers and strict verifier gates; add visual-token refinements without changing content geometry; extend takeover associations and rollback capture without removing competing packages by default.

**Tech Stack:** Rust/egui, winit/X11, Bash, xdg-mime/xdg-settings, existing package scripts.

**Spec:** `docs/superpowers/specs/2026-09-13-aetherbrowser-v2.1.49-delta-design.md`

## Global Constraints
- Version is 2.1.49.
- Delta-only; preserve existing working behavior.
- No proprietary vendor payload redistribution or execution.
- Blank visual probes are hard failures.
- Browser takeover is dependency-safe and rollback-capable.

---

### Task 1: Visible test readiness
**Files:** `crates/aether-engine-servo/src/live.rs`, `scripts/install-current-tree.sh`, `scripts/streamdeck-plus-runtime-test.sh`, `tests/current-v2-1-49-visible-test-readiness.sh`
- [ ] Run readiness regression and confirm RED.
- [ ] Reveal probe window before visual capture and require one presented visible frame.
- [ ] Capture after present and emit visible/capture-scope markers.
- [ ] Keep Stream Deck Studio alive through input acceptance.
- [ ] Convert blank postinstall visual probes from warning-only to hard failure.
- [ ] Run readiness regression and confirm GREEN.

### Task 2: AetherForge GX shell delta
**Files:** `crates/aether-ui/src/lib.rs`, `tests/current-v2-1-49-gx-dragon-glass-shell.sh`
- [ ] Add failing static visual-contract test.
- [ ] Add reference/version marker and AetherForge GX/DragonGlass tokens.
- [ ] Refine tabs/sidebar/title chrome without changing content geometry.
- [ ] Run visual-contract test and existing layout tests.

### Task 3: Full safe browser takeover delta
**Files:** `scripts/browser-takeover.sh`, `scripts/browser-takeover-rollback.sh`, `tests/current-v2-1-49-browser-takeover.sh`
- [ ] Add failing takeover-contract test.
- [ ] Capture/assign browser MIME and scheme defaults beyond http/https/html.
- [ ] Hide competing launchers safely and record all changes.
- [ ] Extend rollback to restore every captured association.
- [ ] Preserve explicit-only purge behavior.
- [ ] Run takeover-contract test.

### Task 4: Version, verifier, package
**Files:** Cargo manifests, README, CHANGELOG, scripts/verify.sh, scripts/package-release.sh, scripts/package-consolidated.sh, release tests.
- [ ] Promote canonical identity to 2.1.49 without rewriting changelog history.
- [ ] Wire all three v2.1.49 gates into verify/clean-break/package.
- [ ] Run static regression suite.
- [ ] Build consolidated release and verify checksums.
