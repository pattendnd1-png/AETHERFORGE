# AetherBrowser v2.1.37 Stream Deck Windows-Parity Design

## Goal

Replace the current diagnostic-first Stream Deck user experience with a full native Rust Stream Deck configuration application that looks and behaves like the Windows Stream Deck software while preserving AetherForge's single-hardware-owner daemon and direct HID path. Stream Deck+ is the primary acceptance device: 4×2 keys, 4 pressable dials, and 4 touch-strip regions.

## Release Scope

AetherBrowser v2.1.37 is a focused Velora.tv + Stream Deck/Elgato release. Twitch, YouTube, and YouTube Music are known-good and are excluded from the v2.1.37 runtime test matrix. Their code paths must not be changed unless required to keep the workspace compiling.

## Windows-Parity Workspace

The new `aether-streamdeck-studio` binary is a standalone native desktop application. Its default visual hierarchy follows the Windows Stream Deck configuration app: device/profile selectors across the top, a large device canvas at left/center, searchable action library at right, and a context-sensitive property inspector across the lower editor region. The default skin is neutral Elgato-like dark gray so the application visually reads like its Windows counterpart rather than an AetherForge-themed dashboard.

The Stream Deck+ canvas represents eight key slots in a 4×2 grid, a full-width touch strip divided into four encoder regions, and four dial controls under the strip. Selection state is shared between key and encoder slots. Dragging or activating an action-library item assigns it to the selected slot. Slots support clear, duplicate, move, folder navigation, and multi-action semantics.

## Profile and Page Model

Profiles are device-specific and persistent. A profile contains one or more pages. Each page contains keypad and encoder assignments. Users can create, rename, duplicate, delete, import, and export profiles; create and navigate pages; and associate profiles with an application identity for automatic switching. `Forge Master` remains available as a built-in seeded profile but is no longer the only editable profile.

Persistent state is stored under `~/.local/share/aetherforge/aether-browser/streamdeck/` in JSON. Writes are atomic: serialize to a sibling temporary file, flush, then rename. Corrupt state is never silently discarded; the app falls back to a clean in-memory document and reports the load error in the status bar.

## Actions and Plugins

The built-in action catalog is grouped into Windows-like categories and is searchable. Initial categories include System, Stream/OBS, Browser, Multimedia, Navigation, Profiles, Multi Action, and AetherForge. Every action has stable metadata: id, display name, category, controller compatibility, icon treatment, default settings, and optional property-inspector schema.

The plugin layer models Elgato's public concepts rather than claiming binary compatibility with proprietary Windows plugins. A plugin manifest adapter recognizes public Stream Deck manifest concepts including Keypad and Encoder controllers, action states, action settings, global settings, property inspector paths, and profile bundles. Protected Marketplace plugins continue to require an authorized/official runtime and are not reverse engineered.

## Property Inspector

Selecting an assigned slot opens a property inspector. Common fields include Title, Icon/Image, behavior, and action-specific settings. Built-in actions use native Rust controls. Plugin actions may declare an HTML property inspector; v2.1.37 records and surfaces the inspector contract but does not claim execution of arbitrary proprietary Marketplace property inspectors unless an authorized runtime exists.

Settings are per action instance. Global plugin settings are separate from action-instance settings. Changes update the canvas preview immediately and are persisted through the profile store.

## Stream Deck+ Hardware Synchronization

The existing direct HID path remains the single writer to hardware. The editor sends desired state through the Aether Deck layer; hardware output includes real JPEG key images, the 800×100 LCD/touch display, brightness, and boot-logo restore. Encoder feedback uses four independent 200×100 regions and is updated whenever an encoder action or title/value changes.

The input loop maps key down/up, dial rotation, dial down/up, and touch events to the active profile/page action assignments. The UI provides live visual feedback for physical interaction. Hardware reconnect restores the active profile/page without requiring editor restart.

## Twitch Account Integration

Stream Deck Studio exposes Twitch under Preferences → Accounts. The app uses Twitch's public Device Code OAuth flow so the user authorizes in Twitch without entering a Twitch password into AetherForge. Access and refresh tokens are stored through the desktop Secret Service, validated against Twitch, and refreshed as a public client when validation fails. Browser Twitch playback remains outside the v2.1.37 runtime test matrix; only Stream Deck account connectivity and Twitch action availability are in scope.

The built-in Twitch action catalog includes chat message, viewer count, stream title/category, commercial, clip, stream marker, creator dashboard, slow/followers/subscribers/emote-only chat modes, and Shield Mode.

## Velora.tv

Velora remains browser-authoritative using the persistent Chromium/X11 profile and first-class provider routing established in v2.1.35. v2.1.37 adds focused Velora verification only: provider registration, WebSession auth mode, persistent profile routing, URL ownership, and a launch/session smoke contract. No private API scraping is added.

## Verification

The v2.1.37 preinstall hard gate runs full Rust formatting/check/clippy/workspace tests/build, then focused shell contracts for release identity, Velora, Stream Deck Windows-parity model/UI contracts, profile persistence, HID output contract, Stream Deck+ device capabilities, and packaging. It explicitly does not run Twitch, YouTube, or YouTube Music runtime probes.

Host acceptance generates a focused `Aether-Browser-v2.1.37-STREAMDECKPLUS-VERIFY.txt` and `Aether-Browser-v2.1.37-VELORA-VERIFY.txt`. The Stream Deck+ acceptance covers software launch, device detection, direct HID, key image output, touch-strip output, brightness, 8 keys, 4 dial rotations, 4 dial presses, 4 touch regions, profile persistence/reopen, and reconnect. The previous diagnostic monitor remains available for low-level confirmation and rollback.

## Safety and Non-Goals

Firmware flashing remains restricted to verified official vendor packages matching VID/PID and SHA-256, with explicit user confirmation. The release does not bypass Elgato DRM, impersonate an official Elgato runtime, or claim Windows plugin binary compatibility. It does not change working Twitch, YouTube, or YouTube Music behavior.
