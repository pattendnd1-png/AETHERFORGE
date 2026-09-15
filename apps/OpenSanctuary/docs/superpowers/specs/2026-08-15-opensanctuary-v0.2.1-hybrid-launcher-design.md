# OpenSanctuary v0.2.1 Hybrid Launcher UI Design

## Goal

Make the OpenSanctuary launcher feel substantially closer to the modern Battle.net launcher while remaining original, Linux-native, and legally distinct. The launcher should open directly to Diablo III, but also provide a launcher-wide Games/Home surface so it behaves like a game platform rather than a single diagnostic utility.

## Scope

v0.2.1 is a presentation-focused release. It preserves the v0.2.0 install discovery, content indexing/cache, diagnostics, settings, and native engine launch paths while reorganizing how they are presented.

The release adds:

- a launcher-wide Games/Home surface;
- direct startup into the Diablo III selected-game page;
- a denser game rail with a selected-game accent and compact placeholders;
- a stronger launcher top bar with Games, Activity, native-runtime status, diagnostics, and settings controls;
- a selected-game header and secondary navigation for Overview, Content, Activity, and Settings;
- a larger cinematic hero composition and visually dominant Play/Index/Locate action;
- compact install/build information adjacent to the primary action;
- a smaller collapsed-by-default Downloads & Activity tray with an expandable recent-task preview;
- refined dark navy/charcoal styling, tighter spacing, restrained blue accenting, and fewer default-looking egui controls.

## Non-Goals

This release does not decode BLTE payloads, add logical asset extraction, connect to Blizzard services, clone Blizzard artwork or logos, or modify the read-only Diablo III installation/indexing behavior.

## Navigation Model

`Page` gains a launcher-wide `Games` page. Startup remains `Overview`, which is the Diablo III selected-game page.

- Top `GAMES` -> `Games`
- Left Diablo III tile -> `Overview`
- Top `ACTIVITY` -> `Activity`
- Diablo III subnav -> `Overview`, `Content`, `Activity`, `Settings`
- Diagnostics and settings utility controls remain available from the top bar

The selected-game accent appears only for Diablo III pages (`Overview`, `Content`, `Settings`, `Diagnostics`) and not for launcher-wide `Games` or `Activity`.

## Games/Home Surface

The Games page has a large `YOUR GAMES` header, one featured Diablo III card, and a smaller library row. The Diablo III card reuses the original procedural OpenSanctuary hero treatment, shows install/index state and local build metadata, and exposes the same primary action as the selected-game page. Placeholder cards communicate future supported titles without pretending those games are functional.

## Selected-Game Surface

The Diablo III page is restructured into:

1. compact selected-game header + subnav;
2. large cinematic hero;
3. game identity/title and one dominant primary action;
4. small options/locate/re-index affordances adjacent to the primary action;
5. concise build/install state text;
6. lower information cards and build strip.

Technical details remain available but no longer dominate the first viewport.

## Activity Tray

The bottom activity tray defaults to collapsed. The collapsed row shows a download/activity glyph, the most recent task summary, active count, and expansion chevron. Expanded state shows up to three recent tasks. The full Activity page remains available for complete history.

## Styling

The UI uses original OpenSanctuary branding and procedural artwork. The visual language targets the same usability characteristics as Battle.net without copying proprietary assets:

- near-black navy global chrome;
- dark charcoal content background;
- muted steel text;
- vivid blue selection/action accents;
- minimal border radius on utility controls;
- strong visual hierarchy between launcher chrome, selected-game identity, and primary action;
- compact rail and top navigation;
- subtle animated hero particles when reduced motion is disabled.

## Testing

Pure page/navigation helpers are unit-tested:

- `Page::is_game_page()` identifies selected-game pages;
- `Page::games_nav_active()` distinguishes launcher-wide Games from the selected-game page;
- activity tray summary helper returns Idle, active-count, or most-recent-task text deterministically.

The release keeps the existing v0.2 verifier:

- `cargo fmt --all -- --check`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo build --workspace --release`

## Compatibility

- Rust 1.92+; expected user environment Rust 1.97.x.
- egui/eframe 0.35 APIs only (`egui::Panel`, `set_theme`, `style_mut_of`).
- No Wine, Proton, DXVK, VKD3D, Lutris, Bottles, or Winetricks dependencies.
- No writes to the Diablo III installation.
