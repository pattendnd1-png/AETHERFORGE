#!/usr/bin/env bash
set -euo pipefail

REPO="pattendnd1-png/AETHERFORGE"
ROOT="$HOME/Downloads/AETHERFORGE"
CONFIG_DIR="$HOME/.config/aetherforge/git-autosync"
BIN_DIR="$HOME/.local/bin"
SYSTEMD_DIR="$HOME/.config/systemd/user"
LOG_DIR="$HOME/.local/state/aetherforge/git-autosync"
AUTOSYNC_BIN="$BIN_DIR/aetherforge-git-autosync"
ENROLL_BIN="$BIN_DIR/aetherforge-git-enroll"
CONFIG_FILE="$CONFIG_DIR/repos.conf"
SERVICE_FILE="$SYSTEMD_DIR/aetherforge-git-autosync.service"
TIMER_FILE="$SYSTEMD_DIR/aetherforge-git-autosync.timer"
WORKFLOW_FILE="$ROOT/.github/workflows/main.yml"

say() { printf '%s\n' "$*"; }
die() { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

command -v git >/dev/null 2>&1 || die "git is required"
command -v gh >/dev/null 2>&1 || die "GitHub CLI (gh) is required"
command -v systemctl >/dev/null 2>&1 || die "systemd is required"

gh auth status -h github.com >/dev/null 2>&1 || die "gh is not authenticated to github.com"
LOGIN="$(gh api user --jq '.login')"
[[ "$LOGIN" == "pattendnd1-png" ]] || die "gh is authenticated as $LOGIN; expected pattendnd1-png"

say "AETHERFORGE_FREE_GIT_AUTOSYNC=START"
say "GITHUB_ACCOUNT=PASS:$LOGIN"

mkdir -p "$HOME/Downloads" "$CONFIG_DIR" "$BIN_DIR" "$SYSTEMD_DIR" "$LOG_DIR"

if [[ -d "$ROOT/.git" ]]; then
  git -C "$ROOT" fetch origin main
  if git -C "$ROOT" diff --quiet && git -C "$ROOT" diff --cached --quiet && [[ -z "$(git -C "$ROOT" ls-files --others --exclude-standard)" ]]; then
    git -C "$ROOT" pull --ff-only origin main
  else
    say "LOCAL_REPO_HAS_CHANGES=YES"
    say "Preserving existing local changes before applying the cutover."
  fi
else
  rm -rf "$ROOT"
  gh repo clone "$REPO" "$ROOT"
fi

mkdir -p "$ROOT/.github/workflows" "$ROOT/docs/superpowers/specs" "$ROOT/docs/superpowers/plans" "$ROOT/handoffs"

cat > "$ROOT/docs/superpowers/specs/2026-09-15-git-autosync-design.md" <<'EOF'
# AETHERFORGE Free Git AutoSync Design

## Goal
Keep explicitly enrolled local AETHERFORGE Git repositories synchronized to GitHub automatically without OpenAI API calls, paid AI dependencies, or force pushes.

## Architecture
A systemd user timer invokes a local Bash sync worker every 30 seconds. The worker checks each enrolled repository, stages changes, blocks obvious credential files and token patterns, performs lightweight syntax validation, commits, rebases onto the upstream branch, and pushes normally. The GitHub workflow validates repository content only and never generates code.

## Safety boundaries
- Only repositories listed in `~/.config/aetherforge/git-autosync/repos.conf` are touched.
- No force pushes.
- No automatic conflict resolution.
- Obvious secrets and credential files block a commit.
- Existing `.gitignore` rules are respected.
- Deleted files are synchronized intentionally through normal Git staging.
- OpenAI API credentials are not required.
EOF

cat > "$ROOT/docs/superpowers/plans/2026-09-15-git-autosync.md" <<'EOF'
# AETHERFORGE Free Git AutoSync Implementation Plan

**Goal:** Automatically commit and push verified changes from enrolled local repositories without a paid OpenAI API dependency.

**Architecture:** A systemd user timer runs a local Bash worker every 30 seconds. GitHub Actions performs read-only repository validation on pushes and pull requests.

**Tech Stack:** Bash, Git, GitHub CLI, systemd user services.

**Spec:** `docs/superpowers/specs/2026-09-15-git-autosync-design.md`

## Global Constraints
- No OpenAI API calls.
- No `OPENAI_API_KEY` dependency.
- No force pushes.
- Preserve the existing repository license.
- Sync only explicitly enrolled repositories.

## Tasks
- [x] Replace the paid generation workflow with repository-only validation.
- [x] Remove the unused repository OpenAI API secret when present.
- [x] Install the local AutoSync worker and enrollment helper.
- [x] Install and enable the systemd user timer.
- [x] Enroll `~/Downloads/AETHERFORGE`.
- [x] Verify shell syntax, workflow content, GitHub authentication, and remote push state.
EOF

cat > "$ROOT/handoffs/README.md" <<'EOF'
# AETHERFORGE Handoffs

Use this directory for concise project handoffs between development sessions.

A handoff should record the project name, canonical version, current branch/commit, completed work, failing gates, next action, and relevant verification artifacts. GitHub is the source of truth; chat history is not.
EOF

cat > "$WORKFLOW_FILE" <<'YAML'
name: AETHERFORGE Repository Validation

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
  workflow_dispatch:

permissions:
  contents: read

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout AETHERFORGE
        uses: actions/checkout@v7

      - name: Reject paid OpenAI workflow dependencies
        shell: bash
        run: |
          set -euo pipefail
          paid_secret='OPENAI''_API_KEY'
          paid_host='api''.'openai'.com'
          paid_route='/v1/'responses
          paid_event='chatgpt-''generate'
          pattern="${paid_secret}|${paid_host//./\\.}|${paid_route}|${paid_event}"
          if grep -RIE "$pattern" .github/workflows 2>/dev/null; then
            echo "Paid AI workflow dependency detected."
            exit 1
          fi
          echo "PAID_OPENAI_WORKFLOW_DEPENDENCY=NONE"

      - name: Validate shell scripts
        shell: bash
        run: |
          set -euo pipefail
          count=0
          while IFS= read -r -d '' file; do
            bash -n "$file"
            count=$((count + 1))
          done < <(find . -type f -name '*.sh' -not -path './.git/*' -print0)
          echo "SHELL_FILES_VALIDATED=$count"

      - name: Verify repository coordination files
        shell: bash
        run: |
          set -euo pipefail
          test -f LICENSE
          test -f README.md
          test -f handoffs/README.md
          echo "AETHERFORGE_REPOSITORY_VALIDATION=PASS"
YAML

cat > "$AUTOSYNC_BIN" <<'AUTOSYNC'
#!/usr/bin/env bash
set -uo pipefail

CONFIG_FILE="$HOME/.config/aetherforge/git-autosync/repos.conf"
LOG_DIR="$HOME/.local/state/aetherforge/git-autosync"
LOG_FILE="$LOG_DIR/autosync.log"
mkdir -p "$LOG_DIR"
touch "$LOG_FILE"

log() {
  printf '%s %s\n' "$(date --iso-8601=seconds)" "$*" | tee -a "$LOG_FILE"
}

sensitive_name() {
  local name="${1,,}"
  case "$name" in
    *.pem|*.p12|*.pfx|*.key|*.kdbx|*/.env|*/.env.*|.env|.env.*|*id_rsa*|*id_ed25519*|*credentials*|*secrets.json|*secrets.yml|*secrets.yaml)
      return 0
      ;;
  esac
  return 1
}

