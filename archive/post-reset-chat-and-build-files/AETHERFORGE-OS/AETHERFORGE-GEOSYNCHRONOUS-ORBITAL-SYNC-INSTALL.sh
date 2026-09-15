#!/usr/bin/env bash
set -Eeuo pipefail

REPO_FULL="${AETHERFORGE_REPO_FULL:-pattendnd1-png/AETHERFORGE}"
REPO_DIR="${AETHERFORGE_REPO_DIR:-$HOME/Downloads/AETHERFORGE}"
BIN_DIR="$HOME/.local/bin"
UNIT_DIR="$HOME/.config/systemd/user"
STATE_DIR="$HOME/.local/state/aetherforge/orbital-sync"
ORBIT_BIN="$BIN_DIR/aetherforge-orbital-sync"
AUTOSYNC_BIN="$BIN_DIR/aetherforge-git-autosync"
FULL_SWEEP_BIN="$BIN_DIR/aetherforge-postreset-sweep"
STATUS_FILE="$HOME/Downloads/AETHERFORGE-ORBITAL-SYNC-STATUS.txt"

log(){ printf '%s\n' "$*"; }
die(){ printf 'ERROR: %s\n' "$*" >&2; exit 1; }
for c in git gh systemctl flock; do command -v "$c" >/dev/null 2>&1 || die "$c is required"; done
[[ -d "$REPO_DIR/.git" ]] || die "AETHERFORGE repo not found at $REPO_DIR"
[[ -x "$AUTOSYNC_BIN" ]] || die "AutoSync worker missing: $AUTOSYNC_BIN"
[[ -x "$FULL_SWEEP_BIN" ]] || die "Post-reset sweep worker missing: $FULL_SWEEP_BIN"
gh auth status -h github.com >/dev/null 2>&1 || die 'GitHub CLI is not authenticated.'

log 'AETHERFORGE_GEOSYNCHRONOUS_ORBITAL_SYNC=START'
mkdir -p "$BIN_DIR" "$UNIT_DIR" "$STATE_DIR" "$REPO_DIR/os/services/orbital-sync" \
  "$REPO_DIR/docs/superpowers/specs" "$REPO_DIR/docs/superpowers/plans"

# Retire competing schedulers first. Keep their workers as callable components.
systemctl --user disable --now aetherforge-git-autosync.timer >/dev/null 2>&1 || true
systemctl --user stop aetherforge-git-autosync.service >/dev/null 2>&1 || true
systemctl --user disable --now aetherforge-postreset-sweep.timer >/dev/null 2>&1 || true
systemctl --user stop aetherforge-postreset-sweep.service >/dev/null 2>&1 || true
log 'LEGACY_SYNC_TIMERS=RETIRED'

cat > "$ORBIT_BIN" <<'ORBIT_WORKER'
#!/usr/bin/env bash
set -Eeuo pipefail

REPO_FULL="${AETHERFORGE_REPO_FULL:-pattendnd1-png/AETHERFORGE}"
REPO_DIR="${AETHERFORGE_REPO_DIR:-$HOME/Downloads/AETHERFORGE}"
PROFILE_REPO="${AETHERFORGE_PROFILE_REPO:-pattendnd1-png/pattendnd1-png}"
AUTOSYNC_BIN="${AETHERFORGE_AUTOSYNC_BIN:-$HOME/.local/bin/aetherforge-git-autosync}"
FULL_SWEEP_BIN="${AETHERFORGE_FULL_SWEEP_BIN:-$HOME/.local/bin/aetherforge-postreset-sweep}"
STATE_DIR="${AETHERFORGE_ORBITAL_STATE_DIR:-$HOME/.local/state/aetherforge/orbital-sync}"
STATE_FILE="$STATE_DIR/state.env"
STATUS_FILE="${AETHERFORGE_ORBITAL_STATUS_FILE:-$HOME/Downloads/AETHERFORGE-ORBITAL-SYNC-STATUS.txt}"
LOG_FILE="$STATE_DIR/orbital.log"
LOCK_FILE="$STATE_DIR/orbital.lock"
FULL_INTERVAL="${AETHERFORGE_ORBITAL_FULL_INTERVAL:-300}"
RELEASE_TAG="${AETHERFORGE_WIP_RELEASE_TAG:-wip-post-reset}"

mkdir -p "$STATE_DIR" "$(dirname -- "$STATUS_FILE")"

ts() { date --iso-8601=seconds; }
log() { printf '%s %s\n' "$(ts)" "$*" | tee -a "$LOG_FILE"; }

orbit_mode_for() {
  local now="$1" last="${2:-}"
  if [[ -z "$last" || ! "$last" =~ ^[0-9]+$ ]]; then
    printf 'full\n'; return 0
  fi
  if (( now - last >= FULL_INTERVAL )); then printf 'full\n'; else printf 'fast\n'; fi
}

