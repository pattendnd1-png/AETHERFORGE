# Aether Browser Unified Surface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver Aether Browser 2.1.0 as a clean unified browser surface matching the approved render.

**Architecture:** One browser-owned Aether UI WebView owns the complete browser interface and home composition. Servo active content is composited only when the active route is not `aether://home`; live UI state patches in place without reloading the Aether UI document.

**Tech Stack:** Rust 2024, Servo 0.5.0, winit, HTML/CSS, AetherAI Unix sockets, Bash release tooling.

**Spec:** `docs/superpowers/specs/2026-09-09-unified-browser-surface-design.md`

## Global Constraints
- Canonical release version: 2.1.0.
- Approved 2026-09-09 Cosmic Interface render is the visual specification.
- No screenshot-as-interface implementation.
- No inherited split UI runtime or historical compatibility harness.
- One canonical `.run` installer and versioned host verification files.

---

### Task 1: Unified UI contract
- [x] Add failing current render/surface tests.
- [x] Rebuild `aether-ui` around one complete HTML/CSS document.
- [x] Embed only content artwork used by the real DOM/CSS implementation.
- [x] Verify current render and surface contracts.

### Task 2: Servo home composition
- [x] Route `aether://home` to the unified UI document.
- [x] Prevent active content paint/composite on the home route.
- [x] Route mouse, keyboard, scroll, links, and navigation correctly between unified UI and active content.
- [x] Preserve single-load/in-place UI updates and atomic startup reveal.

### Task 3: Current-only cleanup
- [x] Remove retired UI terminology, historical status contracts, and old version markers from production/control files.
- [x] Keep only current unversioned release gates.
- [x] Update current source/package manifests to the unified UI assets.

### Task 4: Installer observability and exclusivity
- [x] Stream Cargo verification output to terminal and VERIFY file.
- [x] Add a non-blocking single-instance install/build lock before output files are truncated.
- [x] Preserve early-failure VERIFY/INSTALL-VERIFY handoff.

### Task 5: Release verification
- [x] Run current static gates and Bash syntax verification locally.
- [x] Run source-package clean-break verification locally.
- [ ] Run Cargo fmt/check/clippy/tests/release build on the AetherForge host.
- [ ] Install package and verify services on the AetherForge host.
- [ ] Compare live runtime against approved render and verify zero flicker.
