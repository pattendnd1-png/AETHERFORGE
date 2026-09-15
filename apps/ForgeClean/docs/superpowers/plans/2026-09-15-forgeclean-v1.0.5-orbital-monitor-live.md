# ForgeClean v1.0.5 Orbital Monitor Live Integration Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the existing NetworkCard-style orbital monitor an actual navigable ForgeClean GUI page.

**Architecture:** Keep the orbital worker as the single synchronization authority. ForgeClean's GUI only reads lightweight status, migration-tail, and `/proc/net/dev` data once per second and renders them inside the normal ForgeClean central page area.

**Tech Stack:** Rust 2024, eframe/egui 0.36, systemd user services, Git/GitHub CLI.

**Spec:** `docs/superpowers/specs/2026-09-15-forgeclean-network-monitor-design.md`

## Global Constraints

- No pie charts.
- NetworkCard-style scrolling graph.
- One-second GUI telemetry refresh only.
- The GUI refresh loop must not run Git, GitHub, hashing, migration scans, or uploads.
- Orbital Sync remains the single scheduling authority.
- GitHub remains the external project-storage backend.
- Strict `cargo fmt`, Clippy `-D warnings`, tests, release build, and GUI self-test gate cutover.

---

### Task 1: Prove the monitor is not yet reachable

- [ ] Add `tests/regression_v1_0_5_orbital_ui.sh`.
- [ ] Run it before production changes and require failure.

### Task 2: Wire the monitor page

- [ ] Export `orbital_ui` from `src/lib.rs`.
- [ ] Add `Orbital Sync` to the GUI page enum/navigation.
- [ ] Render the monitor inside the normal central page.
- [ ] Keep the existing one-second telemetry cache and scrolling history.

### Task 3: Align v1.0.5 versioning

- [ ] Set Cargo package version to `1.0.5`.
- [ ] Derive GUI runtime version from `CARGO_PKG_VERSION`.
- [ ] Update current build/install scripts to `1.0.5`.

### Task 4: Verify and cut over

- [ ] Require the new regression to pass.
- [ ] Run fmt, strict Clippy, all-target/all-feature tests, release build, and GUI self-test.
- [ ] Install all three ForgeClean binaries only after every gate passes.
- [ ] Package source ZIP, checksums, rollback script, and verification TXT.
- [ ] Run one artifact-upload-disabled full orbit and require ForgeClean v1.0.5 to survive on GitHub `main`.
