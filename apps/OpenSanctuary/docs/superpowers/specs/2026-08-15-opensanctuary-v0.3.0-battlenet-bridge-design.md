# OpenSanctuary v0.3.0 Battle.net Bootstrap Bridge Design

## Goal

Make OpenSanctuary the primary launcher for Diablo III while using Blizzard's official Battle.net desktop app only for legitimate acquisition, patching, and account authentication. After Diablo III is installed, OpenSanctuary detects and indexes the local installation and exposes one PLAY button that prefers the native OpenSanctuary engine and can fall back to the official Windows client when explicitly configured/available.

## Boundaries

- Do not bundle, mirror, scrape, or redistribute Blizzard executables or game data.
- Do not capture, proxy, store, or automate Battle.net credentials or 2FA.
- Do not automate clicks inside Battle.net.
- The official installer handoff opens `https://download.battle.net/en-us/desktop` with the system URL opener when no local Battle.net client can be launched.
- A Windows compatibility runner is isolated to the Battle.net/official-client bridge. The OpenSanctuary native engine remains Wine/Proton-independent.
- Existing v0.2.5 content indexing, diagnostics, settings, and native engine behavior remain intact.

## Architecture

Add `crates/sanctuary-battlenet` as an external-client orchestration boundary. It discovers common Battle.net Wine prefixes, a compatible Wine runner, Battle.net executables, and Diablo III executables; builds spawn requests; launches Battle.net or the official download page; launches the official Diablo III client when fallback is selected; and watches a bounded set of likely install roots for a valid `.build.info` + `Data/` installation.

`crates/sanctuary-launcher` owns asynchronous orchestration. It translates Battle.net bridge results into `LauncherEvent` values and Task activity without making the UI directly manage processes or filesystem polling.

`apps/launcher` exposes INSTALL DIABLO III for an unconfigured game, launches the bridge, displays official-client status, automatically probes a detected Diablo III install, and keeps the existing native PLAY route. If native readiness is unavailable but an official executable plus Battle.net bridge are available, the gear/settings surface can select official fallback and PLAY launches Battle.net first, then the game through the same runner/prefix.

## Discovery

Battle.net discovery checks:

1. `OPENSANCTUARY_BATTLENET_EXE` and `OPENSANCTUARY_WINE` overrides.
2. `WINEPREFIX` when present.
3. `~/.wine`.
4. common Lutris-style `~/Games/battlenet` and `~/Games/battle-net` prefixes.
5. Bottles prefixes under `~/.local/share/bottles/bottles/*`.
6. Flatpak Bottles prefixes under `~/.var/app/com.usebottles.bottles/data/bottles/bottles/*`.

Wine runner discovery checks `OPENSANCTUARY_WINE`, then PATH for `wine`, then `wine64`.

Diablo III candidate roots include OpenSanctuary's current candidates plus Diablo III locations inside discovered prefixes. A candidate is accepted only when `sanctuary-install::probe_install` succeeds.

## Install Flow

1. User clicks INSTALL DIABLO III.
2. OpenSanctuary attempts to discover and launch Battle.net.
3. If no launchable Battle.net client exists, OpenSanctuary opens the official Battle.net desktop download page and reports that the installer must be completed first.
4. When Battle.net launches, OpenSanctuary starts a watcher thread and reports `Battle.net running — install Diablo III there`.
5. The watcher periodically checks bounded candidate directories, without recursively scanning the full home directory.
6. On detection, OpenSanctuary saves the install path and triggers the existing probe/index flow.
7. The primary button naturally advances from INSTALL to INDEX CONTENT to PLAY through existing install states.

## Launch Flow

Native launch remains preferred. When `LauncherModel::play_ready()` is true, PLAY uses `opensanctuary-engine` exactly as v0.2.5 does.

Official fallback is a separate mode. It requires a discovered Wine runner, Battle.net executable/prefix, and `Diablo III.exe`. OpenSanctuary launches Battle.net first, waits briefly for agent startup, then launches Diablo III using the same `WINEPREFIX`. It does not pass credentials or use undocumented Battle.net command-line product launch commands.

## Error Handling

- Missing Battle.net executable: open official download page and keep INSTALL available.
- Missing Wine runner: display an actionable bridge error; never claim Battle.net launched.
- Battle.net spawn failure: record a failed activity item with the OS error.
- Install watcher timeout: stop watching after a long bounded interval and keep Locate/Install available.
- Diablo executable missing: native engine may still run if indexed; official fallback reports that only data assets were found.
- Process exits: record success/failure in launcher Activity without deleting the indexed install.

## Testing

Pure discovery/path functions receive explicit home/PATH/environment inputs so tests use temporary fixtures. Tests cover prefix discovery, runner search, Battle.net executable discovery, Diablo executable discovery, download-page command construction, and preservation of existing config defaults. Launcher tests cover INSTALL as the primary action for an unconfigured game and translation of bridge events into model messages/activity.