load_state_value() {
  local key="$1"
  [[ -f "$STATE_FILE" ]] || return 0
  awk -F= -v k="$key" '$1==k {sub(/^[^=]*=/,""); print; exit}' "$STATE_FILE" 2>/dev/null || true
}

repo_branch() {
  git -C "$REPO_DIR" symbolic-ref --quiet --short HEAD 2>/dev/null || printf 'main'
}

repo_metrics() {
  local branch remote localh ahead behind dirty
  branch="$(repo_branch)"
  git -C "$REPO_DIR" fetch origin "$branch" --quiet >/dev/null 2>&1 || true
  localh="$(git -C "$REPO_DIR" rev-parse HEAD 2>/dev/null || true)"
  remote="$(git -C "$REPO_DIR" rev-parse "origin/$branch" 2>/dev/null || true)"
  ahead=0; behind=0
  if [[ -n "$localh" && -n "$remote" ]]; then
    read -r behind ahead < <(git -C "$REPO_DIR" rev-list --left-right --count "origin/$branch...HEAD" 2>/dev/null || printf '0 0\n')
  fi
  dirty="$(git -C "$REPO_DIR" status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$branch" "$localh" "$remote" "$ahead" "$behind" "$dirty"
}

release_updated_at() {
  gh release view "$RELEASE_TAG" -R "$REPO_FULL" --json updatedAt --jq '.updatedAt // "unknown"' 2>/dev/null || printf 'unknown'
}

write_status() {
  local mode="$1" result="$2" note="$3" full_success="${4:-}"
  local now epoch branch localh remote ahead behind dirty release_updated old_auto old_sweep timer_state
  now="$(ts)"; epoch="$(date +%s)"
  IFS=$'\t' read -r branch localh remote ahead behind dirty < <(repo_metrics)
  release_updated="$(release_updated_at)"
  old_auto="$(systemctl --user is-enabled aetherforge-git-autosync.timer 2>/dev/null || true)"
  old_sweep="$(systemctl --user is-enabled aetherforge-postreset-sweep.timer 2>/dev/null || true)"
  timer_state="$(systemctl --user is-active aetherforge-orbital-sync.timer 2>/dev/null || true)"
  [[ -n "$full_success" ]] || full_success="$(load_state_value LAST_FULL_SUCCESS_EPOCH)"
  {
    printf 'LAST_ORBIT_AT=%s\n' "$now"
    printf 'LAST_ORBIT_EPOCH=%s\n' "$epoch"
    printf 'LAST_MODE=%s\n' "$mode"
    printf 'LAST_RESULT=%s\n' "$result"
    printf 'LAST_NOTE=%s\n' "${note//$'\n'/ }"
    printf 'LAST_FULL_SUCCESS_EPOCH=%s\n' "$full_success"
    printf 'LOCAL_HEAD=%s\n' "$localh"
    printf 'REMOTE_MAIN=%s\n' "$remote"
    printf 'QUEUE_AHEAD=%s\n' "$ahead"
    printf 'QUEUE_BEHIND=%s\n' "$behind"
    printf 'DIRTY_COUNT=%s\n' "$dirty"
    printf 'RELEASE_UPDATED_AT=%s\n' "$release_updated"
  } > "$STATE_FILE.tmp"
  mv -f "$STATE_FILE.tmp" "$STATE_FILE"
  {
    echo 'AETHERFORGE_ORBITAL_SYNC_STATUS=START'
    printf 'updated_at=%s\n' "$now"
    printf 'mode=%s\nresult=%s\nnote=%s\n' "$mode" "$result" "$note"
    printf 'repo=%s\nbranch=%s\nlocal_head=%s\nremote_main=%s\n' "$REPO_FULL" "$branch" "$localh" "$remote"
    printf 'queue_ahead=%s\nqueue_behind=%s\ndirty_count=%s\n' "$ahead" "$behind" "$dirty"
    printf 'last_full_success_epoch=%s\nrelease_tag=%s\nrelease_updated_at=%s\n' "$full_success" "$RELEASE_TAG" "$release_updated"
    printf 'orbital_timer=%s\nlegacy_autosync_timer=%s\nlegacy_sweep_timer=%s\n' "$timer_state" "$old_auto" "$old_sweep"
    echo 'AETHERFORGE_ORBITAL_SYNC_STATUS=END'
  } > "$STATUS_FILE.tmp"
  mv -f "$STATUS_FILE.tmp" "$STATUS_FILE"
}

operation_in_progress() {
  [[ -d "$REPO_DIR/.git/rebase-merge" || -d "$REPO_DIR/.git/rebase-apply" || -f "$REPO_DIR/.git/MERGE_HEAD" || -f "$REPO_DIR/.git/CHERRY_PICK_HEAD" ]]
}

