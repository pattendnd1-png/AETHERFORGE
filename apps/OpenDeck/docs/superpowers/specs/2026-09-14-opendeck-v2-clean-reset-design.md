# OpenDeck v2 Clean Reset Design

## Purpose

Retire the accumulated OpenDeck v1.x implementation and replace it with a clean-room v2 baseline built from an empty source tree. The first priority is a usable Windows Stream Deck-style application, not feature count.

## Reference authority

- Windows Stream Deck 7.5.1.22901 is the visual and interaction benchmark.
- Elgato Marketplace Connect 2.5.1.119 is reference evidence for Marketplace/OBS surfaces.
- Previously supplied local icon packs are user-owned inputs, not source code.
- No vendor executable/DLL code is executed or redistributed.
- No source file from the retired OpenDeck v1.x tree is copied into v2.

## First usable baseline

1. Windows-style shell: compact device/profile header, central Stream Deck Plus editor, right Action List, bottom Property Inspector.
2. Stream Deck Plus visual contract: 8 keys, 4 touch sections, 4 dials.
3. OBS WebSocket 5.x: connection, scene list/selection, stream toggle, record toggle, input mute toggle.
4. Twitch: public-client OAuth Device Code Flow, browser activation, token validation, refresh, and identity state.
5. Elgato Marketplace: official browser handoff and discovery of compatible local downloads; no password collection or undocumented login API.
6. Local icon-pack discovery from the retained user data and Downloads.
7. No daemon, no background services, no autostart, no DragonGlass in the first baseline.

## Cutover

The new source must build and pass its gates before destructive cutover. Then:
- snapshot user profiles/config and icon-pack data;
- stop and disable retired OpenDeck services/processes;
- remove old user-local OpenDeck binaries, desktop entries, service units, autostart entries, and the active v1.3.1 source tree;
- remove an exact package-owned system OpenDeck package only when ownership is safely identified as an OpenDeck package;
- install only the new v2 Studio binary and desktop entry;
- leave OpenDeck stopped after installation.

## Success criteria

The baseline is accepted only when frontend tests/lint/build, Rust fmt/check/strict Clippy/tests/release build, Tauri build, clean source contract, old-install absence checks, and new binary checksum checks pass. Runtime OBS/Twitch acceptance remains explicitly observable in the UI rather than faked by source markers.

## Storage resilience

The editor must remain usable when browser/WebView local storage is unavailable. Key-layout persistence is best-effort: storage read/write failures must never prevent the Windows-style editor from rendering or accepting edits.
