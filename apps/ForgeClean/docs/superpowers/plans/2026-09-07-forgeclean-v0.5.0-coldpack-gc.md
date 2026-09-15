# ForgeClean v0.5.0 ColdPack Store Maintenance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add fail-closed reference-aware ColdPack status, audit, quarantine GC, 7-day purge, and persistent daily maintenance.

**Architecture:** Keep archive/restore format logic in `coldstore.rs`; expose crate-private manifest/object inspection and store lock primitives. Add `coldpack_gc.rs` as the sole owner of mark/sweep/quarantine policy. CLI and watch loop call that module using the existing `ForgeLayout` root/store paths.

**Tech Stack:** Rust 2024, Rust 1.98 host baseline, std `File::lock`/`lock_shared`, serde/serde_json, SHA-256, Zstd, systemd --user, Bash host verification.

**Spec:** `docs/superpowers/specs/2026-09-07-forgeclean-v0.5.0-coldpack-gc-design.md`

## Global Constraints

- Canonical version is `0.5.0`; one authoritative artifact, no RC/revision suffixes.
- GC is fail-closed on any manifest/object integrity ambiguity.
- Active `objects/` entries are never directly deleted by GC; first transition is rename into quarantine.
- Quarantine is fixed at 7 days / 604800 seconds.
- Archive creation and GC apply use the same exclusive store lock.
- Restore/audit/status use shared store locking where applicable.
- Existing v0.4.2 ColdPack and legacy `.fcold.tar.zst` restore compatibility is preserved.
- v0.4.2 verification TXT precreation/failure trapping is preserved.

---

### Task 1: ColdPack inspection and lock primitives

**Files:**
- Modify: `src/coldstore.rs`
- Test: `tests/coldstore.rs`

**Interfaces:**
- Produces: `pub(crate) struct ColdPackManifestInfo { logical_bytes: u64, chunks: Vec<ColdPackObjectRef> }`
- Produces: `pub(crate) struct ColdPackObjectRef { sha256: String, length: u64 }`
- Produces: `pub(crate) fn inspect_coldpack_manifest(path: &Path, verify_store: Option<&Path>) -> Result<ColdPackManifestInfo, String>`
- Produces: `pub(crate) fn object_path_for_hash(store: &Path, hash: &str) -> Result<PathBuf, String>`
- Produces: `pub(crate) fn verify_object_reference(store: &Path, reference: &ColdPackObjectRef) -> Result<u64, String>`
- Produces: `pub(crate) fn lock_store_exclusive(store: &Path) -> Result<File, String>` and `lock_store_shared`.

- [ ] **Step 1: Write lock/inspection tests first**

Add tests that open a ColdPack manifest, assert returned references, acquire an exclusive store lock, and assert a second handle's `try_lock()` reports contention.

- [ ] **Step 2: Run the focused tests and confirm RED**

Run: `cargo test --test coldstore coldpack_manifest_inspection coldpack_store_lock -- --nocapture`
Expected: compile/test failure because inspection/lock APIs do not exist.

- [ ] **Step 3: Implement minimal inspection + lock APIs**

Use existing checksum/schema/object helpers. The lock file is `.coldpack-store/maintenance.lock`, opened read/write/create and rejected if it is a symlink/non-regular file. Use `File::lock()` for exclusive and `File::lock_shared()` for shared.

- [ ] **Step 4: Integrate locks into archive and restore**

`archive_to_coldpack` takes an exclusive lock before object mutation and holds it through source deletion. `restore_coldpack_archive` takes a shared lock around manifest/object verification and restore.

- [ ] **Step 5: Run focused and inherited ColdStore tests**

Run: `cargo test --test coldstore`
Expected: all tests PASS.

### Task 2: Mark/audit/status engine

**Files:**
- Create: `src/coldpack_gc.rs`
- Modify: `src/lib.rs`
- Create: `tests/coldpack_gc.rs`

**Interfaces:**
- Produces: `pub struct ColdPackStoreReport` containing manifest/logical/reference/object/orphan/quarantine/expired counts and bytes.
- Produces: `pub fn coldpack_status(root: &Path, store: &Path, now_secs: u64) -> Result<ColdPackStoreReport, String>`.
- Produces: `pub fn coldpack_audit(root: &Path, store: &Path, now_secs: u64) -> Result<ColdPackStoreReport, String>`.

- [ ] **Step 1: Write status/audit tests first**

Create two archives with distinct content, delete one archive manifest+sidecar, and assert status reports live + orphan objects. Add malformed manifest, corrupt referenced object, and conflicting-hash-length cases that must fail audit.

- [ ] **Step 2: Run GC tests and confirm RED**

Run: `cargo test --test coldpack_gc`
Expected: compile failure because module/APIs do not exist.

- [ ] **Step 3: Implement recursive manifest mark phase**

Scan `root` recursively while skipping `store`; treat every `.fcoldpack` as authoritative. Use `inspect_coldpack_manifest`. Deduplicate references by SHA-256 and reject conflicting lengths.

- [ ] **Step 4: Implement active/quarantine inventory**

Accept only `objects/<2hex>/<64hex>.zst` active object paths and `quarantine/<numeric timestamp>/<2hex>/<64hex>.zst`. Reject symlinks and malformed protected entries. Sum compressed sizes from metadata.

- [ ] **Step 5: Implement status vs audit behavior**

Status validates manifests/checksums/path structure and reports accounting. Audit additionally verifies every referenced active object by decompression/hash/length.

