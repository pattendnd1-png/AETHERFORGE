# OpenDeck+ 2.0.24 — Rustfmt + Qualified Replacement Cutover

## 2.0.24 closure

OpenDeck+ 2.0.24 carries forward the approved 1536×1024 DragonGlass render, canonical-canvas fit scaling, remappable keys/dials/touch behavior, and Adaptive Unified Touch Strip architecture from 2.0.23.

The only production-code delta beyond version metadata is the exact Rust 1.98.1 `cargo fmt --check` shape requested by the 2.0.23 host: the Spotify adaptive-touch resolver assertion is compacted and the reconnect `sync_workspace_to_device` call is formatted exactly as rustfmt requires. No runtime logic changes are introduced.

After every automated qualification gate passes and the 2.0.24 qualification screenshot is explicitly approved, the replacement activation installs 2.0.24 under `~/.local/lib/opendeck-v2.0.24`, repoints the canonical `~/.local/bin/opendeck-studio` / `opendeck` launch paths, refreshes the desktop entry/icon, preserves 2.0.8 as the sole rollback install, and removes obsolete versioned/staging OpenDeck installs. No background service or autostart entry is created.

Dial Stacks are deferred to 2.0.25.
