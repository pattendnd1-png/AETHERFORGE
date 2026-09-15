# OpenSanctuary v0.1.0 Design

Date: 2026-08-14
Status: Approved

## 1. Goal

OpenSanctuary is a clean-room, open-source Linux-native launcher and engine foundation for legally obtained Diablo III installations. The project does not execute Battle.net.exe or Diablo III.exe and does not depend on Wine, Proton, DXVK, VKD3D, a Windows VM, or Windows runtime libraries.

The v0.1.0 release establishes the native launcher, installation discovery, CASC/content indexing boundary, renderer/audio/input bootstrap, diagnostics, packaging, and a native engine process that can be launched from the UI. Full Diablo III gameplay and official Battle.net multiplayer are explicitly outside v0.1.0.

## 2. Legal and Clean-Room Boundaries

The repository contains no Blizzard source code, executables, logos, game artwork, fonts, cinematics, audio, or other redistributed proprietary assets.

The UI may reproduce broad interaction patterns familiar from modern game launchers and Battle.net, but must use original OpenSanctuary branding, iconography, typography, colors, spacing, artwork, and implementation. No pixel-for-pixel copying of Blizzard UI resources is permitted.

Users point OpenSanctuary at their own legally obtained Diablo III installation. Asset support is implemented by independently documented formats and clean-room interoperability work.

Authentication must not collect or imitate a Battle.net username/password form. Any future Blizzard account integration must use a supported external authorization flow or remain disabled.

## 3. UX Direction

The launcher should feel immediately familiar to a Battle.net user while remaining visibly OpenSanctuary.

### Main layout

- Narrow vertical game rail on the far left.
- Diablo III/OpenSanctuary game tile as the first supported title.
- Large selected-game content surface occupying the center and right.
- Full-width original hero artwork/background treatment supplied by OpenSanctuary, never copied from Blizzard.
- Bottom-left primary Install / Play / Repair button.
- Adjacent version/channel selector and installation status.
- Top bar with OpenSanctuary wordmark, navigation, connectivity indicator, and settings/account controls.
- Collapsible downloads/activity tray along the bottom edge.
- Secondary content cards for installation health, native-runtime readiness, patch/index status, and recent project news.

### Visual character

- Dark graphite/chrome shell.
- Cool-blue emissive accents and subtle amber highlights for Diablo-specific state.
- Translucent layered panels, restrained bloom, thin separators, rounded 6-10 px geometry.
- Large cinematic hero area with a dark readability gradient.
- Original open-source icon set.
- Smooth 150-250 ms panel transitions and hover fades; animation must be optional for reduced-motion users.

### Window behavior

- Native resizable desktop window.
- Minimum target size: 1100x700.
- Designed primarily for 1440p and 1080p desktops.
- Persistent window size/position and selected game.
- Native desktop notifications for completed verification/indexing tasks when enabled.

## 4. Architecture

Rust workspace:

```text
OpenSanctuary/
├── Cargo.toml
├── crates/
│   ├── sanctuary-core/
│   ├── sanctuary-launcher/
│   ├── sanctuary-install/
│   ├── sanctuary-casc/
│   ├── sanctuary-assets/
│   ├── sanctuary-render/
│   ├── sanctuary-audio/
│   ├── sanctuary-input/
│   ├── sanctuary-engine/
│   └── sanctuary-diagnostics/
├── apps/
│   ├── launcher/
│   └── game/
├── packaging/arch/
├── assets/opensanctuary/
├── tests/
└── docs/
```

### sanctuary-core

Owns shared types, versioning, paths, install-state models, task progress events, configuration, and typed error definitions. It must not depend on UI, Vulkan, PipeWire, or platform-specific launcher code.

### sanctuary-install

Discovers user-selected and conventional Diablo III installation locations, reads installation metadata, verifies expected structure, and reports install health. It never launches Windows executables.

Install state:

```text
NotConfigured
Searching
FoundUnindexed
Indexing
Ready
NeedsRepair
UnsupportedBuild
Error
```

### sanctuary-casc

Provides a read-only content-store interface. v0.1.0 focuses on storage discovery, metadata parsing, index enumeration, path/hash lookup where independently supported, and integrity reporting. The crate exposes an abstract content-source API so the implementation can evolve without coupling the engine to archive internals.

### sanctuary-assets

Defines engine-facing asset handles and conversion boundaries. In v0.1.0 it may expose metadata/placeholders before complete D3 asset decoding exists.

### sanctuary-render

Native Linux rendering bootstrap using wgpu with Vulkan as the preferred backend. Owns adapter/device selection, swapchain/surface setup, capability reporting, frame timing, and a minimal engine scene proving native rendering works.

### sanctuary-audio

Native Linux audio bootstrap. PipeWire is preferred. The abstraction must allow a secondary Linux backend later without changing game logic.

### sanctuary-input

Keyboard, mouse, gamepad, focus, and window input abstraction. No Windows input APIs.

### sanctuary-engine

Native ELF game/runtime executable. v0.1.0 launches a diagnostic Sanctuary scene/window, receives an installation/content-source descriptor, initializes rendering/audio/input, reports runtime health, and exits cleanly.

### sanctuary-diagnostics

Collects GPU/Vulkan, PipeWire, filesystem, content-store, permissions, and installation-health information. Diagnostics can be exported as user-readable text/JSON without secrets.

### sanctuary-launcher

