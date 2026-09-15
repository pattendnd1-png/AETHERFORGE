# ForgeClean v1.0.5

## v1.0.5 — Orbital Sync Monitor Live

- Makes the NetworkCard-style Orbital Sync monitor a real navigable ForgeClean page.
- Keeps the one-second graph loop read-only: status/tail/`/proc/net/dev` only.
- Keeps the orbital worker as the single scheduling and upload authority.
- Derives GUI version from the Cargo package version to prevent version drift.


ForgeClean is the persistent Rust sorting, cleaning, deletion, build-routing and ColdPack utility for AetherForge. v1.0.1 is the first canonical stable 1.x promotion of the fully host-verified v0.6.4 stack: native DragonGlass frontend, persistent organizer, build-aware routing, cleanup/offload safety, ColdPack/GC, and all compatibility fixes through v0.6.4.

## Native ForgeClean GUI

v1.0.1 ships two binaries over the same Rust core:

- `forgeclean` — CLI, persistent organizer, cleanup/offload engine, build router and ColdPack/GC backend.
- `forgeclean-gui` — native eframe/egui frontend for visibility, settings and user-triggered actions.

Closing the GUI does **not** stop ForgeClean. `forgeclean-organizer.service` remains the always-active `systemd --user` organizer and continues sorting old and newly downloaded project files.

Launch from the desktop menu as **ForgeClean**, or run:

```bash
forgeclean-gui
```

The GUI includes Dashboard, Projects, Downloads/Inbox, Cleanup, ColdPack, Storage, Activity, Settings and About/Diagnostics pages. Long-running actions are dispatched through a background worker so the render loop stays responsive.

## DragonGlass theme

The GUI uses the AetherForge DragonGlass theme/schema with a **90% transparent / 10% smoky-glassy** treatment throughout the main window, titlebar, navigation rail, cards, menus, dialogs, confirmation prompts, controls and status surfaces.

- dark smoked glass
- indigo/violet/darker-blue emphasis
- pale near-white/light-lavender text
- compact custom window chrome
- fully rounded window controls
- rounded cards, panels and controls
- minimal translucent borders

`DRAGONGLASS_WINDOW_ALPHA=26` is the canonical 8-bit background alpha target (approximately 10% opacity).

## GUI settings

Settings are stored atomically at:

```text
~/.config/forgeclean/gui-settings.json
```

The GUI controls:

- persistent organizer enabled/startup state
- file stability delay
- polling interval
- compatibility/legacy symlink visibility
- automatic ColdPack maintenance
- maintenance interval
- notifications
- diagnostics visibility
- logging level

The build-aware routing policy is intentionally authoritative: project/build identity remains higher priority than generic file type, continuous build reconciliation remains active, and generic categories remain fallback storage only when ForgeClean cannot safely identify a project/build.

External-drive behavior remains safety-locked: no detected external drive means `AUTO_CLEAN`; an eligible detected external drive means `AUTO_OFFLOAD`; detection failure blocks offload.

Saving runtime settings restarts or enables/disables the persistent organizer as needed. Explicit CLI timing flags still override persisted defaults for that invocation.

## Build-aware continuous reconciliation

ForgeClean continuously handles **old and new files for all recognizable projects**. Project/version identity is authoritative before generic file type.

Recognized related artifacts are grouped together:

```text
~/Downloads/ForgeClean/Projects/<Project>/Builds/<Version>/
```

A build folder can contain:

- source ZIP/tar archives
- HIT-IT/install/build scripts
- packages/installers
- checksum files
- verification TXT files
- logs and release metadata
- other build-support artifacts

ForgeClean performs a migration sweep over existing live files in legacy generic buckets and project `Downloads/` / `Releases/` areas, then repeats reconciliation on every watcher cycle for newly arriving files. Guarded compatibility symlinks preserve old paths where safe.

Incomplete `.part`, `.partial`, and `.tmp` files are never promoted into canonical build bundles.

## Canonical layout

