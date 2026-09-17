# OpenDeck+ 2.0.36 — Event-Loop Startup Visibility Closure

OpenDeck+ 2.0.36 is the bounded correction after 2.0.35 passed its build, frontend, Rust, Tauri, Stream Deck+, and visual gates but failed the real menu-launch gate. The 2.0.35 process exited from the Tauri setup hook because it called `window.is_visible()` immediately after `window.show()` before the application event loop had a chance to present the window.

This release keeps the 2.0.35 Tauri window permissions, Action Wheel, Dial Stacks, and qualified UI unchanged. Normal startup still requests `show()` and `maximize()` natively, but it no longer treats pre-event-loop visibility as a fatal condition. The frontend then performs its permitted window show/maximize calls after WebView bootstrap and invokes `startup_visible_ack`; only that post-bootstrap Tauri visibility check can satisfy the menu-launch probe. Qualification mode remains intentionally hidden until its qualification harness explicitly focuses the window.

The host qualifier runs the event-loop startup contract first, then the inherited capability, launcher, frontend, strict Clippy, Rust, Tauri, Stream Deck+, visual, and real desktop-ID menu-launch gates. Activation remains last, so the current qualified OpenDeck installation is preserved unless every 2.0.36 gate passes.
