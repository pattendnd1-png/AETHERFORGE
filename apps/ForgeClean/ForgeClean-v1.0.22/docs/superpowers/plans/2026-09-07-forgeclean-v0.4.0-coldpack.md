# ForgeClean v0.4.0 ColdPack Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add shared content-defined chunk deduplication and exact restore for new ColdStore archives without breaking v0.3.2 archives.

**Architecture:** `coldstore.rs` owns ColdPack manifests, chunking, atomic object writes, verification, and both new/legacy restore paths. `ForgeLayout` supplies one shared object-store root. Organizer and CLI archive/restore paths call the explicit shared-store APIs.

**Tech Stack:** Rust 2024, SHA-256, Zstd, serde/serde_json, Unix filesystem metadata, systemd user service.

**Spec:** `docs/superpowers/specs/2026-09-07-forgeclean-v0.4.0-coldpack-design.md`

## Global Constraints

- Canonical release version is 0.4.0.
- Active project trees are never cold-compressed.
- Source removal happens only after chunk, manifest, checksum, and source-identity verification.
- `.fcold.tar.zst` restore compatibility is retained.
- New ColdPack manifests never contain absolute restore paths or symlinks.

---

### Task 1: ColdPack behavior tests
**Files:** Modify `tests/coldstore.rs`; create `tests/regression_v0_4_0.sh`.
**Produces:** Failing behavioral/static contracts for shared dedup, exact restore, and legacy suffix compatibility.

### Task 2: ColdPack engine
**Files:** Modify `src/coldstore.rs`, `Cargo.toml`.
**Produces:** Content-defined chunking, shared object store, JSON manifest, atomic verified archive and restore APIs.

### Task 3: Organizer and CLI integration
**Files:** Modify `src/organizer.rs`, `src/main.rs`.
**Produces:** Canonical global chunk-store path and automatic use for persistent sorting, project rollover, manual archive, and restore.

### Task 4: Release and host verification
**Files:** Modify `README.md`, `build-and-verify.sh`, `install-local.sh`, existing version regression scripts; add v0.4.0 regression gate.
**Produces:** Host tests for dedup reuse, exact restore, legacy restore compatibility, Clippy, all tests, release build, organizer E2E, service persistence, and version-specific VERIFY output.