Owns UI presentation and orchestration only. Long-running install scans/indexing/verification run asynchronously and publish typed progress events. The UI must remain responsive and support cancellation where safe.

## 5. Launcher Data Flow

```text
User starts launcher
  -> load config
  -> detect native system capabilities
  -> inspect configured D3 installation
  -> determine InstallState
  -> render launcher shell

User chooses installation
  -> sanctuary-install validates directory
  -> sanctuary-casc probes content store
  -> background index/verification task begins
  -> progress events update download/activity tray
  -> state becomes Ready or NeedsRepair/UnsupportedBuild

User presses Play
  -> launcher validates Ready state
  -> writes a short-lived native launch descriptor
  -> spawns sanctuary-engine ELF
  -> engine initializes Vulkan + audio + input
  -> engine opens diagnostic/native scene
  -> launcher tracks child process and exit status
```

No stage invokes a Windows executable.

## 6. v0.1.0 UI Screens

### Library / Diablo III

Primary Battle.net-inspired shell. Shows selected installation, readiness, hero treatment, Install/Locate/Play/Repair state, version metadata, and native-runtime status.

### Downloads / Activity

Shows scan, index, verification, and repair tasks with progress, throughput when meaningful, elapsed work, cancellation support, and errors.

### Settings

- installation path
- rendering adapter/backend information
- audio device information
- reduced motion
- launcher startup preference
- diagnostics/export
- update channel placeholder for future OpenSanctuary releases

### Diagnostics

Readable status cards and copy/export action for Linux distribution, kernel, GPU, Vulkan adapter, PipeWire status, content path, content health, OpenSanctuary version, and error trace IDs.

## 7. Error Handling

Errors are typed and user-facing messages are actionable.

Examples:

- InstallationNotFound: offer Locate.
- ContentStoreUnreadable: show path and permission guidance.
- UnsupportedBuild: preserve the install and disable Play rather than attempting unknown parsing.
- VulkanUnavailable: show detected adapters and prevent engine launch.
- PipeWireUnavailable: allow launcher operation; mark audio runtime unavailable.
- IndexCorrupt: offer Re-index.
- EngineCrash: capture child exit status and point to diagnostics.

The launcher must never delete or rewrite user game data automatically in v0.1.0.

## 8. Persistence

Configuration lives under XDG paths:

- `$XDG_CONFIG_HOME/opensanctuary/config.toml`
- `$XDG_CACHE_HOME/opensanctuary/`
- `$XDG_STATE_HOME/opensanctuary/`

No credentials are stored by v0.1.0.

Index/cache files are disposable and can be regenerated from the user's installation.

## 9. Testing

### Unit tests

- install-state transitions
- path/config serialization
- metadata parsing with synthetic fixtures
- content-source lookup behavior using generated test archives/fixtures
- typed error mapping
- launch descriptor validation

### Integration tests

- empty system / no installation
- user-selected synthetic installation
- corrupted synthetic index
- unsupported synthetic build
- renderer bootstrap where CI graphics permits
- launcher -> engine child-process lifecycle using test mode

### UI tests

- state-driven primary action: Locate / Index / Play / Repair
- game rail selection
- progress/activity tray
- settings persistence
- reduced-motion behavior

No Blizzard proprietary files are committed as test fixtures.

## 10. Packaging

Primary development target: Arch Linux.

Artifacts:

- `opensanctuary-launcher`
- `opensanctuary-engine`
- `opensanctuary-diagnostics`
- Arch PKGBUILD/package script
- desktop entry and original OpenSanctuary icons

The package declares native Linux dependencies only and must not depend on wine, proton, dxvk, vkd3d, winetricks, bottles, lutris, or a Windows runtime.

## 11. Definition of Done for v0.1.0

v0.1.0 is complete when:

1. The project builds as native Linux ELF binaries on Arch Linux.
2. The launcher presents the approved Battle.net-inspired OpenSanctuary shell.
3. A user can locate a Diablo III installation without invoking its Windows binaries.
4. The launcher can probe/index supported content metadata through sanctuary-casc.
5. Installation health and unsupported states are clearly surfaced.
6. Native Vulkan and PipeWire capability checks are displayed.
7. Play is enabled only when the installation/content/runtime state is Ready.
8. Play launches sanctuary-engine, not Diablo III.exe.
9. sanctuary-engine proves native Vulkan rendering and Linux input/audio initialization.
10. Diagnostics can be exported.
11. An Arch package can be built and installed.
12. Automated tests pass without requiring Blizzard assets.

## 12. Explicit Non-Goals for v0.1.0

- Full Diablo III gameplay.
- Official Battle.net multiplayer compatibility.
- Reimplementation of Blizzard authentication.
- Downloading proprietary Diablo III content from unofficial sources.
- Executing or embedding Battle.net.exe or Diablo III.exe.
- Wine/Proton/DXVK/VKD3D integration.
- Redistribution of Blizzard assets.
- Pixel-identical reproduction of the Battle.net launcher.

## 13. Follow-On Releases

v0.2.x: broader asset decoding and native asset viewer.

v0.3.x: native world/rendering pipeline and D3-style camera/scene reconstruction from user-owned content.

v0.4.x: actor/gameplay runtime foundations.

v0.5.x: local playable clean-room runtime slices.

Networking remains isolated and is only implemented where lawful, independently understood, and technically supportable.
