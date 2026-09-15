# OpenSanctuary v0.3.1 Battle.net Bridge State Machine Design

## Goal

Make the v0.3.0 Battle.net bridge durable across launcher restarts and resilient to moved installs, missing runners, transient process failures, and normal game exits. OpenSanctuary remains the primary user-facing launcher and keeps native launch preferred while retaining the official-client fallback.

## Boundaries

- Do not store, inspect, proxy, or automate Battle.net credentials or 2FA.
- Do not bundle or redistribute Blizzard executables or Diablo III game data.
- Keep all Wine/Windows-client process code inside `sanctuary-battlenet`.
- Persist only filesystem/process-discovery metadata: runner, prefix, Battle.net launcher, Diablo install root, and Diablo executable.
- Revalidate every persisted path before reuse; stale records never count as ready.
- Preserve v0.3.0 probe/index/native-engine behavior.

## Persistence

`sanctuary-battlenet` owns a versioned `BridgeRecord` serialized as TOML. The default path is `~/.config/opensanctuary/battlenet.toml` or `$XDG_CONFIG_HOME/opensanctuary/battlenet.toml`.

The record stores:

- schema version,
- Wine runner path,
- Wine prefix,
- Battle.net launcher path,
- Diablo III install root,
- Diablo III executable path.

Writes are atomic through a sibling temporary file followed by rename. Loading a missing file returns no record. Decode errors and I/O errors are explicit `BridgeError` variants. `BridgeRecord::validated()` returns a `BridgeDiscovery` only when the saved files/directories still exist and the Diablo install still has `.build.info` and `Data/`.

## State Machine

Add `BridgeState` with explicit lifecycle values:

- `MissingClient`
- `NeedsSetup`
- `Installing`
- `Updating`
- `Verifying`
- `Indexing`
- `Ready`
- `Starting`
- `Running`
- `BrokenInstall`
- `Error`

The launcher model owns the current state and a short state detail. Bridge events transition the model; the egui app no longer derives all official-client status from `official_install_active` / `official_game_active` alone.

## Startup/Revalidation

At startup:

1. Load the saved bridge record.
2. Validate it.
3. If valid, use it immediately and probe the saved Diablo install when no separate config install is available.
4. If invalid, discard it from active memory and perform bounded normal discovery.
5. If discovery finds a valid bridge/install, persist the refreshed record.

A moved or deleted install therefore falls back to discovery instead of leaving PLAY pointed at dead paths.

## Launch Reliability

Official launch uses bounded retry only for process spawn failures:

- Battle.net spawn: up to 3 attempts with short delays.
- Diablo III spawn: up to 3 attempts after Battle.net startup.
- Missing files/prefix/runner are not retried; they force revalidation/rediscovery.

`OfficialGameStarted` moves the state to `Running`. A clean or non-clean process exit returns the bridge to `Ready` when the persisted installation still validates; otherwise it becomes `BrokenInstall`.

Native engine remains first choice. If native spawn fails, the existing official fallback is attempted using the validated saved/discovered bridge.

## Install/Update States

`Installing` is entered when OpenSanctuary launches/watches Battle.net for a missing Diablo III installation. `Verifying` is entered when the install is detected and probe begins. `Indexing` tracks the existing index worker. `Ready` requires an accepted local install plus indexed content.

`Updating` is reserved for a detected Battle.net-managed install whose metadata is temporarily unstable during revalidation. v0.3.1 does not automate patch controls; it reports the state and opens Battle.net on user action.

## UI

The Diablo control column shows dedicated rows for Game, Battle.net, Install, Content, and Runtime. The primary button displays `INSTALL DIABLO III`, `VERIFYING`, `INDEXING`, `STARTING...`, `RUNNING`, or `PLAY` from state/action. While Running, PLAY is disabled. On exit it returns to PLAY if validation succeeds.

## Testing

`sanctuary-battlenet` fixture tests cover TOML round-trip, missing-record default, valid-record validation, stale path rejection, XDG/default config paths, and discovery conversion. Pure retry policy helpers are tested without spawning Wine.

`sanctuary-launcher` tests cover state transitions for client launch, install detection, indexing, game start/exit, failure, and recovery-to-ready. The existing Rust 1.97 Arch verifier remains the authoritative compile/Clippy gate.
