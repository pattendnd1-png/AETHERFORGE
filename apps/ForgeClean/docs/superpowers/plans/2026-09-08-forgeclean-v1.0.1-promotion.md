# ForgeClean v1.0.1 Stable Promotion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Promote the fully host-verified ForgeClean v0.6.4 CLI, persistent organizer, ColdPack engine, and DragonGlass GUI to canonical stable version v1.0.1 without changing runtime behavior.

**Architecture:** Preserve the v0.6.4 Rust core and both binaries exactly in behavior. Change only current-version identity, versioned manifest/report names, installer/verifier/HIT-IT bindings, release documentation, and add a regression that prevents stale v0.6.4 current-version identifiers from leaking into the v1.0.1 release.

**Tech Stack:** Rust 2024, eframe/egui 0.36.2, Bash, systemd --user, Arch/Garuda/AetherForge host tooling.

**Spec:** `README.md` plus the verified v0.6.4 host contract and the user's explicit instruction that the next canonical release is v1.0.1.

## Global Constraints

- Preserve direct-unlink deletion semantics and all existing safety gates.
- Preserve persistent `forgeclean-organizer.service` behavior.
- Preserve build-aware routing and compatibility symlinks.
- Preserve ColdPack CDC/dedup/Zstd/SHA256 behavior and GC quarantine policy.
- Preserve the native DragonGlass GUI at 90% transparent / 10% smoky-glassy.
- Preserve all historical regression scripts and execute them in the v1.0.1 host verifier.
- One canonical artifact set for v1.0.1.

---

### Task 1: Stable-version contract regression

**Files:**
- Create: `tests/regression_v1_0_1_promotion.sh`
- Modify: `build-and-verify.sh`
- Modify: `hit-it-template.sh`

**Interfaces:**
- Consumes: existing v0.6.4 current-version constants and release scripts.
- Produces: `FORGECLEAN_V1_0_1_PROMOTION=PASS` gate.

- [ ] **Step 1: Write the failing regression** asserting Cargo, CLI, GUI, manifests, installer, verifier and docs all identify v1.0.1 while historical regressions remain present.
- [ ] **Step 2: Run it against unmodified v0.6.4 and confirm RED.**
- [ ] **Step 3: Add the new regression to the host verifier and HIT-IT preflight.**
- [ ] **Step 4: Re-run after Task 2 and confirm GREEN.**

### Task 2: Promote current release identity to v1.0.1

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/main.rs`
- Modify: `src/system_scan.rs`
- Modify: `src/gui.rs`
- Modify: `src/gui_state.rs`
- Modify: `src/gui_main.rs`
- Modify: `build-and-verify.sh`
- Modify: `install-local.sh`
- Modify: `hit-it-template.sh`
- Modify: `README.md`

**Interfaces:**
- Consumes: v0.6.4 verified implementation.
- Produces: v1.0.1 binaries, reports, self-tests and installation records with unchanged behavior.

- [ ] **Step 1: Replace current version constants with 1.0.1.**
- [ ] **Step 2: Update versioned Pacman/offload report names.**
- [ ] **Step 3: Update GUI version/self-test and installer/verifier/HIT-IT bindings.**
- [ ] **Step 4: Update README to mark v1.0.1 as stable canonical baseline.**
- [ ] **Step 5: Confirm no current release path still emits v0.6.4.**

### Task 3: Seal and verify canonical artifacts

**Files:**
- Create: `/mnt/data/ForgeClean-v1.0.1-SOURCE.zip`
- Create: `/mnt/data/ForgeClean-v1.0.1-HIT-IT.sh`
- Create: `/mnt/data/ForgeClean-v1.0.1-SHA256SUMS.txt`
- Create: `/mnt/data/ForgeClean-v1.0.1-LOCAL-VERIFY.txt`

**Interfaces:**
- Consumes: completed v1.0.1 source tree.
- Produces: exact host-gate handoff artifacts.

- [ ] **Step 1: Run all shell regressions and shell syntax checks.**
- [ ] **Step 2: Build a normalized ZIP with 0755 directory metadata and executable scripts.**
- [ ] **Step 3: Verify ZIP integrity and exact extracted regression results.**
- [ ] **Step 4: Bind HIT-IT to the exact source SHA256.**
- [ ] **Step 5: Verify early-failure `VERIFY.txt` behavior and checksums.**
- [ ] **Step 6: Report that Rust compile/runtime remain host-authoritative if Cargo is unavailable locally.**
