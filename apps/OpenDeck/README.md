# OpenDeck v2.0.2


OpenDeck is a clean-room Linux Stream Deck controller/editor. v2.0.2 advances the host-qualified v2.0.1 customization baseline with explicit action execution/testing while preserving the existing OBS, Twitch, and Elgato Marketplace integration boundaries.

## v2.0.1 customization baseline

- Stream Deck Plus editor with 8 keys, 4 rotary controls, and 4 touch-strip regions.
- Per-control action bindings with independent dial press / rotate-left / rotate-right interactions and touch interactions.
- Contextual Property Inspector with Action, Appearance, and States panels.
- Editable title visibility, font, size, weight, text/background colors, alignment, offsets, icon/background references, fit mode, opacity, and active-state overrides.
- Profiles and pages with create, rename/duplicate/delete profile controls and add/duplicate/delete page controls.
- Drag actions from the Action Library directly onto controls.
- Drag controls to move their complete configuration; Ctrl/Alt drag copies instead of moves.
- Searchable asset browser for imported images, preserved OpenDeck icon packs, and Marketplace-sourced assets.
- Undo/redo for editor content changes.
- Independently collapsible and resizable Action Library and Property Inspector with persisted panel preferences.
- Compact Windows-style geometry; DragonGlass remains intentionally deferred until the workflow is proven.

## Persistence

The Rust backend is authoritative for editor data. Workspace state is stored under `~/.config/opendeck-v2/editor/` using atomic writes and a backup workspace. Imported image assets are copied into the OpenDeck asset library under `~/.config/opendeck-v2/assets/`. Existing icon packs remain discoverable from `~/.local/share/opendeck/icon-packs`.

The old `opendeck-v2.keys` browser-storage key is read only as a one-time v2.0.0 migration input. It is deleted only after the migrated workspace is successfully written by the Rust backend. Browser localStorage is not the editor source of truth.

## Connections retained

- OBS WebSocket 5.x: version/status, scene discovery/switching, stream toggle, record toggle, and input mute toggle.
- Twitch OAuth Device Code Flow with token validation/refresh; no Twitch password collection.
- Official Elgato Marketplace browser handoff plus discovery of supported downloaded Stream Deck package types.

## Runtime policy

OpenDeck v2.0.2 does not install or start a background OpenDeck daemon and does not enable autostart. Hardware/HID runtime remains deliberately separate from this editor qualification step; no public-stream action is triggered by build or install verification.

## v2.0.2 action execution closure

- Adds an explicit Property Inspector **Test Action** path for configured actions.
- Executes OBS Scene and Toggle Input Mute through the existing bridge.
- Requires an additional confirmation before Test Action can toggle OBS streaming or recording.
- Executes Folder, Next Page, Previous Page, and Switch Profile locally through the persisted editor model.
- Keeps qualification non-destructive: automated tests never start a public stream or recording.