push_queue_if_possible() {
  local branch localh remote ahead behind dirty
  IFS=$'\t' read -r branch localh remote ahead behind dirty < <(repo_metrics)
  if (( behind > 0 && ahead == 0 && dirty == 0 )); then
    git -C "$REPO_DIR" pull --rebase origin "$branch" >/dev/null 2>&1 || return 2
    IFS=$'\t' read -r branch localh remote ahead behind dirty < <(repo_metrics)
  fi
  if (( ahead > 0 && behind == 0 )); then
    git -C "$REPO_DIR" push origin "HEAD:$branch" >/dev/null 2>&1 || return 3
  elif (( ahead > 0 && behind > 0 )); then
    return 4
  fi
  return 0
}

run_fast() {
  [[ -d "$REPO_DIR/.git" ]] || { write_status fast FAIL 'repo-missing'; return 1; }
  operation_in_progress && { write_status fast BLOCKED 'git-operation-in-progress'; return 0; }

  if push_queue_if_possible; then
    :
  else
    local rc=$?
    case "$rc" in
      3) write_status fast QUEUED_NETWORK 'push-failed-will-retry-next-orbit'; return 0 ;;
      4) write_status fast BLOCKED 'branch-diverged-no-force-push'; return 0 ;;
      *) write_status fast BLOCKED 'pull-rebase-failed'; return 0 ;;
    esac
  fi

  if [[ -n "$(git -C "$REPO_DIR" status --porcelain 2>/dev/null)" ]]; then
    if [[ -x "$AUTOSYNC_BIN" ]]; then
      if ! "$AUTOSYNC_BIN"; then
        write_status fast BLOCKED 'autosync-worker-blocked-or-failed'; return 0
      fi
    else
      write_status fast FAIL 'autosync-worker-missing'; return 1
    fi
  fi

  if push_queue_if_possible; then
    :
  else
    local rc=$?
    case "$rc" in
      3) write_status fast QUEUED_NETWORK 'post-autosync-push-failed-will-retry';;
      4) write_status fast BLOCKED 'post-autosync-branch-diverged';;
      *) write_status fast BLOCKED 'post-autosync-reconcile-failed';;
    esac
    return 0
  fi
  write_status fast PASS 'fast-orbit-clean'
}

run_full() {
  [[ -x "$FULL_SWEEP_BIN" ]] || { write_status full FAIL 'full-sweep-worker-missing'; return 1; }
  log 'FULL_ORBIT=START'
  if "$FULL_SWEEP_BIN" sync; then
    local now
    now="$(date +%s)"
    if ! push_queue_if_possible; then
      write_status full QUEUED_NETWORK 'full-sweep-complete-push-queued' "$now"
      return 0
    fi
    write_status full PASS 'full-scan-import-upload-profile-sync-complete' "$now"
    log 'FULL_ORBIT=PASS'
  else
    write_status full FAIL 'full-sweep-failed-see-migration-diagnostic'
    log 'FULL_ORBIT=FAIL'
    return 1
  fi
}

run_orbit() {
  exec 9>"$LOCK_FILE"
  if ! flock -n 9; then
    write_status skipped SKIP_ALREADY_RUNNING 'shared-orbit-lock-busy'
    return 0
  fi
  local now last mode
  now="$(date +%s)"
  last="$(load_state_value LAST_FULL_SUCCESS_EPOCH)"
  if [[ "${AETHERFORGE_ORBITAL_FORCE_FULL:-0}" == 1 ]]; then mode=full; else mode="$(orbit_mode_for "$now" "$last")"; fi
  log "ORBIT_MODE=$mode"
  if [[ "$mode" == full ]]; then run_full; else run_fast; fi
}

show_status() {
  if [[ -f "$STATUS_FILE" ]]; then cat "$STATUS_FILE"; else echo 'AETHERFORGE_ORBITAL_SYNC_STATUS=NOT_YET_AVAILABLE'; fi
}

if [[ "${AETHERFORGE_ORBITAL_LIB_ONLY:-0}" == 1 ]]; then
  return 0 2>/dev/null || exit 0
fi

case "${1:-orbit}" in
  orbit) run_orbit ;;
  fast) exec 9>"$LOCK_FILE"; flock -n 9 || exit 0; run_fast ;;
  full) exec 9>"$LOCK_FILE"; flock -n 9 || exit 0; run_full ;;
  status) show_status ;;
  *) echo "usage: $0 {orbit|fast|full|status}" >&2; exit 2 ;;
esac
ORBIT_WORKER
chmod +x "$ORBIT_BIN"
cp -f "$ORBIT_BIN" "$REPO_DIR/os/services/orbital-sync/aetherforge-orbital-sync"
chmod +x "$REPO_DIR/os/services/orbital-sync/aetherforge-orbital-sync"

