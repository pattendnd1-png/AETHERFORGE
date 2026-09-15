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
