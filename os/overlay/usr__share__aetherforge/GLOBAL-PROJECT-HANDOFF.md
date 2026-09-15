# AetherForge 10.2.7 — Global Project Handoff

## Canonical ecosystem version policy

Effective with AetherForge 10.2.7, the AetherForge OS and every active first-party application share one synchronized ecosystem version.

Whichever first-party component is changed first triggers the next full ecosystem version. The trigger component may be the OS, Aether Terminal, Control Center, OpenDeck, ForgeHX, ReForge Logitech, STO Linux Command, OpenSanctuary, Aether OpenAI, or any other active first-party AetherForge component.

Once the next version is claimed, ALL active first-party components and the OS synchronize to that exact version in the same canonical release.

No revision suffixes, no r1/r2 labels, no RC labels, no "corrected" suffixes, and no reuse of a version number for different bytes.

## Current synchronized release

AetherForge ecosystem version: 10.2.7

Active identities:
- AetherForge OS: 10.2.7
- Aether Terminal: 10.2.7
- AetherForge Control Center: 10.2.7
- OpenDeck: 10.2.7
- ForgeHX: 10.2.7
- ReForge Logitech: 10.2.7
- STO Linux Command: 10.2.7
- OpenSanctuary: 10.2.7
- Aether OpenAI: 10.2.7

Older component version numbers may remain only as implementation/provenance lineage. They are not active release identities.

## Global version lock

A release is canonical only if:
- OS release identity == global ecosystem version
- every active first-party app release identity == global ecosystem version
- every first-party package/installer release identity == global ecosystem version
- release manifest == global ecosystem version
- status/diagnostic tooling == global ecosystem version
- rollback metadata == global ecosystem version
- per-app RELEASE.json records == global ecosystem version

Any mismatch is a release failure.

## Global README authority

Beginning with 10.2.1, all ecosystem changes are recorded in one canonical append-only file:

/usr/share/aetherforge/GLOBAL-README.md

Every synchronized release must append exactly one complete version section covering:
- features
- fixes
- QoL changes
- QA/verification changes
- migrations
- cleanup/removals
- packaging/release changes
- rollback/status behavior
- components that changed functionally
- components that changed only because of global version synchronization

A release that does not update the global README fails qualification.

## Aether Terminal selection contract retained in 10.2.5

Aether Terminal selection is history-relative rather than viewport-relative.

Required behavior:
- selected text stays highlighted while the terminal viewport is scrolled
- text that scrolls off-screen remains part of the selection
- scrolling back to that text restores the visible highlight
- edge-drag autoscroll extends the same selection
- wheel scrolling, PageUp/PageDown, Home/End history movement, scrollbar movement, and jump-to-live must not rewrite or clamp the selection
- no duplicate scrollback buffer
- the existing VT/PTY history remains authoritative
- persistent terminal checkpoints store history-relative selection coordinates

## Existing terminal behavior retained

- independent terminal-window lifetimes
- clicking any part of a terminal window activates the whole window
- Ctrl+C clears the visible terminal and interrupts the active PTY command
- Ctrl+Shift+C copies
- selection autoscroll
- interactive AETHER menu
- command palette
- persistent settings and rendering controls
- persistent session broker architecture
- transactional update/restart architecture
- state-preserving update rollback

## Cross-project continuation rule

All previous project work remains valid and must be continued from the newest available implementation lineage.

When any project is changed:
1. choose the next full ecosystem version
2. apply that version to the OS and every active first-party component
3. preserve unchanged components as version-sync-only updates
4. append the complete change record to GLOBAL-README.md
5. update all per-app RELEASE.json records and global authority metadata
6. run fail-closed version-lock qualification
7. produce one canonical artifact per component/version/architecture
8. preserve rollback and migration behavior

This handoff supersedes older per-project independent-version policies for active first-party AetherForge projects.

## Single global control point

This conversation is the single canonical ecosystem control point for all OS and first-party project changes. All OS and first-party project changes are routed through the same project registry, global version trigger, global README, file/source authority, and strict Rust/Clippy qualification policy. Historical chat/library artifacts remain indexed as lineage and recovery evidence; the newest canonical source donor is the active qualification target.

