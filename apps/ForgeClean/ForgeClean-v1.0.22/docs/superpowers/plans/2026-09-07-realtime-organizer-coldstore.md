# ForgeClean Realtime Organizer + ColdStore Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add automatic Downloads sorting, project activation/build redirection, guarded legacy symlinks, and restorable compressed cold archives under `~/Downloads/ForgeClean`.

**Architecture:** Keep existing cleanup/offload modules unchanged. Add `organizer` for classification/layout and stable-file moves, `registry` for canonical project Active paths and legacy aliases, and `coldstore` for tar+zstd archives with SHA-256 verification and restore. CLI commands expose one-shot organization, continuous watch, activate/build/resolve, archive, and restore.

**Tech Stack:** Rust 2024, std filesystem/process APIs, sha2, tar, zstd, Linux symlinks/systemd user service.

**Spec:** Conversation-approved ForgeClean v0.3.0 design: canonical `~/Downloads/ForgeClean`, project Active/Downloads/Releases/Restored/Cold folders, restores into sorted Restored folders, compatibility symlinks, build-manager redirection, active sources never compressed.

## Global Constraints

- Existing no-external=AUTO_CLEAN and external=AUTO_OFFLOAD behavior remains unchanged.
- Never archive or move a file that is still changing.
- Never cold-compress an active project build tree.
- Cold archive source deletion happens only after archive close, SHA-256 creation, and archive verification.
- Restore never overwrites an existing destination.
- Legacy symlink creation never overwrites a real file/directory or escapes the user's Downloads tree.
- Organizer never recursively processes `~/Downloads/ForgeClean` itself.
- Version is exactly `0.3.0`.

---

### Task 1: Canonical layout and classification
**Files:** Create `src/organizer.rs`; Test `tests/organizer.rs`; Modify `src/lib.rs`.
**Interfaces:** Produces `ForgeLayout`, `Category`, `classify_download`, `project_name_from_entry`, `organize_once`.
- [ ] Write tests for category/project classification and self-root exclusion.
- [ ] Verify RED because module/API does not exist.
- [ ] Implement deterministic paths and stable-entry organization.
- [ ] Verify GREEN on host with `cargo test --test organizer`.

### Task 2: Project registry and compatibility aliases
**Files:** Create `src/registry.rs`; Test `tests/registry.rs`.
**Interfaces:** Produces `ProjectRegistry`, `ProjectRecord`, `activate_project`, `resolve_project`, `create_legacy_alias`.
- [ ] Write tests for registry round-trip, Active resolution, alias refusal on real paths, and alias target containment.
- [ ] Verify RED.
- [ ] Implement TSV registry and guarded symlink creation.
- [ ] Verify GREEN on host.

### Task 3: ColdStore archive/restore
**Files:** Create `src/coldstore.rs`; Test `tests/coldstore.rs`; Modify `Cargo.toml`.
**Interfaces:** Produces `archive_to_cold`, `restore_cold_archive`, `ColdArchiveRecord`.
- [ ] Write tests for archive→delete→restore byte equality, no overwrite, and active-tree exclusion.
- [ ] Verify RED.
- [ ] Add `tar`/`zstd`; implement tar.zst archive with SHA-256 sidecar and verification before source removal.
- [ ] Verify GREEN on host.

### Task 4: CLI/watch/build redirect
**Files:** Modify `src/main.rs`; Test `tests/regression_v0_3_0.sh`.
**Interfaces:** Commands `organize-once`, `watch`, `activate`, `resolve-project`, `build`, `archive`, `restore`.
- [ ] Add static regression requiring commands/version/safety markers; verify RED.
- [ ] Implement commands; watcher polls Downloads and only handles stable entries.
- [ ] Build command resolves registry Active path and executes command there.
- [ ] Verify regression GREEN.

### Task 5: Auto-start installer and host verification
**Files:** Modify `install-local.sh`, `build-and-verify.sh`, README; create `forgeclean-organizer.service.in`.
**Interfaces:** Installer writes/enables user service executing `forgeclean watch`.
- [ ] Add verification gates for new tests and non-destructive organizer E2E fixture.
- [ ] Ensure build verifier never reorganizes the user's real Downloads.
- [ ] Package canonical v0.3.0 artifacts and checksums.
