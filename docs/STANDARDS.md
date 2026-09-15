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
