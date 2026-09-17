# OpenDeck+ 2.0.26 — Startup Type Integration Closure

## 2.0.26 closure

OpenDeck+ 2.0.26 carries forward the approved 1536×1024 DragonGlass render, canonical-canvas fit scaling, remappable keys/dials/touch behavior, Adaptive Unified Touch Strip architecture, hidden/non-focus startup benchmark, and qualified replacement-cutover path from 2.0.25.

The only product-code closure in this release is the frontend type integration required by the hidden startup benchmark. `QualificationHarness` now consumes the shared `QualificationPhase` type (including `startup`), declares the optional `startedAtMs` and `tauriSetupMs` milestone props already supplied by `App`, and records a WebView JavaScript-start timestamp at module evaluation. This directly closes the four TypeScript build errors observed on the 2.0.25 host run without changing render geometry or interaction behavior.

Startup qualification remains hidden and non-focus-stealing: the cold-start sample, 20 warm-start samples, and 60-second idle sample use the `startup` phase; only the visual qualification and intentional hardware exercise surface a visible window. Startup thresholds remain cold <=1200 ms and warm p95 <=650 ms.

After every automated qualification gate passes and the 2.0.26 qualification screenshot is explicitly approved, replacement activation installs 2.0.26 under `~/.local/lib/opendeck-v2.0.26`, repoints the canonical `~/.local/bin/opendeck-studio` / `opendeck` launch paths, refreshes the desktop entry/icon, preserves 2.0.8 as the sole rollback install, and removes obsolete versioned/staging OpenDeck installs. No background service or autostart entry is created.

Dial Stacks are deferred to 2.0.27.
