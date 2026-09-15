# OpenSanctuary v0.3.3 Bridge Health & Self-Healing Design

## Goal

Make the Battle.net bridge recover from stale local metadata, moved installations, ordinary client/game exits, and repeated launch failures without deleting Battle.net, the Wine prefix, or Diablo III.

## Architecture

`sanctuary-battlenet` owns health evaluation, persisted recovery metadata, crash-loop protection, and bounded rediscovery. `sanctuary-launcher` receives typed health/recovery events and maps them into existing `BridgeState` and Activity surfaces. The GUI never edits bridge files directly; it invokes bridge repair/rediscovery through launcher orchestration.

## Health model

Add `BridgeHealthState` with `Healthy`, `Degraded`, `Recovering`, and `Broken`. `BridgeHealthReport` records component readiness for runner, Wine prefix, Battle.net launcher, Diablo install, Diablo executable, plus a summary and optional recovery detail.

Health evaluation is read-only. A complete validated discovery is Healthy. A discoverable prefix/client with missing or moved Diablo paths is Degraded. A missing runner/prefix/client is Broken. During an explicit recovery attempt the launcher presents Recovering while rediscovery is running.

## Recovery model

`recover_bridge(home, record)` first merges the saved record with bounded normal discovery. It rescans the saved prefix if the saved Diablo path is stale. A successful complete discovery produces a fresh `BridgeRecord`; incomplete recovery returns a typed report without overwriting the saved record.

Corrupt or unsupported TOML records are quarantined by renaming them to `battlenet.toml.invalid` before rediscovery. Repair & Rediscover clears/quarantines only OpenSanctuary metadata and never deletes the Wine prefix or game files.

## Failure protection

Add a `FailureTracker` with a three-failure threshold inside a 60-second window and a 30-second cooldown. Spawn failures update the tracker; successful launch resets it. While cooling down, the launcher refuses another automatic official-client launch and exposes a clear bridge-error message rather than spawning repeatedly.

## Launcher behavior

Startup evaluates persisted bridge health. Healthy goes directly to normal verification/readiness. Degraded triggers one bounded recovery attempt. Recovered game paths enter the existing `VERIFYING -> INDEXING -> READY` pipeline. Broken stays actionable as `NEEDS SETUP` or `BRIDGE ERROR`.

The Diablo details column gains a compact Bridge Health block: overall health, Battle.net, Agent, Diablo III, Runner, Index, and the last recovery/error detail. Settings replaces the old Repair Bridge wording with `REPAIR & REDISCOVER`.

## Safety boundaries

No Blizzard credentials are read or stored. No Battle.net, Wine-prefix, or Diablo III files are deleted. Wine-specific code remains isolated to `crates/sanctuary-battlenet`. Recovery searches only the existing bounded discovery roots/prefixes.

## Verification

Unit tests cover healthy/degraded/broken reports, stale-path recovery, corrupt-record quarantine, and crash-loop cooldown/reset. Launcher tests cover recovered install routing and bridge-health event application. The Arch verifier runs fmt, tests, Clippy with `-D warnings`, and release build.
