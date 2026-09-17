# OpenDeck+ 2.0.21 — Adaptive Unified Touch Strip

OpenDeck+ 2.0.21 keeps the approved 1536×1024 DragonGlass editor and the 2.0.19 canonical-canvas fit behavior while adding a native adaptive Stream Deck+ touch-strip architecture.

The physical 800×100 touch display can now operate as:

- **Segmented** — four independent 200×100 touch regions.
- **Unified** — one seamless 800×100 display and logical touch control.
- **Adaptive** — automatically selects segmented or unified from the active application identifier, with an explicit fallback and editable app rules.

Unified touch input preserves the full 0–799 X coordinate space and cross-strip flick coordinates. The unified slot is remappable through the same Action / Appearance / States workflow as the existing controls. Switching back to segmented mode preserves all four region configurations.

Linux active-application detection is best effort and non-fatal: `OPENDECK_ACTIVE_APP` override, then `kdotool`, then `xdotool`, otherwise no application id. Polling occurs only for pages in Adaptive mode.

The strict-rustfmt correction identified by the 2.0.19 host run is carried forward. No background service or autostart is introduced. The active installed baseline remains OpenDeck+ 2.0.8 until 2.0.21 passes all host qualification gates and human visual review. Dial Stacks are deferred to 2.0.22.
