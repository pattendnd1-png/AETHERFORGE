# AetherForge Contribution Guidelines

## Core Principles

1. **No Paid AI Dependencies** - Use git, git-cliffs, conventional commits
2. **Verify Before Committing** - Run project verification gates first
3. **Document State Changes** - Update CURRENT_STATE.md with every change
4. **No Secrets in Repo** - Never commit credentials, tokens, or API keys

## Workflow

1. Read the project's `CURRENT_STATE.md`
2. Make the change
3. Run verification gates (`cargo test`, `cargo clippy`)
4. Update `CURRENT_STATE.md` with new state
5. Commit with verification evidence
6. Push to main or create PR branch

## Versioning

Follow semver. Every change advances the version. Never reuse version numbers.

## Testing

- All Rust code must pass `cargo clippy --all-targets -- -D warnings`
- All unit tests must pass `cargo test --release`
- Gate closure scripts must succeed before release

## File Layout

- `apps/<project>/` - Application source
- `projects/<project>/CURRENT_STATE.md` - Canonical handoff state
- `os/<component>/` - OS integration overlays
- `artifacts/<version>/` - Verification checksums