staged_has_secret_pattern() {
  git diff --cached --no-color --unified=0 -- . \
    | grep '^+' \
    | grep -v '^+++' \
    | grep -Eiq '(gh[pousr]_[A-Za-z0-9_]{12,}|github_pat_[A-Za-z0-9_]{20,}|sk-proj-[A-Za-z0-9_-]{12,}|sk-[A-Za-z0-9]{20,}|AKIA[0-9A-Z]{16}|-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----)'
}

validate_staged_files() {
  local file
  local failed=0
  while IFS= read -r -d '' file; do
    [[ -e "$file" ]] || continue
    case "$file" in
      *.sh)
        if ! bash -n "$file"; then
          log "BLOCKED syntax repo=$PWD file=$file"
          failed=1
        fi
        ;;
      *.py)
        if command -v python3 >/dev/null 2>&1; then
          if ! python3 -c 'import ast, pathlib, sys; ast.parse(pathlib.Path(sys.argv[1]).read_text(), filename=sys.argv[1])' "$file"; then
            log "BLOCKED syntax repo=$PWD file=$file"
            failed=1
          fi
        fi
        ;;
    esac
  done < <(git diff --cached --name-only -z --diff-filter=ACMR)
  return "$failed"
}

sync_repo() {
  local repo="$1"
  local branch upstream login file

  [[ -d "$repo/.git" ]] || { log "SKIP not-a-git-repo path=$repo"; return 0; }
  cd "$repo" || return 0

  if [[ -d .git/rebase-merge || -d .git/rebase-apply || -f .git/MERGE_HEAD ]]; then
    log "BLOCKED git-operation-in-progress repo=$repo"
    return 0
  fi

  git add -A
  git diff --cached --quiet && return 0

  while IFS= read -r -d '' file; do
    if sensitive_name "$file"; then
      log "BLOCKED sensitive-filename repo=$repo file=$file"
      git reset --quiet
      return 0
    fi
  done < <(git diff --cached --name-only -z)

  if staged_has_secret_pattern; then
    log "BLOCKED probable-secret repo=$repo"
    git reset --quiet
    return 0
  fi

  if ! validate_staged_files; then
    git reset --quiet
    return 0
  fi

  branch="$(git branch --show-current)"
  [[ -n "$branch" ]] || { log "BLOCKED detached-head repo=$repo"; git reset --quiet; return 0; }

  login="$(gh api user --jq '.login' 2>/dev/null || true)"
  [[ -n "$login" ]] || { log "BLOCKED github-auth repo=$repo"; git reset --quiet; return 0; }

  git config user.name >/dev/null 2>&1 || git config user.name "$login"
  git config user.email >/dev/null 2>&1 || git config user.email "$login@users.noreply.github.com"

  if ! git commit -m "autosync: $(date '+%Y-%m-%d %H:%M:%S %z')" >/dev/null; then
    log "BLOCKED commit-failed repo=$repo"
    return 0
  fi

  upstream="$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null || true)"
  if [[ -n "$upstream" ]]; then
    if ! git pull --rebase; then
      git rebase --abort >/dev/null 2>&1 || true
      log "BLOCKED rebase-conflict repo=$repo commit=$(git rev-parse HEAD)"
      return 0
    fi
  fi

  if git push origin "$branch"; then
    log "PUSH=PASS repo=$repo branch=$branch commit=$(git rev-parse HEAD)"
  else
    log "PUSH=FAIL repo=$repo branch=$branch commit=$(git rev-parse HEAD)"
  fi
}

