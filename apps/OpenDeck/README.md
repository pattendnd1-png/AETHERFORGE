# OpenDeck+ 2.0.18 — Candidate-Bound Screenshot Closure

2.0.17 fixed the screenshot-before-tests order, but its capture helper still used Spectacle active-window mode without proving that the newly launched candidate owned focus. That allowed an older OpenDeck window to be captured under a newer qualification filename.

2.0.18 binds qualification evidence to the candidate itself. The frontend requests focus through a Tauri qualification command, Rust writes an atomic focus ACK containing release `2.0.18` and the candidate PID, the host validates both the focus ACK and visual-metrics release before capture, and the capture helper optionally reinforces focus with `kdotool`/`xdotool` when available.

As a final stale-window defense, qualification mode renders two tiny version-specific capture identity tokens at opposite corners. The normalized PNG must contain both tokens or the screenshot is rejected. These tokens never render in normal OpenDeck use and are negligible for the broad visual-similarity gate.

The screenshot still occurs before frontend tests/lint and later Rust/Tauri/hardware/performance gates. The active baseline remains 2.0.8 until every gate and human visual approval passes. Dial Stacks are deferred to 2.0.19.
