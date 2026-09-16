# OpenDeck v2.0.8


OpenDeck is a clean-room Linux Stream Deck controller/editor. v2.0.8 carries forward the v2.0.7 Dials/encoder closure, applies the Rustfmt correction found by host qualification, and preserves the v2.0.6 Windows editor hierarchy, Twitch sign-in, HID runtime, persistence, OBS actions, and clean-room assets.

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

OpenDeck v2.0.8 does not install or start a background OpenDeck daemon and does not enable autostart. The Stream Deck + HID runtime is owned by the OpenDeck Studio process: it opens the device only while the app is running, reconnects on hotplug, and releases the device on exit. Build/install qualification never synthesizes a hardware press and never starts public streaming or recording.

## Stream Deck + Linux access

OpenDeck targets Elgato Stream Deck + USB VID `0fd9`, PID `0084`. Install the narrow udev rule once, reload the rules, and reconnect the deck:

```bash
sudo install -Dm644 packaging/70-opendeck-streamdeck.rules /etc/udev/rules.d/70-opendeck-streamdeck.rules && sudo udevadm control --reload-rules && sudo udevadm trigger
```

The rule uses `TAG+="uaccess"`; it does **not** make HID devices world-writable. Check visibility and current-user hidraw permissions without sending any device command:

```bash
./scripts/check-streamdeck-plus.sh
```

A successful host probe ends with `OPENDECK_STREAMDECK_PLUS_PROBE=PASS`. Physical input/output acceptance is a separate runtime gate after software qualification.

## v2.0.2 action execution closure

- Adds an explicit Property Inspector **Test Action** path for configured actions.
- Executes OBS Scene and Toggle Input Mute through the existing bridge.
- Requires an additional confirmation before Test Action can toggle OBS streaming or recording.
- Executes Folder, Next Page, Previous Page, and Switch Profile locally through the persisted editor model.
- Keeps qualification non-destructive: automated tests never start a public stream or recording.



## v2.0.8 Dials/encoder + Rustfmt closure

- Carries forward the full v2.0.6 Windows editor hierarchy candidate without activating the failed v2.0.6 build.
- Fixes the Dials action-library filter so only actions declaring `dial` support appear in Dials mode.
- Prevents touch-only/key+touch actions such as **Folder** from leaking into the Dials list.
- Preserves the existing regression assertion that Folder is visible in Keys mode and absent after switching to Dials.
- Keeps encoder **Press**, **Rotate Left**, and **Rotate Right** as independent bindings.
- Adds explicit **Press + Rotate Left** and **Press + Rotate Right** bindings; rotation events now carry the current encoder-button held state from the Rust HID runtime.
- Makes the configuration interaction selector authoritative for Action Library assignment so every dial interaction can actually be assigned independently.
- Preserves Twitch 2.0.5 auth polling, Stream Deck+ HID runtime, udev rules, persistence, OBS actions, and qualify-before-cutover behavior unchanged.

## v2.0.6 Windows editor parity

- Replaces the flattened toolbar with compact device/profile identity controls and compact app actions.
- Places the Stream Deck+ controls directly on the editor canvas with numbered page navigation beneath the dials.
- Adds authoritative Keys and Dials action-library modes with automatic selection-aware switching and collapsible action groups.
- Converts the generic Property Inspector into a selected-control configuration strip below the device.
- Removes the permanent status footer; save state lives in the header and action feedback uses an in-shell status toast.
- Applies DragonGlass materials over Windows-style editor geometry without copying Elgato binaries or artwork.

## v2.0.5 Twitch sign-in completion

- Replaces the complex `open_device` tuple result with a named `OpenedDevice` domain type.
- Preserves the HID transport, model, serial, reconnect, render, and action-dispatch behavior unchanged.
- Keeps `-D warnings` intact; no Clippy suppression is introduced.
- Requires the full frontend, Rust, release, and Tauri qualification sequence before activation.

## v2.0.3 hardware and Windows-workflow closure

- Adds a single-owner Rust HID runtime for Stream Deck + with hotplug/reconnect and 50 ms timed input reads.
- Decodes all eight keys, four dial buttons, signed rotary ticks, and touch-strip tap/press/flick reports.
- Renders eight 120×120 JPEG key frames and one 800×100 touch-window frame from the persisted workspace.
- Routes physical controls through the same action executor as the editor. Physical Stream/Record presses are explicit user input; automated qualification never invokes them.
- Keeps the Action Library in a full-height right column, the device editor in the upper-left workspace, page controls directly below it, and the Property Inspector below the device.
- Adds live Stream Deck + connection status in the top bar.
