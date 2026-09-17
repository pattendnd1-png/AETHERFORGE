# OpenDeck+ 2.0.13 — Render/Performance Final Test + Qualifier Closure

Base active release: OpenDeck+ 2.0.8.

2.0.11 was not activated. Its host qualification reached 51/52 frontend tests, then stopped because the dial interaction assignment regression test navigated to `Previous control` instead of assigning the Action Library action `Previous Page`. The same run also exposed stale 2.0.10 top-level qualifier labels. 2.0.13 carries the approved render/performance implementation forward unchanged, restores that test to `Previous Page`, and corrects all release/qualification markers to 2.0.13.

Acceptance is unchanged: frontend/Rust/Tauri/Stream Deck+ probe + visual geometry + screenshot similarity + responsiveness budgets must pass before human visual approval. The active 2.0.8 binary remains untouched until explicit human approval activates the already-qualified 2.0.13 candidate.

Dial Stacks are deferred to 2.0.13.
