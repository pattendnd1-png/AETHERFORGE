# OpenSanctuary v0.3.6 Battle.net Integration Design

## Goal

Make OpenSanctuary the visible front door for Battle.net account interaction, Diablo III installation/update supervision, and Diablo III launch while preserving Blizzard-controlled authentication and the existing self-healing bridge.

## Non-negotiable security boundary

OpenSanctuary does not store, log, inject, or automate raw Battle.net passwords, authenticator codes, recovery codes, security answers, or captcha responses. Authentication happens in Blizzard-controlled UI. OpenSanctuary may store non-secret account hints and bridge/session metadata. Optional Blizzard OAuth identity is supported only when a developer OAuth client is configured externally; no client secret is embedded in the source tree.

## Architecture

The existing `sanctuary-battlenet` crate remains the only compatibility-runtime boundary. v0.3.6 adds three focused units:

1. `account.rs` — safe account/profile hints and desktop-session status.
2. `window_host/` — platform-specific Battle.net surface hosting.
3. launcher Battle.net/account pages — the visible integrated workflow.

The existing discovery, health, persistence, supervisor, and launch APIs remain authoritative for Battle.net and Diablo III process/data state.

## Account flow

The launcher exposes a Battle.net account page with:

- optional email/account hint (non-secret), persisted under OpenSanctuary config;
- desktop client state derived from Battle.net/Agent supervision;
- `SIGN IN WITH BATTLE.NET`, which launches the Blizzard client/login surface;
- `MANAGE ACCOUNT`, which opens Blizzard's account-management URL;
- optional OAuth readiness display when `OPENSANCTUARY_BNET_CLIENT_ID` is configured;
- no password field.

The desktop client's remembered Blizzard session is the authoritative install/play session.

## Diablo III installation flow

The existing `INSTALL DIABLO III` action remains the primary entry point. v0.3.6 changes the presentation so install is initiated and monitored from the integrated Battle.net page:

`INSTALL -> Battle.net surface -> INSTALLING -> VERIFYING -> INDEXING -> READY -> PLAY`

OpenSanctuary never bundles Blizzard game content and does not rely on undocumented commands to auto-click Battle.net's Install button. Battle.net performs entitlement, destination selection, download, patch, and login interaction; OpenSanctuary supervises completion and automatically adopts/indexes the resulting installation.

## Launch flow

`PLAY` remains a single OpenSanctuary action.

- If the native engine is usable, OpenSanctuary launches it.
- If the native engine fails or is not ready, OpenSanctuary starts/reuses Battle.net/Agent and launches the installed official Diablo III client through the existing bridge.
- Duplicate Battle.net or Diablo III launch attempts remain blocked by the existing supervisor.

## Window integration

### X11

When the OpenSanctuary process itself is running under X11, the launcher may embed the Battle.net top-level X11 window into an OpenSanctuary-owned region. The host:

- discovers the OpenSanctuary top-level X11 window by `_NET_WM_PID`;
- discovers Battle.net by window title / WM_CLASS containing Battle.net identifiers;
- reparents the Battle.net window under the OpenSanctuary top-level window;
- moves/resizes it to the target egui rectangle;
- restores it to the root window when the host is released;
- treats all discovery/reparent failures as recoverable and falls back to companion mode.

### Wayland

OpenSanctuary does not claim arbitrary foreign-surface embedding on Wayland. It uses a managed companion workflow: Battle.net remains a separate compositor-managed surface while the integrated page displays its state and controls. The page clearly says that Blizzard interaction is open in the managed companion surface.

## UI

Add global `BATTLE.NET` navigation and dedicated `BattleNet` and `Account` pages.

The Battle.net page contains:

- Account status;
- Bridge Health status;
- Battle.net/Agent process state;
- Diablo III install/update/index state;
- primary INSTALL / UPDATE / PLAY action;
- `OPEN BATTLE.NET` / `SIGN IN` action;
- platform host-mode state;
- a large Battle.net host region on X11 or a managed-companion card on Wayland.

The account utility drawer links to the full Account page rather than pretending a disconnected local profile is a Blizzard account.

## Persistence

Create `~/.config/opensanctuary/account.toml` (or `$XDG_CONFIG_HOME/opensanctuary/account.toml`) containing only non-secret fields:

- schema version;
- optional email/account hint;
- optional last-known BattleTag if obtained through an approved OAuth identity flow;
- region preference.

No access token, password, authenticator value, or recovery secret is written to TOML.

## Error handling

- Missing Battle.net -> existing official-download flow.
- Missing runner/prefix -> existing bridge health/recovery state.
- X11 embedding failure -> release any partial host state and fall back to companion mode.
- Wayland -> companion mode by design.
- Battle.net window recreation -> host retries discovery on bounded periodic sync.
- Account file parse failure -> quarantine/ignore local non-secret account file and continue with an empty profile.

## Testing and release gates

Pure tests cover:

- account config path and round trip;
- account config contains no password/token fields;
- session-to-account-state mapping;
- host-mode detection from environment;
- X11 window-name matching predicates;
- launcher navigation/search routes for Battle.net and Account;
- account page never exposes a password field literal;
- existing bridge/session tests continue unchanged.

The Arch verifier remains authoritative for Rust/Clippy and will enforce that Wine/Windows-specific logic remains inside `sanctuary-battlenet`.