```text
~/Downloads/ForgeClean/
├── Projects/<Project>/
│   ├── Active/
│   ├── Builds/<Version>/
│   ├── Downloads/      # legacy live staging; continuously reconciled
│   ├── Releases/       # legacy live staging; continuously reconciled
│   ├── Restored/
│   └── Cold/*.fcoldpack
├── ColdStorage/
│   ├── .coldpack-store/
│   │   ├── maintenance.lock
│   │   ├── objects/<2hex>/<sha256>.zst
│   │   └── quarantine/<unix-seconds>/<2hex>/<sha256>.zst
│   └── <Category>/*.fcoldpack
├── Restored/<Category>/
└── registry.tsv
```

Active source trees are never ColdPack-compressed.

## ColdPack and GC

ColdPack uses deterministic content-defined chunking (64 KiB minimum, about 256 KiB average, 1 MiB maximum), SHA-256 content addressing and Zstandard level 19 compression. Identical chunks across files, projects and archive generations are stored once. Compression ratio is data-dependent; ForgeClean does not promise a fixed 10–15% ratio.

Reference-aware GC uses mark-and-sweep with a fixed **7-day quarantine**. Active objects are never directly deleted by GC. A successful apply audits authoritative manifests first, moves orphan objects into quarantine, and permanently purges only objects that remain unreferenced after the quarantine period.

Archive creation and GC apply share the ColdPack maintenance lock. Restore/status/audit use compatible shared locking. Corrupt or unreadable authoritative metadata blocks destructive GC operations.

```bash
forgeclean coldpack-status
forgeclean coldpack-audit
forgeclean coldpack-gc --preview
forgeclean coldpack-gc --apply
```

Legacy `*.fcold.tar.zst` + `.sha256` restore compatibility remains supported.

## Build redirection

```bash
forgeclean resolve-project ForgeHX
forgeclean build ForgeHX -- cargo build --release
```

Registered Active trees and guarded legacy aliases keep existing build workflows resolving after ForgeClean reorganizes files.

## Package cleanup / conditional external offload

- no external drive detected: `AUTO_CLEAN`
- eligible external drive detected: `AUTO_OFFLOAD`
- external detection failure: fail closed
- destructive automatic mode requires explicit `--yes`
- offload commit: copy → SHA-256 verify → fsync → source identity reverify → unlink
- deletion is direct unlink with no desktop Trash
- Pacman package retention keeps the newest two versions and pins installed versions

`scan-system` remains preview-only against `/var/cache/pacman/pkg` and writes:

```text
~/Downloads/ForgeClean-v1.0.1-PACMAN-BATCH.txt
```

## Persistent service

The installed service uses:

```text
ExecStart=%h/.local/bin/forgeclean watch
```

With no explicit timing flags, the daemon reads persisted GUI/runtime settings. Default behavior remains a 30-second stability delay and 2-second polling interval. ColdPack maintenance defaults to once every 24 hours.

## Safety model

- permanent deletion remains direct unlink / no Trash
- cleanup preview remains available before user-triggered purge
- external offload only activates after actual external-drive detection
- copy/archive commit requires verification, fsync and source revalidation
- symlink and path-escape protections remain mandatory
- Active source trees are never cold-compressed
- ColdPack GC quarantines orphans before permanent purge
- corrupt/missing authoritative metadata blocks destructive paths
- GUI actions call the authoritative ForgeClean CLI/core instead of reimplementing destructive logic


## Stable baseline

**v1.0.1** is the canonical stable ForgeClean baseline promoted from the fully host-verified v0.6.4 release. Future changes increment normally from v1.0.1; no 0.x release becomes canonical again.

## Verification handoff

`ForgeClean-v1.0.1-HIT-IT.sh` precreates:

```text
~/Downloads/ForgeClean-v1.0.1-VERIFY.txt
```

before source validation, extraction, regressions, Cargo, GUI build, installation or service restart. Its EXIT trap records the final launcher result and TXT path even on early failure.

The host verification gate builds and tests both `forgeclean` and `forgeclean-gui`, runs Rust 1.98 Clippy with warnings denied, executes the GUI self-test, verifies inherited sorting/cleanup/ColdPack/GC behavior, installs the desktop entry and confirms the persistent organizer remains active.