- [ ] **Step 6: Run GC tests**

Run: `cargo test --test coldpack_gc`
Expected: all status/audit tests PASS.

### Task 3: Preview/apply quarantine GC

**Files:**
- Modify: `src/coldpack_gc.rs`
- Modify: `tests/coldpack_gc.rs`

**Interfaces:**
- Produces: `pub fn coldpack_gc_preview(root: &Path, store: &Path, now_secs: u64) -> Result<ColdPackStoreReport, String>`.
- Produces: `pub fn coldpack_gc_apply(root: &Path, store: &Path, now_secs: u64) -> Result<ColdPackGcResult, String>`.
- `ColdPackGcResult` includes report plus quarantined/purged object and byte counts.

- [ ] **Step 1: Write preview/apply/purge tests first**

Assert preview leaves active object paths byte-for-byte unchanged. Assert apply moves orphan objects to `quarantine/<now>/...` and does not move live objects. Create an old quarantine timestamp (`now - 604800`) and assert only expired, still-unreferenced objects are permanently removed.

- [ ] **Step 2: Run focused tests and confirm RED**

Run: `cargo test --test coldpack_gc gc_preview gc_apply gc_purge -- --nocapture`
Expected: failure because mutation APIs are absent.

- [ ] **Step 3: Implement preview**

Take a shared store lock, perform full audit, and return candidates without mutation.

- [ ] **Step 4: Implement apply**

Take one exclusive store lock for the full operation. Perform full audit under that lock, rename active orphans to timestamped quarantine, fsync touched directories, then purge only expired quarantine entries allowed by the fresh live mark set. Never unlink from `objects/`.

- [ ] **Step 5: Run complete GC tests**

Run: `cargo test --test coldpack_gc`
Expected: all tests PASS.

### Task 4: CLI and persistent maintenance

**Files:**
- Modify: `src/main.rs`
- Modify: `forgeclean-organizer.service.in` only if descriptive text needs update
- Create: `tests/regression_v0_5_0.sh`

**Interfaces:**
- Adds commands: `coldpack-status`, `coldpack-audit`, `coldpack-gc --preview`, `coldpack-gc --apply` with optional `--downloads PATH`.
- `watch` invokes `coldpack_gc_apply` no more than once per 86400 seconds.

- [ ] **Step 1: Write static CLI/maintenance regression first**

Require command dispatch/help strings, fixed `QUARANTINE_SECONDS = 7 * 24 * 60 * 60`, shared/exclusive lock calls, and watch maintenance interval `86400`.

- [ ] **Step 2: Run regression and confirm RED**

Run: `./tests/regression_v0_5_0.sh`
Expected: FAIL because commands/module are absent.

- [ ] **Step 3: Implement CLI output**

Status/audit print `FORGECLEAN_COLDPACK_STATUS=PASS` / `FORGECLEAN_COLDPACK_AUDIT=PASS` plus stable key=value metrics. Preview prints candidates and `PURGE_EXECUTED=0`. Apply prints quarantine/purge counters and `FORGECLEAN_COLDPACK_GC=PASS`.

- [ ] **Step 4: Integrate 24-hour watch maintenance**

Track monotonic elapsed time with `Instant`; run one apply after service startup and then at intervals >= 86400 seconds. Log errors to stderr and continue organizing.

- [ ] **Step 5: Run static regression**

Run: `./tests/regression_v0_5_0.sh`
Expected: `FORGECLEAN_V0_5_0_REGRESSION=PASS`.

### Task 5: Host verification, release binding, and packaging

**Files:**
- Modify: `Cargo.toml`, `README.md`, `build-and-verify.sh`, `install-local.sh`, `hit-it-template.sh`
- Modify historical static regressions only where they incorrectly pin the current version instead of retained behavior.
- Create generated release artifacts outside source tree.

**Interfaces:**
- Canonical host outputs use `ForgeClean-v0.5.0-*` names.

- [ ] **Step 1: Bump version and documentation**

Set Cargo/package/scripts/README to 0.5.0. Document mark/sweep, 7-day quarantine, commands, lock safety, automatic daily maintenance, and no guaranteed compression ratio.

- [ ] **Step 2: Extend host build gates**

Add `cargo test --test coldpack_gc`, targeted GC tests, Clippy/full test/release build, and disposable E2E that: creates live + orphan ColdPack chunks, confirms preview non-mutation, applies quarantine, confirms live restore still exact, runs status/audit, and leaves the real Downloads tree untouched.

- [ ] **Step 3: Preserve verification-TXT guarantee**

Update HIT-IT to precreate `ForgeClean-v0.5.0-VERIFY.txt`, trap all exits, record stage failures, run v0.5.0 regression before host build, then install/restart persistent service.

- [ ] **Step 4: Run all static regressions + service validation locally**

Run every `tests/regression*.sh`, `tests/workspace_isolation.sh`, `bash -n` on scripts, and `systemd-analyze --user verify` with a temporary expected binary path.
Expected: all static gates PASS.

- [ ] **Step 5: Package and verify exact ZIP bytes**

Create `ForgeClean-v0.5.0-SOURCE.zip`, `HIT-IT.sh`, `SHA256SUMS.txt`, and `LOCAL-VERIFY.txt`; extract the exact ZIP fresh and rerun static gates. Record that Cargo compile/runtime verification remains host-authoritative if Rust is unavailable in the sandbox.
