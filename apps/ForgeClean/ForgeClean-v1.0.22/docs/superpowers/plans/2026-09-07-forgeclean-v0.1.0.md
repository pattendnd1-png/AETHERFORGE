# ForgeClean v0.1.0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a testable Rust CLI that scans Pacman package caches into identity-bound cleanup manifests and permanently unlinks only approved superseded archives/partials.

**Architecture:** A library owns package parsing, version ranking, scanning, manifest serialization, safety validation, and deletion. A thin CLI converts user arguments into library calls. Purge is manifest-gated and fail-closed.

**Tech Stack:** Rust 2024 edition, standard library only, Arch `vercmp` at runtime for package ordering.

**Spec:** `docs/superpowers/specs/2026-09-07-forgeclean-v0.1.0-design.md`

## Global Constraints
- Direct permanent deletion only; never desktop Trash.
- Never recursively delete directories.
- Default retention is 2 package versions.
- Package deletion fails closed if `vercmp` is unavailable.
- Purge revalidates root confinement and file identity before unlink.

---

### Task 1: Package parsing and retention
**Files:** `src/package.rs`, `src/lib.rs`, `tests/package.rs`
**Interfaces:** `parse_package_filename(&Path) -> Option<PackageArchive>`; `select_superseded(Vec<PackageArchive>, usize, &dyn VersionComparator) -> Result<Vec<PackageArchive>, String>`.
- [ ] Write failing parsing/retention tests.
- [ ] Run tests and verify RED.
- [ ] Implement parser and comparator-backed retention.
- [ ] Run tests and verify GREEN.

### Task 2: Scanner and manifest
**Files:** `src/scan.rs`, `src/manifest.rs`, `tests/scan_manifest.rs`
**Interfaces:** `scan_cache(&ScanOptions, &dyn VersionComparator) -> Result<CleanupBatch, String>`; `CleanupBatch::{write_to,read_from}`.
- [ ] Write failing scanner/manifest tests.
- [ ] Verify RED.
- [ ] Implement regular-file scan, partial detection, identity metadata, and manifest round-trip.
- [ ] Verify GREEN.

### Task 3: Manifest-gated purge
**Files:** `src/purge.rs`, `tests/purge.rs`
**Interfaces:** `verify_batch(&CleanupBatch) -> VerificationReport`; `purge_batch(&CleanupBatch) -> PurgeReport`.
- [ ] Write failing successful-delete and refusal tests.
- [ ] Verify RED.
- [ ] Implement confinement, symlink, metadata identity checks, and `remove_file` deletion.
- [ ] Verify GREEN.

### Task 4: CLI and host verification
**Files:** `src/main.rs`, `build-and-verify.sh`, `README.md`
**Interfaces:** commands `scan`, `verify`, `purge`.
- [ ] Write CLI smoke tests around argument parsing/library behavior where practical.
- [ ] Implement thin CLI.
- [ ] Run `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test`.
- [ ] Build release binary and emit `ForgeClean-v0.1.0-VERIFY.txt` and SHA256 sums.
