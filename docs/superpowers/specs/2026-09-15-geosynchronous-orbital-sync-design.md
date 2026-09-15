# AETHERFORGE Geosynchronous Orbital Sync Design

**Status:** Approved 2026-09-15

## Goal
Keep the post-reset Garuda-backed AETHERFORGE Rust reforge continuously synchronized to GitHub without competing timers, manual uploads, force pushes, or paid API calls.

## Architecture
One `aetherforge-orbital-sync` orchestrator is the synchronization authority. A single persistent systemd user timer fires every 30 seconds. The orchestrator chooses a fast orbit unless the last successful full orbit is at least 300 seconds old. Both modes use one shared `flock`.

### Fast orbit — 30 seconds
- Flush an already-committed local-ahead queue before making new commits.
- Run the existing safety-aware Git AutoSync worker when the canonical monorepo is dirty.
- Retry ordinary network push failures on the next orbit.
- Never force-push; divergence is reported as BLOCKED.

### Full orbit — every 5 minutes
- Run the post-reset Garuda source/artifact sweep in `sync` mode.
- Rediscover current post-reset OS/app source trees.
- Refresh the Garuda base and AETHERFORGE overlay capture.
- Re-index artifacts and externalize large files according to the existing migration policy.
- Commit/push source changes, upload new/changed WIP prerelease assets, and update project/profile README status.

## State and observability
The orchestrator writes persistent state under `~/.local/state/aetherforge/orbital-sync/` and a paste-friendly `~/Downloads/AETHERFORGE-ORBITAL-SYNC-STATUS.txt` containing mode, result, local/remote HEAD, queue depth, dirty count, release timestamp, and timer state.

## Cutover
The old `aetherforge-git-autosync.timer` and `aetherforge-postreset-sweep.timer` are disabled. Their worker binaries remain reusable implementation components, invoked only by the orbital orchestrator. This removes scheduler races while preserving their safety logic.

## Safety
No force pushes. Existing secret scanning, source-age/reset boundaries, large-file externalization, and WIP prerelease policy remain authoritative. Git operations in progress block fast sync. Network push failures stay queued for later retry.
