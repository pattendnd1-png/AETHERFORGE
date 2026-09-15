#!/usr/bin/env bash
set -euo pipefail

REPO="pattendnd1-png/AETHERFORGE"
ROOT="$HOME/Downloads/AETHERFORGE"
COMMIT_MESSAGE="Initialize AETHERFORGE canonical project backbone"

command -v gh >/dev/null || { echo "ERROR: gh is not installed"; exit 1; }
command -v git >/dev/null || { echo "ERROR: git is not installed"; exit 1; }

gh auth status -h github.com >/dev/null

if [[ -d "$ROOT/.git" ]]; then
  cd "$ROOT"
  git fetch origin main
  git checkout main
  git pull --ff-only origin main
else
  rm -rf "$ROOT"
  gh repo clone "$REPO" "$ROOT"
  cd "$ROOT"
  git checkout main
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "ERROR: $ROOT has uncommitted changes. Commit/stash them first; nothing was modified."
  git status --short
  exit 2
fi

mkdir -p docs handoffs projects .github/workflows

cat > PROJECTS.md <<'EOF'
# AETHERFORGE Project Registry

This repository is the canonical coordination backbone for AETHERFORGE work.
Each project keeps its current handoff state in `projects/<project>/CURRENT_STATE.md`.

| Project | Canonical state | Import status |
| --- | --- | --- |
| AetherForge Desktop | `projects/AetherForge-Desktop/CURRENT_STATE.md` | Pending state import |
| AetherForge Control Center | `projects/AetherForge-Control-Center/CURRENT_STATE.md` | Pending state import |
| AetherBrowser | `projects/AetherBrowser/CURRENT_STATE.md` | Pending state import |
| OpenDeck | `projects/OpenDeck/CURRENT_STATE.md` | Pending state import |
| ForgeHX | `projects/ForgeHX/CURRENT_STATE.md` | Pending state import |
| ForgeKonsole | `projects/ForgeKonsole/CURRENT_STATE.md` | Pending state import |
| AetherAI | `projects/AetherAI/CURRENT_STATE.md` | Pending state import |
| AetherStream | `projects/AetherStream/CURRENT_STATE.md` | Pending state import |
| Behringer Control | `projects/Behringer-Control/CURRENT_STATE.md` | Pending state import |
| OpenSanctuary | `projects/OpenSanctuary/CURRENT_STATE.md` | Pending state import |
| STO Launcher | `projects/STO-Launcher/CURRENT_STATE.md` | Pending state import |
| ReForge | `projects/ReForge/CURRENT_STATE.md` | Pending state import |
| Darkstone-RS | `projects/Darkstone-RS/CURRENT_STATE.md` | Pending state import |
| Stellar Online | `projects/Stellar-Online/CURRENT_STATE.md` | Pending state import |
| AetherForge Mobile | `projects/AetherForge-Mobile/CURRENT_STATE.md` | Pending state import |

## Rule

A chat or workspace should read the relevant `CURRENT_STATE.md` before changing a project and update that file with the same commit as the project change whenever practical.
EOF

cat > docs/ARCHITECTURE.md <<'EOF'
# AETHERFORGE Repository Architecture

## Purpose

GitHub is the shared source of truth between development sessions. Chat history may provide context, but repository state is authoritative for project status, handoffs, policies, verification records, and source that has been committed.

## Coordination layout

- `PROJECTS.md` — project registry.
- `projects/<project>/CURRENT_STATE.md` — canonical current handoff for one project.
- `handoffs/` — handoff format and cross-project notes.
- `docs/` — global architecture, release, and development standards.
- `.github/workflows/` — repository validation only; no paid AI/API dependency is required.

## State flow

1. Read the project's `CURRENT_STATE.md`.
2. Make the project change.
3. Run the project's verification gates.
4. Commit source, verification evidence, and updated current state together when practical.
5. Push to `main` or use a reviewed branch/PR when the change needs isolation.

## Secrets

Never commit API keys, access tokens, passwords, credentials, or private keys. GitHub Actions secrets may exist for other purposes, but the canonical-state workflow does not require an OpenAI API key.
EOF

cat > docs/VERSIONING.md <<'EOF'
# AETHERFORGE Versioning Policy

- Every released change, hotfix, or feature advances the canonical version.
- Do not reuse a released version number.
- Prefer one authoritative artifact for each canonical version.
- Record the current version and next intended version in the project's `CURRENT_STATE.md`.
- Verification/checksum artifacts should identify the version they verify.
- Release notes should include features, fixes, quality-of-life changes, verification, migrations, cleanup/removals, packaging changes, and rollback/status information when applicable.
EOF

cat > docs/STANDARDS.md <<'EOF'
# AETHERFORGE Repository Standards

## Canonical state

Repository content is the durable handoff layer between chats and workspaces. Do not assume another session can reconstruct uncommitted work from chat history alone.

## No paid AI dependency

Normal AETHERFORGE synchronization uses Git, GitHub CLI, commits, and repository files. The default GitHub workflow performs validation only and does not call OpenAI or any other paid model API.

## Safety

- Never commit credentials or tokens.
- Preserve the repository license unless a deliberate licensing change is explicitly approved.
- Do not overwrite a project's existing `CURRENT_STATE.md` during bootstrap/import.
- Keep verification evidence close to the source/version it validates.

## Handoffs

Every current-state file should make the next session able to answer: what version is current, what works, what is broken, what changed most recently, what must not regress, how to verify it, and what should happen next.
EOF

