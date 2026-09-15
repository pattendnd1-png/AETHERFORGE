# ReForge Logitech Linux v0.1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a safe Rust userspace configuration stack for Logitech HID++ devices with real adjustable-DPI control, profiles, daemon, CLI, GUI, and Arch packaging.

**Architecture:** A pure protocol crate owns HID++ framing/parsing. A hidapi backend owns hardware I/O. Core owns data/RPC/profile contracts. Daemon, CLI, and GUI consume those interfaces without exposing arbitrary HID writes.

**Tech Stack:** Rust 2024 (MSRV 1.92), hidapi 2.6.6 native Linux backend, serde/serde_json, clap 4.6, eframe/egui 0.35, Unix domain sockets, systemd user services, udev, Arch PKGBUILD.

## Global Constraints
- Only vendor ID `0x046d` is managed.
- No DFU/firmware operations.
- No arbitrary raw-report API.
- DPI writes require confirmed feature `0x2201`, range validation, and read-back verification.
- Normal input remains owned by Linux kernel HID drivers.
- UI and package identify themselves as unofficial.

---

### Task 1: Workspace contracts and protocol tests
**Files:** workspace `Cargo.toml`; `crates/reforge-core/*`; `crates/reforge-protocol/*`.

- [ ] Write failing protocol tests for short/long framing, captured root feature resolution, feature enumeration, DPI range/state parsing, DPI validation, and set-DPI frame encoding.
- [ ] Add core tests for profile matching.
- [ ] Implement the minimum serializable models, RPC enums, HID++ request/response types, feature constants, and DPI parser needed for tests.
- [ ] Run `cargo test -p reforge-protocol -p reforge-core`.
- [ ] Commit protocol/core.

### Task 2: Linux hidraw userspace driver
**Files:** `crates/reforge-hid/src/lib.rs`, `crates/reforge-hid/src/transport.rs`, `crates/reforge-hid/src/probe.rs`, tests.

- [ ] Write tests for response matching and device-index probe ordering using hardware-independent helpers.
- [ ] Implement Logitech-only hidapi enumeration and safe HID++ request/response transport with timeouts.
- [ ] Implement root feature resolution, feature-set enumeration, adjustable-DPI probe/read/set/read-back.
- [ ] Run crate tests.
- [ ] Commit HID backend.

### Task 3: Daemon and RPC client
**Files:** `crates/reforge-daemon/*`; RPC client additions in `reforge-core`.

- [ ] Write round-trip tests for one-request-per-line JSON RPC framing.
- [ ] Implement per-user Unix listener with stale socket cleanup and `0600` permissions.
- [ ] Implement health, list devices, get/set DPI, list/save/apply profile operations.
- [ ] Run tests and a local socket smoke test with no hardware required.
- [ ] Commit daemon.

### Task 4: CLI
**Files:** `crates/reforge-cli/*`.

- [ ] Define clap command tree for health/devices/DPI/profile actions.
- [ ] Implement daemon RPC calls and JSON output mode.
- [ ] Run `--help` and daemon health smoke tests.
- [ ] Commit CLI.

### Task 5: GUI
**Files:** `crates/reforge-gui/*`.

- [ ] Implement eframe 0.35 `App::ui` with device list, details, feature IDs, adjustable-DPI control, profiles, refresh/apply, and daemon errors.
- [ ] Keep hardware work off the egui frame path except short RPC calls triggered by explicit actions.
- [ ] Build GUI target.
- [ ] Commit GUI.

### Task 6: Arch package and installer
**Files:** `packaging/arch/PKGBUILD`, `packaging/udev/70-reforge-logitech.rules`, `packaging/systemd/reforge-logitechd.service`, desktop/icon files, `scripts/install-arch.sh`, `README.md`.

- [ ] Install binaries, service, udev rule, desktop entry, icon, docs, and license through PKGBUILD.
- [ ] Add local Arch install script using `makepkg -si`.
- [ ] Add exact launch/test commands and support matrix to README.
- [ ] Validate shell syntax, desktop file structure, TOML/JSON syntax, and package file paths.
- [ ] Commit packaging.

### Task 7: Release verification and bundle
- [ ] Run `cargo fmt --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo build --release --workspace` when a Rust toolchain is available.
- [ ] If compiler installation is blocked by the environment, record that limitation and do not claim compilation succeeded.
- [ ] Run non-Rust structural checks regardless.
- [ ] Create source tarball/zip with checksums and release notes.
