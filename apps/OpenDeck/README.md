# OpenDeck+ 2.0.29 — Hidden Startup Ready / No-rAF Closure

## 2.0.29 closure

OpenDeck+ 2.0.29 carries forward the approved 1536×1024 DragonGlass render, canonical-canvas fit scaling, remappable keys/dials/touch behavior, Adaptive Unified Touch Strip architecture, hidden/non-focus startup benchmark, candidate-bound screenshot pipeline, and qualified replacement-cutover path from 2.0.28.

The only production-code closure in this release is in `apps/opendeck-studio/src/perf/QualificationHarness.tsx`: the hidden `startup` qualification phase no longer waits on `requestAnimationFrame`/`settleFrames` before writing startup readiness. Hidden WebViews may throttle animation frames, which caused the 2.0.28 host run to stop at `COLD_START_READY` before startup timings were produced. Startup readiness is now emitted directly from the mounted React effect with the existing Tauri/WebView/React/workspace milestone payload.

Visible visual/performance qualification keeps its normal font/frame settling and focus behavior. The render, adaptive touch-strip behavior, remapping, HID runtime, Twitch integration, persistence, and action behavior are unchanged. Startup thresholds remain cold <=1200 ms and warm p95 <=650 ms.

After every automated qualification gate passes and the 2.0.29 qualification screenshot is explicitly approved, replacement activation installs 2.0.29 under `~/.local/lib/opendeck-v2.0.29`, repoints the canonical `~/.local/bin/opendeck-studio` / `opendeck` launch paths, refreshes the desktop entry/icon, preserves 2.0.8 as the sole rollback install, and removes obsolete versioned/staging OpenDeck installs. No background service or autostart entry is created.

Dial Stacks are deferred to 2.0.30.
