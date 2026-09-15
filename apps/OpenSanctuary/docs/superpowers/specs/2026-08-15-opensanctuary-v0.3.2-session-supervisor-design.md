# OpenSanctuary v0.3.2 Session Supervisor Design

## Goal

Make the Battle.net bridge resilient across launcher restarts, client/game restarts, and Diablo III updates while preserving the single OpenSanctuary install/update/play flow.

## Architecture

`sanctuary-battlenet` owns a lightweight session supervisor. It inspects Linux `/proc` process metadata without adding a process-enumeration dependency, detects Battle.net/Agent/Diablo III process presence, and fingerprints `.build.info` to recognize installation changes. The launcher periodically asks the supervisor for a snapshot and converts snapshot changes into typed bridge events.

The native engine remains isolated from Wine/Battle.net. The supervisor never reads credentials, automates login, or manipulates Blizzard files.

## Bridge Snapshot

A `SessionSnapshot` contains:

- `battlenet_running`
- `agent_running`
- `diablo_running`
- `install_changing`
- optional `.build.info` fingerprint

Process detection is case-insensitive and uses `/proc/<pid>/cmdline` plus `/proc/<pid>/comm`. Process-name matching recognizes Battle.net, Agent, and Diablo III executable names but does not kill or modify processes.

## Update Detection

The supervisor samples `.build.info` metadata/content fingerprint plus the presence of known temporary/update markers beneath the game directory. A changed fingerprint while Battle.net/Agent is active places the launcher in `UPDATING`. After the installation remains stable for consecutive supervisor samples, the launcher automatically starts verification. If verification reports a stale index, OpenSanctuary automatically re-indexes and returns to `READY`.

## Session Reconciliation

On startup and during periodic polling:

- existing Diablo III process -> `RUNNING`
- Diablo III exits -> revalidate -> `READY` or repair state
- Battle.net/Agent active while install changes -> `UPDATING`
- update settles -> `VERIFYING -> INDEXING -> READY`
- existing Battle.net process prevents duplicate launch where possible
- bridge failures remain recoverable through rediscovery

## Repair Bridge

A launcher action removes only OpenSanctuary's persisted Battle.net bridge record, refreshes discovery, and leaves Blizzard files/prefixes untouched.

## Safety and Boundaries

- No Blizzard credentials are collected or stored.
- No Blizzard files are bundled.
- No process injection, click automation, or undocumented authentication automation.
- Wine/process-specific logic stays within `sanctuary-battlenet` and bridge orchestration.
- Native render/assets/engine crates remain compatibility-layer free.

## Verification

Unit tests cover synthetic `/proc` snapshots, process classification, build fingerprint stability/change, stable-update settling, and supervisor state transitions. `scripts/verify.sh` continues enforcing the bridge boundary. The Arch-side `BUILD-ON-ARCH.sh` remains the authoritative Cargo/Clippy/release-build gate.
