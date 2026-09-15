# ForgeClean v0.6.0 GUI Design

## Goal
Add a native Rust graphical frontend to ForgeClean without duplicating or weakening the verified v0.5.2 engine. The GUI is a control and visibility layer over the existing Rust library, daemon, organizer, ColdPack, GC, cleanup, offload, registry, and build-aware routing systems.

## Architecture
ForgeClean v0.6.0 ships two user-facing binaries over one shared Rust core:

- `forgeclean` — CLI, daemon, persistent organizer, automation, cleanup and storage engine.
- `forgeclean-gui` — native desktop frontend.

Both use the same core library and authoritative state. The GUI never reimplements destructive or routing logic.

The persistent `systemd --user` organizer remains active when the GUI is closed.

## DragonGlass Theme
The GUI must match the AetherForge DragonGlass theme/schema.

- 90% transparency / 10% smoky-glassy visual treatment.
- Dark smoked translucent surfaces.
- Indigo / violet / darker-blue emphasis and status accents.
- Rounded panels, cards, menus, dialogs, popovers, tooltips, buttons and controls.
- Minimal borders.
- Compact window chrome.
- Fully rounded window controls.
- Pale near-white / light-lavender text and glyphs.
- Consistent AetherForge typography and iconography where available.
- No generic stock desktop styling when a DragonGlass treatment is possible.

Transparency applies consistently to:

- main window
- left navigation rail
- top/status bar
- project cards
- build cards
- settings panels
- dialogs
- confirmation prompts
- context menus
- tooltips
- notifications
- activity panels

## Primary Navigation

### Dashboard
Shows:
- organizer service status
- files sorted
- projects detected
- current build bundles
- reclaimable space
- space already recovered
- ColdPack dedup savings
- GC/quarantine summary
- internal/external storage status
- latest organizer activity

### Projects
Shows every recognized project.

Canonical project structure:

```text
Projects/<Project>/
├── Active/
├── Builds/
│   └── <Version>/
├── Releases/
├── Restored/
└── Cold/
```

Selecting a build shows all related artifacts together, including:
- source archives
- HIT-IT/install scripts
- packages/installers
- checksums
- verification files
- logs
- release metadata
- supporting build files

Actions:
- Open Folder
- Copy Path
- Resolve Legacy Path
- Run Build
- Run Installer
- Verify
- Archive
- Restore

### Downloads / Inbox
Shows:
- newly detected files
- files still waiting for stability
- incomplete `.part`, `.partial`, `.tmp` downloads
- unidentified files
- files awaiting project/version resolution
- manual reassignment controls

Incomplete downloads must never be promoted into canonical build bundles.

### Cleanup
Shows:
- cleanup candidates
- reclaimable bytes
- Pacman cache candidates
- stale partials
- preview before destructive cleanup
- exclusions
- execution results

Permanent purge remains explicit and previewable.

### ColdPack
Shows:
- object store status
- active manifests
- deduplication savings
- archive/restore controls
- audit status
- orphan-object preview
- quarantine contents
- GC preview/apply
- 7-day quarantine state

Corruption or unreadable authoritative metadata blocks destructive GC operations.

### Storage
Shows:
- internal storage
- detected external drives
- current `AUTO_CLEAN` / `AUTO_OFFLOAD` mode
- offload destination
- storage health/status

External offload activates only when an actual external drive is detected. Detection errors fail closed.

### Activity
Live chronological activity feed:

```text
file downloaded
→ stability confirmed
→ project/version identified
→ canonical build folder selected
→ file moved
→ compatibility path created
→ optional archive/offload action
```

### Settings
Settings include:
- organizer enabled/disabled state
- startup/service state
- file stability delay
- polling interval
- project identity rules
- build-aware grouping controls
- compatibility symlink behavior
- generic fallback categories
- external storage behavior
- ColdPack maintenance controls
- GC schedule/status
- notifications
- logging level
- diagnostics visibility

### About / Diagnostics
Shows:
- ForgeClean version
- GUI version
- daemon/service state
- organizer root
- ColdPack store path
- registry path
- verification status
- last maintenance run
- recent guard failures
- troubleshooting information

## Build-Aware Sorting Policy
Project identity and build identity always take precedence over generic file-type classification.

For recognized artifacts:

```text
project → version/build → related files
```

Generic categories are fallback only when ForgeClean cannot safely infer project/build identity.

Existing misplaced live files are continuously reconciled into canonical project/version build folders. New downloads are continuously reconciled as well.

Legacy paths remain usable through guarded compatibility symlinks where safe.

## Safety Rules
- Direct deletion remains no-Trash unlink behavior.
- Permanent deletion requires explicit user action where appropriate.
- Source identity/tree revalidation remains mandatory before destructive commit paths.
- Symlink/path-escape protections remain mandatory.
- Active source trees are never ColdPack-compressed.
- External-drive detection failure does not trigger offload.
- GC never directly unlinks active objects; orphan objects enter quarantine first.
- Corrupt/missing authoritative metadata blocks destructive actions.
- GUI must call existing core interfaces rather than duplicating safety logic.

## Native GUI Stack
Preferred stack:
- Rust 2024 edition
- `egui` / `eframe`
- existing ForgeClean core library
- persistent `systemd --user` daemon/service

The GUI should remain responsive while long-running operations execute by using background worker tasks/messages rather than blocking the render loop.

## Success Criteria
ForgeClean v0.6.0 is successful when:

1. `forgeclean` CLI/daemon behavior remains backward compatible.
2. `forgeclean-gui` launches as a native Rust application.
3. GUI reflects live organizer/service state.
4. Project/version/build bundles are browsable and actionable.
5. Settings persist across restarts.
6. Build-aware sorting remains continuously active when GUI is closed.
7. Destructive operations use the same existing safety gates as CLI/daemon operations.
8. DragonGlass treatment is consistently 90% transparent / 10% smoky-glassy throughout the UI.
9. Existing ColdPack, GC, offload, cleanup, build registry and compatibility-routing tests remain green.
10. New GUI/state/settings tests and host verification gates pass before v0.6.0 becomes canonical.
