# OpenSanctuary v0.2.5 Launcher UI Design

## Goal
Move the OpenSanctuary launcher another visual and interaction tier toward the Battle.net desktop launcher while preserving the existing corrected v0.2.4 source indexing, diagnostics, settings, content inventory, and native-engine behavior.

## Design direction
The launcher remains original OpenSanctuary software and does not copy Blizzard logos, proprietary artwork, account UI, or remote service behavior. The visual hierarchy borrows familiar launcher conventions: a compact global navigation bar, customizable-looking favorites/last-played access, a selected-game control surface, expansive game news/content space, persistent utility affordances, and a consolidated downloads/activity drawer.

## v0.2.5 changes
- Add a compact **Last Played** group beside Favorites with a one-click Diablo III entry.
- Strengthen the selected-game strip tile with animated selection motion and installation-state detail.
- Refine the Diablo III control column into a denser game-details shell with version/status grouping and a more prominent primary action area.
- Add a Battle.net-like **Latest Stories** content grid: one large lead card, two stacked secondary cards, and a lower compact story row.
- Add a richer downloads drawer with queue summary, state icon, progress bar, and detail text per activity item.
- Add a compact launcher menu attached to the OpenSanctuary brand for Home, Settings, Diagnostics, and About-style local information.
- Improve notification/account/social surfaces with consistent panel chrome and headings.
- Respect reduced-motion configuration for every new animation.

## Boundaries
- No Blizzard authentication, BattleTag impersonation, commerce, chat network, or updater protocol.
- No Blizzard logos or copied Battle.net/Diablo artwork in the repository.
- No Wine, Proton, DXVK, VKD3D, Bottles, Lutris, or Windows runtime dependency.
- No changes to read-only Diablo III content handling.
- All UI work remains compatible with egui/eframe 0.35.0 and Rust 1.97.

## Verification
`BUILD-ON-ARCH.sh` remains the authoritative formatter, test, Clippy `-D warnings`, and release-build gate. Pure helper behavior for queue presentation and local navigation must have unit tests.
