# OpenSanctuary v0.2.2 Battle.net-Style Launcher UI Design

## Goal

Move the OpenSanctuary launcher substantially closer to the information hierarchy and interaction rhythm of the Battle.net desktop app while preserving original OpenSanctuary branding, procedural artwork, and the existing v0.2.x native Linux backend.

## Visual hierarchy

The launcher uses two persistent top chrome rows. The first row is launcher-wide navigation with OpenSanctuary branding, HOME, GAMES, and SHOP-like navigation, plus notification, social, profile, diagnostics, and settings affordances. The second row is a compact favorites/game strip with a favorite marker, a selected Diablo III tile, muted future-game tiles, and an add-game affordance.

Selected Diablo III pages replace the full-width generic hero with a Battle.net-like game layout. A fixed-width left control column contains the game identity, game-specific quick links, version information, the dominant primary action, options, and runtime/status information. The remaining content area is an expansive update/news surface with a large featured card and smaller latest-content cards.

## Pages

- Startup remains `Overview` for Diablo III.
- `Home` shows a featured Diablo III banner and latest launcher/native-runtime cards.
- `Games` shows the existing native library view.
- `Shop` is a clearly labeled visual placeholder; OpenSanctuary does not impersonate or connect to Blizzard commerce.
- `Overview`, `Content`, `Activity`, `Settings`, and `Diagnostics` preserve existing backend behavior.

## Social and notifications

The top-right controls expose local-only notification and social drawers. Notifications summarize launcher activity and runtime readiness. The social drawer explicitly states that Battle.net social services are not connected in this clean-room release. Neither drawer captures credentials or claims Blizzard service connectivity.

## Data and behavior constraints

- No changes to install discovery, indexing, diagnostics, or engine launch behavior.
- No Blizzard logos, copyrighted art, copied icons, or bundled game assets.
- Diablo III installation content remains read-only.
- No Wine, Proton, DXVK, VKD3D, Windows DLLs, or compatibility-layer dependencies.
- egui/eframe stays pinned to 0.35.0 and Rust MSRV stays 1.92.
- Startup remains direct-to-Diablo rather than Home.
- Reduced-motion preference must continue to disable procedural animation.

## Testing

State-only behavior is covered with unit tests: global navigation classification, game-page classification, activity notification count, and local drawer defaults. The authoritative verification remains `./BUILD-ON-ARCH.sh` on a Rust-equipped Arch system, running format, workspace tests, Clippy with `-D warnings`, and release build.
