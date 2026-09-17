# OpenDeck+ 2.0.33 — Action Wheel Frontend Test Closure

OpenDeck+ 2.0.33 is the correction release for the 2.0.32 Action Wheel candidate. The 2.0.32 host run proved the Action Wheel source/model contracts and 81 of 82 frontend tests, but one legacy PropertyInspector test selected Dial 2 even though Dial 2 is now intentionally the Action Wheel qualification dial.

The production Action Wheel implementation is unchanged. This release fixes the stale regression test to use Dial 3, which is actually a plain dial, and adds a source-level closure gate so the non-container interaction-rail test cannot silently target the Action Wheel or Dial Stack qualification controls again.

The full Action Wheel behavior remains: rotate to select, dial press or matching touch-strip tap to execute, persistent selected entry, add/rename/reorder/remove editing, fresh entry IDs on copy/profile duplication, physical status rendering, Rust persistence/validation, and mutual exclusion with Dial Stacks on the same dial.

The one-shot qualifier runs the frontend-test closure gate first, then the Action Wheel and Dial Stack regression contracts, frontend tests/lint/build, Rust fmt/check/strict Clippy/tests/release, Tauri production build, Stream Deck+ OS/HID probe, visual geometry, and only then activation. No background service or autostart entry is added.