## Explicit exclusions

ForgeCord, ForgeStream, StreamForge, and the standalone Rust OBS ReForge project (`rust-obs-reforge`) are excluded from the active AetherForge first-party synchronization set. The aliases Rust OBS ReForge, OBS ReForge, and ReForge OBS all resolve to that excluded standalone project. Their historical artifacts are lineage-only. They are not version-synced, Clippy-routed, packaged, installed, promoted as canonical donors, or automatically reactivated by file discovery. OpenDeck OBS Studio integration remains active and is not part of the standalone Rust OBS ReForge exclusion.



## Rust-only executable control plane

Effective with the 10.2.2 Rust-control-plane migration and retained in 10.2.4, canonical first-party ecosystem verification, qualification, update, rollback, status, and routing logic is Rust. Python/Bash implementations are not canonical active release logic. All functional Rust changes pass the shared strict global Clippy gate on a protected tree. Legacy donor artifacts remain lineage only. OpenDeck is version-synchronized but its native Rust UI port remains blocked until complete source recovery.

## 10.2.4 control-point rule

All future OS and active first-party work may be initiated from the canonical ecosystem chat/control point. Whichever active project changes first claims the next full ecosystem version; every other active identity and the OS synchronize in the same release. Global QoL/QA and complete release changes are appended to `GLOBAL-README.md`.

Rust executable release/update/QA logic is canonical from 10.2.4 onward. ForgeCord, ForgeStream, StreamForge, and standalone Rust OBS ReForge are excluded and may appear only as historical lineage. OpenDeck OBS Studio integration remains active.


## 10.2.4 bootstrap correction

The global Rust bootstrap now locates the extracted canonical package by required package markers instead of treating `target/release` as the package root. This fixes the duplicated `target/release/target/release/aetherforge-control` failure. All active OS/app identities synchronize to 10.2.4; Terminal and unchanged apps retain their previous functionality as version-sync-only components.


## 10.2.4 Rust qualification-state correction

The global Rust qualification pipeline no longer uses the ambient `AETHERFORGE_RUST_SOURCE_PROTECTED` environment variable. Protected-copy authority is explicit in the typed qualification request. This prevents bootstrap state from leaking into `cargo test` and changing test outcomes. Qualification failures now print a bounded Cargo stdout/stderr excerpt directly while retaining the durable project-routed log. All active OS/app identities synchronize to 10.2.4; Terminal and other unchanged apps are version-sync-only.


## 10.2.5 Terminal installer contract correction

Aether Terminal triggers ecosystem 10.2.5. The native Terminal installer now validates the canonical history-relative selection regression `terminal_selection_stays_bound_to_history_across_scrollback` and no longer requires the retired `terminal_selection_is_revision_bound` symbol. The native update-manager semantic-version test is synchronized to `(10, 2, 5)`. The ecosystem Rust verifier inspects the embedded Terminal installer/UI/update-manager sources before install so stale selection or current-version test contracts fail before source migration. All other active first-party components and AetherForge OS are version-sync-only in this release.


## 10.2.6 Terminal protected-tree Clippy correction

Aether Terminal triggers ecosystem 10.2.6. Its rollback-protected `cargo clippy --fix` phase now includes `--allow-no-vcs` together with `--allow-dirty` and `--allow-staged`, matching the global Rust qualification policy for disposable/no-VCS trees. The Terminal package verifier and ecosystem Rust verifier both require this contract before source migration. The native update-manager semantic-version regression is synchronized to `(10, 2, 6)`. History-relative selection remains unchanged and required. All other active first-party components and AetherForge OS are version-sync-only for this release.


## 10.2.7 Terminal strict-Clippy correction

Aether Terminal triggers ecosystem 10.2.7. Rust 1.98 strict Clippy exposed `clippy::field-reassign-with-default` in the settings profile implementation. The production preset and the two matching settings test fixtures now use direct struct initialization. Package verification rejects the detected reassignment pattern before migration. The 10.2.6 no-VCS autofix correction, history-relative selection, transactional rollback, and all prior cumulative behavior remain required. All other active first-party components and AetherForge OS are version-sync-only for this release.
