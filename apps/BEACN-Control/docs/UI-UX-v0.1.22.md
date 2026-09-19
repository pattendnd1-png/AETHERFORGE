# v0.1.22 UI/UX Contract — Mic Memory / On Device

## Purpose

The interface must distinguish three different things that previously looked like one profile state: the parameter state read from microphone memory, a cached copy of a prior successful read, and a host-local profile/edit. The UI must never imply a hardware write when only the local DSP model changed.

## Live Profile bar

The existing Live Profile command row stays in place. Directly below it is the new `MIC MEMORY` strip implemented by `src/ui_ux.rs` and integrated by `src/main.rs`.

States:

- `READING MIC MEMORY`: asynchronous startup/reload read is in progress. Reload is disabled while a read is active.
- `ON DEVICE ACTIVE`: supported parameters were read from the currently connected microphone.
- `ON DEVICE CACHED`: live hardware read failed, but a previously successful host-side cache was available.
- `LOCAL EDIT`: an On Device state was imported and then changed locally. Mic memory is explicitly unchanged.
- `LOCAL PROFILE`: a normal host-saved profile/snapshot is active. Mic memory is explicitly unchanged.
- `ON DEVICE UNAVAILABLE`: neither a usable live import nor cache was available.

The strip reports firmware, serial when available, imported-setting count, failed/unavailable count, and exposes `RELOAD MIC`.

## Edit transition

A loaded On Device state is treated as read-only provenance. On the first DSP edit, the active profile becomes `On Device - Local`, the UI state becomes `LOCAL EDIT`, and the existing local autosave/private-DSP update path continues. There is no hidden hardware commit.

`SAVE` while still on the canonical `On Device` name also creates/detaches to a local profile rather than pretending to save into microphone memory.

## Revert / reload

When the canonical On Device state is active, REVERT means re-read the microphone. For normal local profiles, REVERT keeps its existing local-profile behavior. The dedicated `RELOAD MIC` action always requests a new mic-memory read.

## Device and Settings language

Required ownership/status language:

- `MIC MEMORY READ-ONLY`
- `HARDWARE WRITES BLOCKED`
- `RAW MIC PRESERVED`
- `SYSTEM DSP ISOLATED`

The UI may expose hardware-write controls such as LED control for parity/layout planning, but those controls remain unavailable in this release.

## DragonGlass treatment

The new strip follows the existing AetherForge visual contract: near-black/navy translucent surface, restrained indigo/violet border, pale lavender metadata, compact typography, rounded corners, and no bright success color that could be confused with an applied hardware write.

## Responsive behavior

The memory strip lives inside the top Live Profile panel and uses wrapped horizontal layout, so it remains visible in compact widths without introducing a second window, modal, or fixed-width side panel.

## v0.1.22 mic-memory provenance refinement

The MIC MEMORY strip now exposes two compact provenance badges:

- **READ ONLY** — reinforces that startup memory import never invokes a hardware setter path.
- **SERIAL VERIFIED** — appears only on cache fallback, meaning the cached profile's embedded mic serial matched the connected BEACN Mic. Cross-device cache fallback is rejected.

Cache source copy is now `Same-mic cache` rather than a generic `Cache fallback` label.
