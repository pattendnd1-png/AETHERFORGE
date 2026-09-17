# OpenDeck+ 2.0.28 — Rustfmt Final Qualifier + Replacement Cutover Closure

## 2.0.28 closure

OpenDeck+ 2.0.28 carries forward the approved 1536×1024 DragonGlass render, canonical-canvas fit scaling, remappable keys/dials/touch behavior, Adaptive Unified Touch Strip architecture, hidden/non-focus startup benchmark, visible qualification bootstrap ordering, and qualified replacement-cutover path from 2.0.27.

The only production-code closure in this release is the host-proven Rust formatting fix in `apps/opendeck-studio/src-tauri/src/qualification.rs`: one extra blank line before `fn epoch_ms()` is removed so `cargo fmt --all -- --check` matches Rust 1.98.1 exactly. No UI, render, touch-strip, remapping, hardware, startup, Twitch, persistence, or action behavior changes are included.

Qualification still captures the visible candidate before later gates, while startup benchmarking remains hidden and non-focus-stealing. Startup thresholds remain cold <=1200 ms and warm p95 <=650 ms.

After every automated qualification gate passes and the 2.0.28 qualification screenshot is explicitly approved, replacement activation installs 2.0.28 under `~/.local/lib/opendeck-v2.0.28`, repoints the canonical `~/.local/bin/opendeck-studio` / `opendeck` launch paths, refreshes the desktop entry/icon, preserves 2.0.8 as the sole rollback install, and removes obsolete versioned/staging OpenDeck installs. No background service or autostart entry is created.

Dial Stacks are deferred to 2.0.29.
