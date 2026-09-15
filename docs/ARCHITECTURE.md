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
