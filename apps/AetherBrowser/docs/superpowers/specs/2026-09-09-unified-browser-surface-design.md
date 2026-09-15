# Aether Browser Unified Surface Design

## Goal
Build Aether Browser 2.1.0 as a clean-room UI/runtime composition that matches the approved Aether Browser Cosmic Interface and contains no inherited split-surface UI architecture.

## Architecture
A single browser-owned Servo WebView renders the complete Aether UI document: title bar, tab strip, navigation, integrated launcher, home composition, utility dock, and telemetry. The Aether home route is represented logically by `aether://home`, but its visual content is owned entirely by the unified UI document. The active content WebView is painted/composited only for non-home routes.

DragonGlass remains a visual language only. No external UI owner, independent side surface, state machine, or compatibility path participates in the runtime composition.

## Visual specification
The approved Aether Browser Cosmic Interface render from 2026-09-09 is authoritative for geometry, density, hierarchy, typography, glass treatment, hero composition, feature cards, integrated launcher, utility dock, and bottom status strip. Runtime data may differ from illustrative render data, but structure and styling must remain aligned.

## Runtime behavior
The Aether UI document loads once. Tab, omnibox, download, bookmark, stream, and telemetry changes are applied with in-place JavaScript. Startup is hidden until the first usable Aether UI frame exists, then revealed atomically. Home never composites the inactive `about:blank` content surface over the Aether UI.

## Preserved systems
Servo web rendering, AetherAI Unix-socket bridge/service, media service, Stream Deck daemon, storage/downloads, native first-party pages, creator integrations, and communications remain functional backends.

## Release gates
The current-only unversioned tests must pass. Source/package archives must contain only current release contracts and current UI assets. The one-file installer must provide a single-instance lock, live Cargo output, version-specific VERIFY/INSTALL-VERIFY files, and a failure-safe diagnostic handoff.