cat > handoffs/README.md <<'EOF'
# AETHERFORGE Handoff Format

Use `projects/<project>/CURRENT_STATE.md` as the active handoff for a project.

Recommended sections:

- Current canonical version
- Status
- Last verified commit
- Working functionality
- Known failures / blockers
- Non-regression requirements
- Verification commands / artifacts
- Next work
- Recent decisions

Update facts, not chat transcripts. Keep enough detail for a new development session to continue without guessing.
EOF

projects=(
  "AetherForge-Desktop"
  "AetherForge-Control-Center"
  "AetherBrowser"
  "OpenDeck"
  "ForgeHX"
  "ForgeKonsole"
  "AetherAI"
  "AetherStream"
  "Behringer-Control"
  "OpenSanctuary"
  "STO-Launcher"
  "ReForge"
  "Darkstone-RS"
  "Stellar-Online"
  "AetherForge-Mobile"
)

for project in "${projects[@]}"; do
  mkdir -p "projects/$project"
  state="projects/$project/CURRENT_STATE.md"
  if [[ ! -e "$state" ]]; then
    cat > "$state" <<EOF
# $project — Current State

- Current canonical version: TBD during state import
- Status: IMPORT PENDING
- Last verified commit: TBD

## Working functionality

State not imported yet.

## Known failures / blockers

State not imported yet.

## Non-regression requirements

State not imported yet.

## Verification

State not imported yet.

## Next work

Import the latest verified project state from the relevant development session/artifacts, then replace all TBD/import-pending entries with repository-backed facts.

## Recent decisions

Canonical state file created during AETHERFORGE repository bootstrap.
EOF
  fi
done

cat > .github/workflows/main.yml <<'YAML'
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

      - name: Validate canonical backbone
        shell: bash
        run: |
          set -euo pipefail
          required=(
            PROJECTS.md
            docs/ARCHITECTURE.md
            docs/VERSIONING.md
            docs/STANDARDS.md
            handoffs/README.md
          )
          for file in "${required[@]}"; do
            [[ -s "$file" ]] || { echo "Missing required file: $file"; exit 1; }
          done
          for dir in projects/*; do
            [[ -d "$dir" ]] || continue
            [[ -s "$dir/CURRENT_STATE.md" ]] || {
              echo "Missing CURRENT_STATE.md: $dir"
              exit 1
            }
          done
          echo "CANONICAL_BACKBONE=PASS"

      - name: Validate tracked Bash syntax
        shell: bash
        run: |
          set -euo pipefail
          count=0
          while IFS= read -r -d '' file; do
            bash -n "$file"
            count=$((count + 1))
          done < <(git ls-files -z '*.sh')
          echo "BASH_SYNTAX_FILES=$count"
          echo "BASH_SYNTAX=PASS"

      - name: Reject committed OpenAI API dependency in default workflow
        shell: bash
        run: |
          set -euo pipefail
          if grep -Eq 'OPENAI_API_KEY|api\.openai\.com' .github/workflows/main.yml; then
            echo "Paid OpenAI API dependency found in default workflow."
            exit 1
          fi
          echo "NO_PAID_AI_WORKFLOW=PASS"
YAML

# Local pre-commit verification.
for required in \
  PROJECTS.md \
  docs/ARCHITECTURE.md \
  docs/VERSIONING.md \
  docs/STANDARDS.md \
  handoffs/README.md; do
  [[ -s "$required" ]] || { echo "ERROR: Missing $required"; exit 3; }
done

for dir in projects/*; do
  [[ -d "$dir" ]] || continue
  [[ -s "$dir/CURRENT_STATE.md" ]] || { echo "ERROR: Missing $dir/CURRENT_STATE.md"; exit 3; }
done

if grep -Eq 'OPENAI_API_KEY|api\.openai\.com' .github/workflows/main.yml; then
  echo "ERROR: paid OpenAI API dependency remains in workflow"
  exit 4
fi

while IFS= read -r -d '' file; do
  bash -n "$file"
done < <(git ls-files -z '*.sh')

git add PROJECTS.md docs handoffs projects .github/workflows/main.yml

if git diff --cached --quiet; then
  echo "AETHERFORGE_BACKBONE=NO_CHANGES"
  exit 0
fi

echo "---- STAGED CHANGES ----"
git diff --cached --stat

git commit -m "$COMMIT_MESSAGE"
git push origin main

LOCAL_SHA="$(git rev-parse HEAD)"
REMOTE_SHA="$(git ls-remote origin refs/heads/main | awk '{print $1}')"
[[ "$LOCAL_SHA" == "$REMOTE_SHA" ]] || {
  echo "ERROR: remote main does not match local HEAD"
  echo "LOCAL_SHA=$LOCAL_SHA"
  echo "REMOTE_SHA=$REMOTE_SHA"
  exit 5
}

# Verify canonical files through GitHub itself.
gh api "repos/$REPO/contents/PROJECTS.md?ref=main" --jq '.sha' >/dev/null
gh api "repos/$REPO/contents/docs/ARCHITECTURE.md?ref=main" --jq '.sha' >/dev/null
gh api "repos/$REPO/contents/.github/workflows/main.yml?ref=main" --jq '.sha' >/dev/null

echo "AETHERFORGE_BACKBONE=PASS"
echo "COMMIT_SHA=$LOCAL_SHA"
echo "PAID_OPENAI_WORKFLOW=DISABLED"
echo "OPENAI_API_KEY_SECRET=UNUSED_BY_DEFAULT_WORKFLOW"
