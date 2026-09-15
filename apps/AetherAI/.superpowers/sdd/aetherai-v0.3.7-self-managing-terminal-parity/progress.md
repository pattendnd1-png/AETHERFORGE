# SDD ledger — plan: docs/superpowers/plans/2026-09-05-aetherai-v0.3.7-self-managing-terminal-parity-implementation-plan.md

Execution mode: inline execution with independent review gates.

Ruling: Workspace-root Rust integration tests from the plan are moved to apps/aetherai-desktop/tests/ because the AetherAI root Cargo.toml is a virtual workspace manifest. The test semantics are unchanged; only the runnable Cargo package location is corrected.

Task 1: in_progress
Task 1: complete
Review 1: donor isolation, native ChatGPT Library, zero-cost/no-browser regression gates PASS
Task 2: in_progress
Task 2: complete
Review 2: exact verified Terminal contract constants are centralized in aetherforge-ui; no AetherAI renderer migration is claimed yet.
Task 3: preflight_in_progress
Task 3: preflight_complete
Ruling: do not guess renderer dependency/API versions. Reuse exact live AetherForge Terminal source/Cargo.lock authority when present; otherwise inspect the preflight before implementing the renderer.
