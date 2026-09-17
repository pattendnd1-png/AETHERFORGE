# OpenDeck+ 2.0.31 — Dial Stacks Frontend-Test Closure

OpenDeck+ 2.0.31 carries forward the 2.0.30 Dial Stacks feature set on top of the exact 2.0.29 donor, with a narrow qualification/test closure after the 2.0.30 host run stopped at `FRONTEND_TESTS`.

## 2.0.31 correction

The 2.0.30 production Dial Stack behavior was not the failing gate. Two frontend test files were stale against the new UI contract:

- `DialStackEditor.test.tsx` searched editable stack labels as normal text nodes even though the labels are rendered as input values.
- `PropertyInspector.test.tsx` used qualification Dial 1, which is intentionally a Dial Stack in 2.0.30+, while still expecting the legacy non-stack Press interaction and `Assigned Action` heading.

2.0.31 fixes those tests, adds explicit stacked/non-stacked Property Inspector coverage, and supplies every required Dial Stack callback in direct component renders so the subsequent TypeScript build can type-check the tests and component contract consistently.

## Preserved Dial Stack behavior

- Dial Stack is available from the Dials action library.
- Dial press cycles the active stack entry.
- Rotate and press+rotate execute bindings from the active entry.
- Stack entries can be added, selected, renamed, reordered, and removed.
- The active entry is reflected on the editor dial and Stream Deck+ touch display.
- Stack definition and active position persist in workspace/profile JSON.
- Existing non-stack dial behavior remains unchanged.
- No background service or autostart entry is introduced.

## Host qualification

The one-shot host qualifier checks the 2.0.31 frontend-test closure first, then the Dial Stack/layout contracts and inherited dial/Twitch regressions. It reuses the existing dependency tree when available and runs frontend tests/lint/build, Rust format/check/strict Clippy/tests/release build, Tauri production build, Stream Deck+ OS/HID probe, and production-candidate visual geometry before activation.

The active OpenDeck binary is not switched until every gate passes. The previous active target and SHA256 are written to `~/Downloads/OpenDeck-v2.0.31-ROLLBACK.txt` for rollback.
