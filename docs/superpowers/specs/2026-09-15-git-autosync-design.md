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
