# AETHERFORGE Geosynchronous Orbital Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace competing AETHERFORGE sync timers with one persistent 30-second orbital scheduler that performs a full discovery/upload orbit every 5 minutes.

**Architecture:** A single systemd timer invokes `aetherforge-orbital-sync orbit`. The worker chooses fast or full mode from persistent state, serializes both through one lock, reuses the proven AutoSync and post-reset sweep workers, and publishes a paste-friendly status file.

**Tech Stack:** Bash, git, GitHub CLI, systemd user units, flock.

**Spec:** `docs/superpowers/specs/2026-09-15-geosynchronous-orbital-sync-design.md`

## Global Constraints
- Fast orbit cadence: 30 seconds.
- Full orbit cadence: 300 seconds.
- No force pushes.
- No paid OpenAI API dependency.
- Garuda/Arch remains the post-reset OS-tree backbone.
- Existing secret and large-file gates remain enabled.

---

### Task 1: Orbital orchestrator
**Files:** Create `os/services/orbital-sync/aetherforge-orbital-sync`.
- [ ] Test mode selection at 299/300 second boundaries.
- [ ] Implement shared lock, fast/full dispatch, retry queue, heartbeat, and status output.
- [ ] Verify `bash -n` and scheduling tests.

### Task 2: Single scheduler cutover
**Files:** Create `os/services/orbital-sync/aetherforge-orbital-sync.service` and `.timer`.
- [ ] Disable the legacy AutoSync and post-reset sweep timers.
- [ ] Install the persistent 30-second orbital timer.
- [ ] Verify only the orbital timer is the synchronization scheduler.

### Task 3: Repository documentation and first orbit
**Files:** Create this design, this plan, and `os/services/orbital-sync/README.md`.
- [ ] Force one full orbit after installation.
- [ ] Verify local and remote `main` converge.
- [ ] Verify `~/Downloads/AETHERFORGE-ORBITAL-SYNC-STATUS.txt` reports PASS.
- [ ] Verify project/profile progress is refreshed by the full sweep.