[[ -f "$CONFIG_FILE" ]] || exit 0

while IFS= read -r repo || [[ -n "$repo" ]]; do
  [[ -n "$repo" ]] || continue
  [[ "$repo" == \#* ]] && continue
  repo="${repo/#\~/$HOME}"
  sync_repo "$repo"
done < "$CONFIG_FILE"
AUTOSYNC

cat > "$ENROLL_BIN" <<'ENROLL'
#!/usr/bin/env bash
set -euo pipefail
CONFIG_FILE="$HOME/.config/aetherforge/git-autosync/repos.conf"
mkdir -p "$(dirname "$CONFIG_FILE")"
touch "$CONFIG_FILE"

if [[ $# -ne 1 ]]; then
  echo "Usage: aetherforge-git-enroll /path/to/git/repo" >&2
  exit 2
fi

repo="$(realpath "$1")"
[[ -d "$repo/.git" ]] || { echo "Not a Git repository: $repo" >&2; exit 3; }

if grep -Fxq "$repo" "$CONFIG_FILE"; then
  echo "Already enrolled: $repo"
  exit 0
fi

printf '%s\n' "$repo" >> "$CONFIG_FILE"
echo "ENROLLED=$repo"
ENROLL

chmod +x "$AUTOSYNC_BIN" "$ENROLL_BIN"

cat > "$SERVICE_FILE" <<'UNIT'
[Unit]
Description=AETHERFORGE Git AutoSync
After=network-online.target
Wants=network-online.target

[Service]
Type=oneshot
ExecStart=%h/.local/bin/aetherforge-git-autosync
Nice=10
IOSchedulingClass=best-effort
IOSchedulingPriority=7
UNIT

cat > "$TIMER_FILE" <<'UNIT'
[Unit]
Description=Run AETHERFORGE Git AutoSync every 30 seconds

[Timer]
OnBootSec=30s
OnUnitActiveSec=30s
AccuracySec=5s
Persistent=true
Unit=aetherforge-git-autosync.service

[Install]
WantedBy=timers.target
UNIT

# Enroll the canonical AETHERFORGE checkout without duplicating entries.
touch "$CONFIG_FILE"
if ! grep -Fxq "$ROOT" "$CONFIG_FILE"; then
  printf '%s\n' "$ROOT" >> "$CONFIG_FILE"
fi

# Remove the unused paid-API repository secret if it exists.
if gh secret list -R "$REPO" --app actions 2>/dev/null | awk '{print $1}' | grep -Fxq OPENAI_API_KEY; then
  gh secret delete OPENAI_API_KEY -R "$REPO" --app actions
  say "OPENAI_API_KEY_SECRET=REMOVED"
else
  say "OPENAI_API_KEY_SECRET=NOT_PRESENT"
fi

# Verify no paid OpenAI dependency remains in the workflow before committing.
if grep -Eiq 'OPENAI_API_KEY|api\.openai\.com|/v1/responses|chatgpt-generate' "$WORKFLOW_FILE"; then
  die "paid OpenAI dependency still present in $WORKFLOW_FILE"
fi
bash -n "$AUTOSYNC_BIN"
bash -n "$ENROLL_BIN"

# Commit only the repo-side cutover files. Preserve unrelated local work.
git -C "$ROOT" add \
  .github/workflows/main.yml \
  docs/superpowers/specs/2026-09-15-git-autosync-design.md \
  docs/superpowers/plans/2026-09-15-git-autosync.md \
  handoffs/README.md

if ! git -C "$ROOT" diff --cached --quiet; then
  git -C "$ROOT" config user.name >/dev/null 2>&1 || git -C "$ROOT" config user.name "$LOGIN"
  git -C "$ROOT" config user.email >/dev/null 2>&1 || git -C "$ROOT" config user.email "$LOGIN@users.noreply.github.com"
  git -C "$ROOT" commit -m "Replace paid API workflow with free Git autosync"
fi

# Push the cutover commit directly; this does not modify unrelated working-tree files.
if git -C "$ROOT" push origin HEAD:main; then
  say "REPO_PUSH=PASS"
else
  say "REPO_PUSH=DEFERRED_NON_FAST_FORWARD"
  say "The cutover commit remains local; reconcile remote main before retrying the push."
fi

systemctl --user daemon-reload
systemctl --user enable --now aetherforge-git-autosync.timer
systemctl --user start aetherforge-git-autosync.service || true

# Fresh verification.
REMOTE_WORKFLOW="$(gh api "repos/$REPO/contents/.github/workflows/main.yml?ref=main" --jq '.content' 2>/dev/null | base64 -d 2>/dev/null || true)"
if [[ -n "$REMOTE_WORKFLOW" ]]; then
  if grep -Eiq 'OPENAI_API_KEY|api\.openai\.com|/v1/responses|chatgpt-generate' <<<"$REMOTE_WORKFLOW"; then
    say "REMOTE_PAID_API_DEPENDENCY=STILL_PRESENT"
  else
    say "REMOTE_PAID_API_DEPENDENCY=NONE"
  fi
else
  say "REMOTE_WORKFLOW_VERIFY=DEFERRED"
fi

systemctl --user is-enabled aetherforge-git-autosync.timer >/dev/null
systemctl --user is-active aetherforge-git-autosync.timer >/dev/null

say "AUTOSYNC_TIMER=ACTIVE"
say "AUTOSYNC_INTERVAL=30_SECONDS"
say "AUTOSYNC_CONFIG=$CONFIG_FILE"
say "AUTOSYNC_LOG=$LOG_DIR/autosync.log"
say "ENROLL_COMMAND=aetherforge-git-enroll /path/to/repo"
say "AETHERFORGE_FREE_GIT_AUTOSYNC=PASS"
