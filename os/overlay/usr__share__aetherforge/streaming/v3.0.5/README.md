# AetherForge OBS / Twitch / Stream Deck Package v3.0.5

## Root Loader Hotfix

v3.0.5 is a package-loading reliability release. It keeps the approved v3.0.1 artwork and animations; no new images were generated.

### What changed

- Installs media to a stable user data directory instead of leaving scene collections tied to the ZIP extraction folder.
- Detects both native Linux OBS and the official Flatpak layout.
- Installs to every detected OBS layout in `auto` mode, or accepts `--native`, `--flatpak`, or `--all`.
- Removes generated text placeholder sources from scene collections, eliminating placeholder transform/canvas offset problems.
- Keeps only full-canvas AetherForge background media in the imported scenes; add Game/Camera/Chat sources above the background after import.
- Adds launch scripts that open OBS with the exact matching profile and scene collection.
- Adds an installed-path diagnostic script.

## Install

Close OBS and extract the ZIP. The package now exposes the installer directly at the extraction root.

For a complete 1080p install, validation, and launch:

`./setup.sh 1080p`

For 1440p:

`./setup.sh 1440p`

The compatibility paths `scripts/install.sh`, `scripts/diagnose.sh`, and the launch scripts remain included.

### Manual OBS pairing

- Profile `AetherForge v3.0.5 1080p` + Scene Collection `AetherForge v3.0.5 1920x1080`
- Profile `AetherForge v3.0.5 1440p` + Scene Collection `AetherForge v3.0.5 2560x1440`

## Scene behavior

Starting Soon remains full-background only with no overlay frames. The warpfield remains removed. Gameplay, Full Camera, Just Chatting, BRB, Sleeping, Stream Ending, and Alerts retain the approved neon-cosmic layout.

## Stream Deck

Import `streamdeck/AetherForge-v3.0.5.streamDeckProfile`.
