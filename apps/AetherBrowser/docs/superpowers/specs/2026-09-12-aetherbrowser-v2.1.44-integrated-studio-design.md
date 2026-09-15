# AetherBrowser v2.1.44 Integrated Studio Design

## Goal
Make AetherBrowser the single-window production shell for OBS Studio, Stream Deck Studio, Twitch authorization, and creator tooling while preserving the working v2.1.43 host-gate contracts.

## Approved UX
- OBS Studio opens inside the active AetherBrowser tab by default. It is the real installed OBS GUI, not a reduced imitation.
- The embedded OBS window remains fully interactive and retains OBS-native preview, Scenes, Sources, Audio Mixer, transitions, Studio Mode, filters, properties, docks/plugins, profiles, scene collections, recording, streaming, replay buffer, virtual camera, and settings.
- OBS may be explicitly detached during the current session, but detached state is session-only; the next AetherBrowser launch adopts it back into the main window.
- AetherBrowser continues to use OBS WebSocket v5 as a typed control/telemetry plane for Stream Deck and Aether integrations.
- The address/search bar supports a real caret with Left/Right/Home/End, Backspace/Delete at the caret, Ctrl+A, insertion at the caret, and visible caret placement.
- Stream Deck Studio stays embedded by default and may explicitly detach/reattach.
- Twitch account connection presents Twitch's own hosted login/authorization page inside the current AetherBrowser window. The user sees Twitch screen name/password/2FA on Twitch's page; AetherBrowser never captures or stores the password.
- The Twitch application Client ID is hidden from normal UI. The registered AetherForge Client ID is supplied at build/runtime configuration; if absent, the UI reports app configuration missing rather than a generic HTTP 400.
- Twitch OAuth tokens remain in Secret Service and are validated/refreshed according to the selected OAuth flow.
- Stream Deck Marketplace stays an embedded browser surface. Local compatible profile/theme packages are tracked by Stream Deck Studio, editable AetherForge copies can be activated, and the most recently used compatible package is restored immediately after Twitch reaches CONNECTED.
- Vendor originals, licensing, DRM, and protected plugin code are not modified or bypassed.

## OBS Architecture
Register OBS as a first-class `ExternalAppTarget` using the Linux X11 child-window host. Launch `obs` with `QT_QPA_PLATFORM=xcb`, adopt its top-level X11 window, reparent it into the AetherBrowser content geometry, and include OBS in restart orphan adoption. Existing `aether://stream` dashboard remains available as a lightweight control/diagnostic page, but its primary CTA opens `aether://external-app/open?id=obs`.

## Twitch Architecture
Keep the OAuth network/authentication logic outside of the GUI. Normal Accounts UI calls `begin_twitch_account_login`; no Client ID field is rendered. Device/hosted authorization pages are opened through the browser-control request path so they become tabs in the existing AetherBrowser window instead of spawning a second top-level browser window. Detailed Twitch JSON error messages are surfaced. The browser profile owns the username/password/2FA fields rendered by Twitch itself.

## Stream Deck Theme Package Architecture
Add a local package registry under `~/.local/share/aetherforge/aether-browser/streamdeck/marketplace/`. A package record stores package path/name, device product id, editability, last-used timestamp, and optional imported Aether profile id. Compatible package selection updates `last-theme.json`; when Twitch transitions to CONNECTED, Studio reloads this record and activates the corresponding editable local profile when available. Official `.streamDeckProfile` and Marketplace ZIP files remain vendor artifacts; Studio may index/copy them but never reverse-engineers or modifies protected code.

## Verification
- Existing v2.1.39 containment, v2.1.40 Deck Clippy, v2.1.41 reintegration, v2.1.42 engine Clippy, and v2.1.43 host-summary regressions remain green.
- New v2.1.44 gates cover omnibox editing, Twitch hosted-login UX, OBS in-window full-workspace target, and last-used Stream Deck theme restore.
- Package/static verification must pass locally. Cargo/Clippy and physical OBS/Stream Deck/X11 behavior remain host-proven unless a Rust toolchain is available in the packaging environment.