cat > "$UNIT_DIR/aetherforge-orbital-sync.service" <<'EOF_SERVICE'
[Unit]
Description=AETHERFORGE geosynchronous orbital AutoScan + AutoUpload
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=%h/.local/bin/aetherforge-orbital-sync orbit
Nice=10
IOSchedulingClass=best-effort
IOSchedulingPriority=7
EOF_SERVICE

cat > "$UNIT_DIR/aetherforge-orbital-sync.timer" <<'EOF_TIMER'
[Unit]
Description=AETHERFORGE orbital sync — 30 second fast orbit / 5 minute full orbit

[Timer]
OnBootSec=30s
OnUnitActiveSec=30s
AccuracySec=2s
Persistent=true
Unit=aetherforge-orbital-sync.service

[Install]
WantedBy=timers.target
EOF_TIMER

cp -f "$UNIT_DIR/aetherforge-orbital-sync.service" "$REPO_DIR/os/services/orbital-sync/aetherforge-orbital-sync.service"
cp -f "$UNIT_DIR/aetherforge-orbital-sync.timer" "$REPO_DIR/os/services/orbital-sync/aetherforge-orbital-sync.timer"

cat > "$REPO_DIR/docs/superpowers/specs/2026-09-15-geosynchronous-orbital-sync-design.md" <<'EOF_SPEC'
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
EOF_SPEC
cat > "$REPO_DIR/docs/superpowers/plans/2026-09-15-geosynchronous-orbital-sync.md" <<'EOF_PLAN'
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
EOF_PLAN
cat > "$REPO_DIR/os/services/orbital-sync/README.md" <<'EOF_README'
# AETHERFORGE Orbital Sync

Single-authority AutoScan + AutoUpload for the post-reset Garuda-backed AETHERFORGE Rust reforge.

- Fast orbit: every 30 seconds for canonical repo changes and queued pushes.
- Full orbit: every 5 minutes for source rediscovery, OS/app import, artifact indexing/upload, and README/profile progress.
- One shared lock; no overlapping syncs.
- No force pushes.
- Paste-friendly status: `~/Downloads/AETHERFORGE-ORBITAL-SYNC-STATUS.txt`.

The legacy timers are retired as schedulers; their worker binaries are invoked only by the orbital orchestrator.
EOF_README

# Durable ignore for transient orbital state if a developer mirrors state into repo manually.
if ! grep -q '^# AETHERFORGE orbital runtime state$' "$REPO_DIR/.gitignore" 2>/dev/null; then
  cat >> "$REPO_DIR/.gitignore" <<'EOF_IGNORE'

# AETHERFORGE orbital runtime state
*.orbital.tmp
AETHERFORGE-ORBITAL-SYNC-STATUS.txt
EOF_IGNORE
fi

systemctl --user daemon-reload
systemctl --user enable --now aetherforge-orbital-sync.timer >/dev/null
log 'ORBITAL_TIMER=ENABLED'

# First orbit is full: proves scan/import/push/upload/profile chain and commits orbital source/docs.
AETHERFORGE_ORBITAL_FORCE_FULL=1 "$ORBIT_BIN" orbit

# Verification gates.
systemctl --user is-enabled aetherforge-orbital-sync.timer | grep -qx enabled || die 'orbital timer is not enabled'
systemctl --user is-active aetherforge-orbital-sync.timer | grep -Eq '^(active|waiting)$' || die 'orbital timer is not active'
if systemctl --user is-enabled aetherforge-git-autosync.timer >/dev/null 2>&1; then die 'legacy git autosync timer still enabled'; fi
if systemctl --user is-enabled aetherforge-postreset-sweep.timer >/dev/null 2>&1; then die 'legacy post-reset sweep timer still enabled'; fi
[[ -f "$STATUS_FILE" ]] || die 'orbital status file was not created'
grep -Eq '^result=(PASS|QUEUED_NETWORK)$' "$STATUS_FILE" || die 'first orbital pass did not reach PASS/queued network state'

local_head="$(git -C "$REPO_DIR" rev-parse HEAD)"
remote_head="$(git -C "$REPO_DIR" ls-remote origin refs/heads/main | awk '{print $1}')"
if [[ "$local_head" != "$remote_head" ]]; then
  log 'ORBITAL_REMOTE_QUEUE=PENDING'
else
  log "ORBITAL_REMOTE_MAIN=$remote_head"
fi

log 'FAST_ORBIT=30_SECONDS'
log 'FULL_ORBIT=5_MINUTES'
log 'AUTO_SCAN=ENABLED'
log 'AUTO_UPLOAD=ENABLED'
log "STATUS_FILE=$STATUS_FILE"
log 'AETHERFORGE_GEOSYNCHRONOUS_ORBITAL_SYNC=PASS'
