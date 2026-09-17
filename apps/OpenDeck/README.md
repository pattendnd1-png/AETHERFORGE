# OpenDeck+ 2.0.23 — Lint + Qualified Replacement Cutover

## 2.0.23 closure

OpenDeck+ 2.0.23 carries forward the approved 1536×1024 DragonGlass render, canonical-canvas fit scaling, full remappable key/dial/touch behavior, and Adaptive Unified Touch Strip architecture from 2.0.22.

This release fixes the single host lint failure by removing the unused `slot` destructuring binding from `UnifiedTouchControl` while preserving the public `slot: ControlSlot` prop shape. No visual behavior changes are intended.

After all automated qualification gates pass and the qualification screenshot is explicitly approved, the 2.0.23 activation phase becomes the first canonical replacement cutover: it installs 2.0.23 under `~/.local/lib/opendeck-v2.0.23`, repoints `~/.local/bin/opendeck-studio` and `~/.local/bin/opendeck`, installs the canonical desktop entry/icon, preserves 2.0.8 as the sole rollback install, and removes obsolete versioned OpenDeck installs/staging directories from `~/.local/lib`. It does not create background services or autostart entries.

The active installed baseline remains 2.0.8 until the host qualification run is fully green and human visual approval is supplied to the activation phase. Dial Stacks are deferred to 2.0.24.
