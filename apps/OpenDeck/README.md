# OpenDeck+ 2.0.27 — Visible Qualification Bootstrap Closure

## 2.0.27 closure

OpenDeck+ 2.0.27 carries forward the approved 1536×1024 DragonGlass render, canonical-canvas fit scaling, remappable keys/dials/touch behavior, Adaptive Unified Touch Strip architecture, hidden/non-focus startup benchmark, and qualified replacement-cutover path from 2.0.26.

The only product-code closure in this release is the visible qualification bootstrap ordering. Because Tauri qualification windows start hidden, 2.0.26 waited for animation frames before calling `qualificationFocusWindow()`. Hidden WebViews can throttle animation frames, so visual readiness could deadlock and the fallback diagnostic captured whichever desktop window was active. 2.0.27 calls the candidate show/focus handshake first for `visual` and `performance`, then waits for fonts/frames and records visual readiness. The `startup` phase still returns before any show/focus call and remains hidden/non-focus-stealing.

Startup qualification remains hidden and non-focus-stealing: the cold-start sample, 20 warm-start samples, and 60-second idle sample use the `startup` phase; only the visual qualification and intentional hardware exercise surface a visible window. Startup thresholds remain cold <=1200 ms and warm p95 <=650 ms.

After every automated qualification gate passes and the 2.0.27 qualification screenshot is explicitly approved, replacement activation installs 2.0.27 under `~/.local/lib/opendeck-v2.0.27`, repoints the canonical `~/.local/bin/opendeck-studio` / `opendeck` launch paths, refreshes the desktop entry/icon, preserves 2.0.8 as the sole rollback install, and removes obsolete versioned/staging OpenDeck installs. No background service or autostart entry is created.

Dial Stacks are deferred to 2.0.28.
