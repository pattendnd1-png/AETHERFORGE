# ReForge Logitech v0.2 Lighting + Control Center Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add typed Logitech lighting control and a system-themed control-center GUI while preserving the v0.1 installation path.

**Architecture:** Add lighting models/protocol builders first, wire them through HID/RPC/profile persistence, then add the system-themed GUI and upgrade packaging. Unsupported device capabilities stay disabled rather than emulated.

**Tech Stack:** Rust 2024, hidapi, serde/serde_json, eframe/egui, Unix sockets, systemd user service, Arch makepkg.

## Global Constraints
- Preserve v0.1 DPI behavior and profile compatibility.
- No firmware flashing.
- No public arbitrary raw HID report injection.
- Enable lighting controls only from discovered HID++ feature support.

---

### Task 1: Lighting models and profile schema
- [ ] Add typed RGB, effect, lighting capability/state models.
- [ ] Extend profiles with optional lighting and application match rules using serde defaults.
- [ ] Add profile compatibility tests.

### Task 2: Protocol lighting builders
- [ ] Add typed request builders for feature-controlled lighting and per-key batches.
- [ ] Add encoding/validation tests before implementation.

### Task 3: HID lighting backend
- [ ] Detect lighting capabilities from enumerated HID++ features.
- [ ] Add typed static/effect/per-key write operations.
- [ ] Keep unsupported feature paths read-only/disabled.

### Task 4: RPC and CLI
- [ ] Add get/apply lighting requests and responses.
- [ ] Extend profile application to apply lighting as well as DPI.
- [ ] Add CLI lighting status/static/effect commands.

### Task 5: Host lighting engine
- [ ] Add gradient/wave/breathing/ripple frame generation as pure functions.
- [ ] Add source models for screen/audio reactive modes and availability reporting.
- [ ] Add unit tests for deterministic frame generation.

### Task 6: System-themed GUI
- [ ] Follow system theme using egui ThemePreference::System.
- [ ] Add navigation rail and dashboard/device cards.
- [ ] Add Lighting page with effect, color, brightness, live apply, and per-key editor.
- [ ] Add Profiles, Integrations, and Settings pages.

### Task 7: Packaging and upgrade installer
- [ ] Bump workspace/package version to 0.2.0.
- [ ] Preserve current udev/systemd/desktop installation.
- [ ] Add upgrade helper that extracts/builds/reinstalls over v0.1.
- [ ] Build source zip/tar release and checksums.
