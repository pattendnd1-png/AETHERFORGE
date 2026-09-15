# ForgeHX Tray + Communication Routing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make ForgeHX close its GUI process to a persistent tray host and route communication capture through ForgeHX Processed Mic.

**Architecture:** Split `ksni` tray ownership into a new `forgehx-tray` binary managed by systemd; let `forgehx-gui` exit normally. Add daemon-owned WirePlumber default-source enforcement after the processed mic is available.

**Tech Stack:** Rust, eframe/egui, ksni StatusNotifierItem, systemd user services, PipeWire, WirePlumber/wpctl.

**Spec:** `docs/superpowers/specs/2026-09-01-tray-and-chat-routing-design.md`

## Global Constraints
- Canonical version is 10.0.6.
- No Bluetooth fallback for mic monitoring.
- Do not restart PipeWire or WirePlumber.
- Do not continuously override explicit per-app microphone choices.

---

### Task 1: Split tray process
**Files:** create `crates/forgehx-tray/*`; modify GUI app/main/Cargo, workspace, package files, desktop and systemd units.
- [ ] Add failing source contract test.
- [ ] Verify RED.
- [ ] Implement standalone tray and normal GUI close.
- [ ] Verify GREEN.

### Task 2: Communication default routing
**Files:** modify `forgehx-dsp` and daemon integration; add source contract test.
- [ ] Add failing routing test.
- [ ] Verify RED.
- [ ] Implement processed-source resolution and `wpctl` routing policy.
- [ ] Verify GREEN.

### Task 3: Release 10.0.6
**Files:** version metadata, package gate, README, release artifacts.
- [ ] Retarget release identity to 10.0.6.
- [ ] Run full static/package gate.
- [ ] Build canonical source/build kit/updater/checksums/verify files.
