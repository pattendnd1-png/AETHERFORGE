# ForgeClean v0.2.0 Conditional Offload Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add conditional external-storage detection and checksum-verified offload while preserving cleanup-only behavior when no external drive exists.

**Architecture:** Extend the existing manifest-gated package-cache engine with a storage detector and an offload transaction module. `auto` scans and verifies once, then selects either existing permanent purge or offload based on detected mounted external storage.

**Tech Stack:** Rust 2024, standard library, `sha2` crate, Linux `lsblk`, existing Pacman/vercmp integration.

**Spec:** `docs/superpowers/specs/2026-09-07-forgeclean-v0.2.0-offload-design.md`

## Global Constraints
- Version is exactly `0.2.0`.
- No external storage detected means cleanup-only mode.
- External storage detected means offload mode for approved package-cache candidates only.
- Source deletion after offload requires SHA-256 equality, destination fsync, and source identity revalidation.
- Never overwrite a conflicting destination file and never follow destination symlinks.
- Existing direct-unlink/no-Trash behavior and manifest guards remain intact.

---

### Task 1: Storage detection
**Files:** Create `src/storage.rs`; create `tests/storage.rs`; modify `src/lib.rs`.
**Interfaces:** Produce `ExternalDrive`, `AutoMode`, `parse_lsblk_pairs`, `detect_external_drives`, and `select_auto_mode`.
- [ ] Write tests for USB/removable parent detection, internal-disk rejection, and no-drive clean-only mode.
- [ ] Run the storage tests and confirm RED because `storage` does not exist.
- [ ] Implement the parser/detector and deterministic candidate selection.
- [ ] Run the storage tests and confirm GREEN.

### Task 2: Verified offload transaction
**Files:** Create `src/offload.rs`; create `tests/offload.rs`; modify `src/lib.rs`, `src/purge.rs`, `Cargo.toml`.
**Interfaces:** Produce `OffloadReport` and `offload_batch(&CleanupBatch, &Path) -> OffloadReport`; expose source-entry validation from purge safely.
- [ ] Write tests proving successful copy+delete, existing conflicting destination preservation, and changed-source preservation.
- [ ] Run offload tests and confirm RED because `offload` does not exist.
- [ ] Add `sha2`, implement transactional copy/hash/fsync/revalidate/delete.
- [ ] Run offload and legacy purge tests and confirm GREEN.

### Task 3: Conditional auto command
**Files:** Modify `src/main.rs`, `src/system_scan.rs`; create `tests/regression_v0_2_0.sh`.
**Interfaces:** Add `storage-status` and `auto --yes` without changing existing commands.
- [ ] Add static regression assertions for version, mode labels, commands, and v0.2.0 manifest filename.
- [ ] Run the regression against the pre-v0.2.0 tree and confirm RED.
- [ ] Implement mode selection and one-shot automatic execution.
- [ ] Run regression and CLI/unit tests and confirm GREEN.

### Task 4: Release verification and packaging
**Files:** Modify `build-and-verify.sh`, `install-local.sh`, `README.md`; create release ZIP/HIT-IT/checksum/local verify files.
**Interfaces:** Host verification emits `FORGECLEAN_STORAGE_TEST`, `FORGECLEAN_OFFLOAD_TEST`, `FORGECLEAN_AUTO_CLEAN_E2E`, `FORGECLEAN_AUTO_OFFLOAD_E2E`, and final `FORGECLEAN_VERIFY`.
- [ ] Extend host verification with disposable cleanup/offload tests only; never touch the host Pacman cache destructively.
- [ ] Run static artifact/version/shell checks locally.
- [ ] Package canonical v0.2.0 source and launcher with SHA256.
