# OpenSanctuary v0.3.7 Battle.net-Fidelity UI/UX Design

## Goal

Make OpenSanctuary feel like a modern Battle.net-style game launcher while preserving the working v0.3.6 Battle.net integration, install/update supervision, account safety boundary, and native-engine-first launch behavior.

## Scope

This release changes presentation architecture only. It does not change Battle.net credentials handling, compatibility-runtime boundaries, install discovery, bridge persistence, Diablo III launch semantics, CASC indexing behavior, or native-engine readiness rules.

## Design Principles

1. Game launcher first; technical control center second.
2. One dominant game action at a time: INSTALL, SIGN IN, UPDATE, VERIFYING, INDEXING, PLAY, STARTING, or PLAYING.
3. Progressive disclosure: player-facing readiness is visible by default; bridge/runtime diagnostics live behind details/diagnostics surfaces.
4. Battle.net fidelity without copying proprietary assets: dark blue-black shell, thin cool-blue separators, large hero surfaces, compact navigation, restrained glow, and spacious content hierarchy.
5. All Blizzard authentication remains on Blizzard-controlled surfaces. OpenSanctuary never captures or stores raw passwords, authenticator codes, recovery codes, captcha responses, or security answers.
6. Reduced-motion preferences continue to suppress nonessential transitions.

## Launcher Shell

The top shell contains the OpenSanctuary brand, HOME, GAMES, BATTLE.NET, SHOP, notification control, downloads control, and account control. Search remains local and secondary. A favorites/last-played strip sits directly beneath the top shell.

The shell should expose only concise state. Technical implementation details such as Wine runner identity, prefix paths, PIDs, CASC fingerprints, and bridge recovery history are excluded from the persistent chrome.

## Diablo III Page

The Diablo III page uses a large hero treatment with title, edition/build metadata, one dominant primary action, a compact readiness caption, game settings, and an overflow menu. The lower content area contains featured launcher/news cards plus a compact GAME STATUS surface.

Default GAME STATUS contains:
- overall ready/not-ready state,
- Battle.net session state,
- installation state,
- content-index state,
- a DETAILS action.

The DETAILS action routes to advanced diagnostics rather than duplicating low-level data in the game hero.

Primary action mapping remains driven by the existing `LauncherModel::primary_action()` and bridge state. OpenSanctuary continues to use the existing canonical install/index/play orchestration methods.

## Battle.net Page

The Battle.net page is dominated by the integrated Battle.net client surface. X11 continues to use the existing embedded window host; Wayland continues to use managed companion mode. Three compact status chips summarize account/session, Agent/bridge, and game readiness. Repair and deep bridge data move behind an overflow/details surface.

## Account

The account page shows player-facing BattleTag/account-hint, region, desktop session state, and Diablo III access/readiness. Advanced session/bridge details route to diagnostics. Password capture remains prohibited.

## Downloads and Activity

A persistent downloads button in the top chrome and bottom tray shows active-count/status. The expandable downloads surface represents both Blizzard-managed lifecycle states and OpenSanctuary-owned local tasks without claiming proprietary byte-level download progress that OpenSanctuary does not actually possess.

For known local operations, progress labels may be semantic rather than numeric: Installing, Updating, Verifying, Indexing, Starting, Running. Existing `LauncherModel.activity` items remain the source for local task history.

## Notifications

Notifications are grouped launcher events derived from existing activity/session state. The drawer presents game, Battle.net, and OpenSanctuary events in a compact feed. It must not fabricate friend/social or Blizzard service data that is unavailable from supported interfaces.

## Diagnostics

Advanced diagnostics remain available and retain bridge health, runner state, paths, build/content metadata, last recovery, native renderer/audio readiness, and export/repair actions. The UI is intentionally one level deeper than normal game actions.

## Presentation Modules

The launcher presentation is split into focused modules:

- `chrome.rs`: top navigation and favorites/last-played strip.
- `game_page.rs`: Diablo III hero, primary action, compact status, overflow actions.
- `downloads.rs`: persistent activity/download tray and expanded drawer.
- `notifications.rs`: notification drawer.
- `diagnostics.rs`: compact advanced status helpers used by diagnostics/detail surfaces.
- existing `account_page.rs` and `battlenet_page.rs`: redesigned in place around the new hierarchy.
- existing `theme.rs`, `widgets.rs`, `ui_motion.rs`: shared visual primitives and motion.

`main.rs` remains the orchestration owner and maps module actions to existing backend methods.

## Error Handling

- Disabled primary actions use existing model/bridge readiness checks.
- Battle.net host failures fall back through the existing companion-mode host behavior.
- Install/update/verify/index failures surface as player-facing error state plus a path to diagnostics/repair.
- UI module actions are pure enums; backend side effects stay in `LauncherApp` methods.

## Testing

Static/unit tests cover:
- top navigation action mapping,
- primary game action labels and lifecycle presentation,
- progressive disclosure (advanced technical labels absent from player-facing game status),
- downloads summary state,
- notification counting,
- account page password-capture prohibition,
- Battle.net host surface mode labels,
- search/page routing remaining stable.

The project release verifier continues to run formatting, workspace tests, strict Clippy (`-D warnings`), and release build on the user’s Arch host. The package-side verifier also checks the new module boundaries and semantic version identity.

## Release Policy

The release version is exactly `0.3.7`. No revision suffixes, release candidates, or duplicate canonical artifacts are permitted. One canonical ZIP and one canonical TAR.GZ may be produced for v0.3.7, plus a cumulative v0.3.6→v0.3.7 patch.
