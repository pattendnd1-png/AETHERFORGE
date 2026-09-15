# OpenSanctuary v0.2.0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add deterministic, cached, read-only Diablo III CASC storage inventorying and expose it through a richer Battle.net-inspired native launcher.

**Architecture:** `sanctuary-casc` owns structured `.build.info` parsing, filesystem inventory construction, fingerprinting, and JSON cache I/O. `sanctuary-launcher` owns cache/index lifecycle and asynchronous events; `apps/launcher` renders model-only Overview/Content/Settings views and never walks the game installation directly.

**Tech Stack:** Rust 2024, Rust >=1.92 (validated target Rust 1.97.x), serde/serde_json, std filesystem/time APIs, eframe/egui 0.35, existing XDG/config helpers.

## Global Constraints

- Version is `0.2.0`.
- Native Linux Rust only; do not execute Blizzard Windows binaries.
- No Wine, Proton, DXVK, VKD3D, Lutris, Bottles, or Winetricks dependencies.
- Blizzard installation access is read-only.
- Inventory cache schema is `1` at `$XDG_CACHE_HOME/opensanctuary/content/inventory-v1.json` (HOME fallback).
- Cache corruption/schema mismatch is non-fatal and treated as absent.
- Cache writes serialize to a sibling temporary file, flush, and rename.
- Entries sort lexicographically by installation-relative path.
- Carry forward the user-proven v0.1 Rust 1.97/Clippy and egui 0.35 compatibility fixes.

---

### Task 1: Restore Proven v0.1 Compatibility and Bump Release

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/sanctuary-core/src/lib.rs`
- Modify: `crates/sanctuary-launcher/src/lib.rs`
- Modify: `apps/launcher/src/main.rs`
- Modify: `packaging/arch/PKGBUILD`
- Modify: `packaging/arch/make-source.sh`

**Interfaces:**
- Consumes: existing v0.1 workspace.
- Produces: v0.2 baseline using derived `Default`, boxed large launcher payloads, egui 0.35 `Panel`, `set_theme/style_mut_of`, and Clippy-clean closures/candidate discovery.

- [ ] Update workspace/package/source versions to `0.2.0`.
- [ ] Apply the exact user-proven Clippy and egui 0.35 compatibility changes.
- [ ] Run `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` on a Rust-equipped host.
- [ ] Commit as `chore: establish v0.2 compatibility baseline`.

### Task 2: Structured Build Metadata and Inventory Model

**Files:**
- Modify: `crates/sanctuary-casc/Cargo.toml`
- Rewrite: `crates/sanctuary-casc/src/lib.rs`

**Interfaces:**
- Produces: `BuildInfo`, `StorageKind`, `StorageEntry`, `ContentInventory`, `parse_build_info`, `build_inventory`, plus v0.1-compatible `ContentStoreSummary`/`probe_content_store`.

- [ ] Write tests for typed headers, `Build Key`/`BuildKey`, unknown columns, malformed/empty data, classification, ordering, and byte totals.
- [ ] Verify tests fail because the new types/functions do not exist.
- [ ] Implement normalized `.build.info` parsing and deterministic recursive inventorying with saturating byte totals.
- [ ] Preserve exact unreadable paths in `SanctuaryError::ContentStoreUnreadable` messages.
- [ ] Run `cargo test -p sanctuary-casc`.
- [ ] Commit as `feat: index local Diablo III storage metadata`.

### Task 3: Fingerprint and Persistent Inventory Cache

**Files:**
- Modify: `crates/sanctuary-casc/src/lib.rs`

**Interfaces:**
- Produces: `read_cached_inventory`, `write_cached_inventory`, `inventory_is_current`, `default_inventory_cache_path`.

- [ ] Write cache round-trip, schema rejection, corrupt-cache, and data-size invalidation tests.
- [ ] Verify tests fail for missing cache functions.
- [ ] Implement deterministic FNV-1a fingerprinting over build/data path, length, and mtime metadata.
- [ ] Implement non-fatal cache reads and atomic temporary-file/flush/rename writes.
- [ ] Run `cargo test -p sanctuary-casc`.
- [ ] Commit as `feat: cache deterministic content inventory`.

### Task 4: Launcher Inventory Lifecycle

**Files:**
- Rewrite: `crates/sanctuary-launcher/src/lib.rs`
- Modify: `crates/sanctuary-diagnostics/src/lib.rs`

**Interfaces:**
- Produces: `InventoryState`, cached `ProbeResult`, `IndexResult`, boxed `LauncherEvent::{ProbeComplete,IndexComplete}`, `spawn_index`, and model lifecycle methods.

- [ ] Write tests for current cache -> ready, missing/stale cache -> index action, successful index -> ready statistics, and failed index -> preserved installation path/error.
- [ ] Verify lifecycle tests fail before implementation.
- [ ] Make probe load only current cache and never build inventory on the UI path.
- [ ] Make index build inventory off-thread, retain in-memory inventory on cache-write failure, and report cache-write failure as activity.
- [ ] Update diagnostics to derive summary/build information from the new CASC model.
- [ ] Run `cargo test -p sanctuary-launcher -p sanctuary-diagnostics`.
- [ ] Commit as `feat: manage cached content indexing lifecycle`.

### Task 5: Battle.net-Inspired Overview and Content Browser

**Files:**
- Rewrite: `apps/launcher/src/main.rs`

**Interfaces:**
- Consumes: `LauncherModel.inventory`, `InventoryState`, `spawn_probe`, `spawn_index`.
- Produces: `Overview`, `Content`, `Settings` secondary navigation; searchable/filterable content list; build strip; archive/index/size cards; cache-state badge; Re-index action.

- [ ] Add UI state for content search and `StorageKind` filtering.
- [ ] Keep all filesystem work behind launcher events; render only model data.
- [ ] Show structured product/version/branch/build/CDN metadata when present.
- [ ] Show stable storage rows with path, kind, and human-readable size.
- [ ] Preserve v0.1 activity, diagnostics, native engine launch, reduced-motion, and locate flows.
- [ ] Run `cargo check -p opensanctuary-launcher` and Clippy with `-D warnings`.
- [ ] Commit as `feat: add indexed content browser to launcher`.

### Task 6: Packaging, Documentation, and Release Verification

**Files:**
- Modify: `README.md`
- Modify: `scripts/verify.sh` only if needed for v0.2 coverage.
- Modify: `docs/verification-status.md`
- Generate: `packaging/arch/OpenSanctuary-0.2.0.tar.gz`

**Interfaces:**
- Produces: user-testable v0.2 source archive and Arch package source archive.

- [ ] Document v0.2 index/cache behavior and non-goals.
- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo test --workspace`.
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] Run `cargo build --workspace --release` (or existing release binary subset if workspace examples are intentionally excluded).
- [ ] Run `./scripts/verify.sh`.
- [ ] Scan manifests/production Rust/PKGBUILD for forbidden compatibility-layer dependencies.
- [ ] Build source archives and verify required files are present.
- [ ] Commit as `release: prepare OpenSanctuary v0.2.0`.
