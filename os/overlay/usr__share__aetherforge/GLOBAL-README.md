# AetherForge Global Release README

This is the canonical, append-only human-readable change record for the synchronized AetherForge ecosystem. Beginning with 10.2.1, every OS or first-party change must advance the single ecosystem version and append one complete release section here. Existing release sections are never rewritten, deleted, or replaced.

## 10.2.1

### Global version synchronization

- Established one synchronized ecosystem release identity: **10.2.1**.
- AetherForge OS and every active first-party application now share that same release version.
- The first changed OS/first-party component claims the next full semantic ecosystem version; all other first-party components and the OS synchronize to it in the same canonical release.
- Active first-party set: Aether Terminal, AetherForge Control Center, OpenDeck, ForgeHX, ReForge Logitech, STO Linux Command, OpenSanctuary, and Aether OpenAI.
- Revision suffixes, RC labels, corrected-copy identities, reused version numbers, and competing current versions are forbidden.
- Older app numbers remain only as implementation-lineage provenance, never as active release identities.

### Aether Terminal

- Replaced viewport-relative selection rows with **history-relative selection coordinates**.
- Highlighted text remains selected while scrolling with the mouse wheel, Shift+PageUp/PageDown, Ctrl+Shift+Home/End, terminal scrollbar, jump-to-live, and selection edge-autoscroll.
- Scrolling no longer rewrites, clamps, shifts, or revision-rebinds the selected range.
- Selected history that leaves the viewport remains selected; when it comes back into view, the highlight is painted again.
- Selection checkpoints advance to schema 2 and persist history-relative anchor/head coordinates across GUI restart/reconnect.
- Existing screen-relative checkpoints migrate using their saved scrollback offset.
- Preserves the 10.2.0 persistent PTY/session broker, transactional GUI update/reconnect, interactive menu/settings/rendering controls, Ctrl+C clear+interrupt behavior, Ctrl+Shift+C copy, independent terminal windows, and whole-window activation.

### First-party continuation

- AetherForge Control Center synchronizes to release 10.2.1 and continues from its latest qualified Control Center implementation lane.
- OpenDeck synchronizes to release 10.2.1 and continues from the 1.2.1 implementation lineage, including Stream Deck Plus, OBS/Twitch/Discord, Twitch login, and Elgato account/store work.
- ForgeHX synchronizes to release 10.2.1 and continues from its latest qualified always-on DSP/hardware-support lane, including SoloCast 2 and Pulsefire Haste Wireless work.
- ReForge Logitech synchronizes to release 10.2.1 and now incorporates the recovered **0.9.0 functional source baseline**: first-class G515 LIGHTSPEED TKL and Brio 100 adapters, richer capability/firmware metadata, safe selected-release fwupd installation with confirmation, and the preserved v0.8 universal fallback/control architecture.
- STO Linux Command synchronizes to release 10.2.1 and continues from its latest DragonGlass Arc/STO launcher lane.
- OpenSanctuary synchronizes to release 10.2.1 and continues from its latest Battle.net/Diablo launcher lane.
- Aether OpenAI synchronizes to release 10.2.1 as a built-in first-party capability.

### ReForge Logitech 0.9 lineage intake

- Recovered the complete ReForge Logitech 0.9.0 source archive from the cross-chat Library routing sweep and promoted it to the canonical implementation donor for ecosystem 10.2.1.
- Adds first-class G515 LIGHTSPEED TKL and Brio 100 device adapters above the existing universal `DeviceControl` path; no second hardware-write backend is introduced.
- Adds camera-oriented Brio 100 V4L2/PipeWire capability surfaces without inventing unsupported writable focus/zoom/pan/tilt controls.
- Adds richer fwupd device/release metadata and selected-release installation only for releases already advertised as safely installable, with explicit confirmation and no force/downgrade/reinstall/trust-bypass path.
- Hardware qualification remains separate from source support; real-device write/readback, reconnect, camera-provider, and firmware acceptance gates remain required.

### OS, packaging, QA, status, and rollback

- AetherForge OS release identity advances to 10.2.1 in the same transaction as the first-party version authority.
- Installs per-app canonical `VERSION` and `RELEASE.json` records at 10.2.1.
- Installs the global authority, release manifest, continuation ledger, and this global README under `/usr/share/aetherforge`.
- The global README is append-only: a new release must contain the existing installed README as an exact prefix before adding its new section; otherwise qualification fails.
- `aetherforge-version-status` verifies OS version, every first-party release record, and the current global README release entry.
- The terminal payload runs Cargo tests, strict Clippy, release build, rendering smoke, diagnostics, and terminal rollback gates on the host before the release is committed.
- Ecosystem identity, per-app release records, global README, global project handoff, terminal source/runtime, and workspace naming are rolled back if the transaction fails.

### Global project/chat handoff

- Adds `GLOBAL-PROJECT-HANDOFF.md` as the canonical portable continuation record for every AetherForge OS and first-party app project.
- Existing chat threads cannot be mutated from another chat, so the handoff is the authoritative cross-chat carry-forward mechanism: any continued OS/first-party project must consume the same 10.2.1 version lock, global README policy, terminal selection contract, and continuation rules.
- The installed authority path is `/usr/share/aetherforge/GLOBAL-PROJECT-HANDOFF.md`, with a release snapshot under `/usr/share/aetherforge/releases/10.2.1/GLOBAL-PROJECT-HANDOFF.md`.
- Future ecosystem releases update the handoff together with the global README and version authority so every project continuation receives the same release policy regardless of which component triggered the version bump.
### Global control plane

- This conversation is the single canonical ecosystem control point for all OS and first-party project changes.
- `PROJECT-REGISTRY.json` enumerates the OS plus every active first-party project and routes every Rust project through the same strict qualification policy.
- `GLOBAL-CONTROL-PLANE.json` makes the first changed project the global-version trigger, records all global QoL and QA, and fails the release if any synchronized identity or required authority is missing.

### Global QoL and QA

- Global QoL and QA changes are recorded in this chat and this append-only README for every release, including component-specific changes and version-sync-only components.
- Release status is fail-closed: the version lock, project registry, control plane, global README, global handoff, file/source routing, and Rust qualification policy must all be installed and valid.

### Global Rust / Clippy qualification

- Standardized Rust qualification across all first-party Rust projects and OS Rust components.
- The canonical gate performs format normalization, attempts safe machine-applicable Clippy fixes on a disposable or rollback-protected source tree, reformats, runs `cargo fmt --all -- --check`, `cargo check --workspace --all-targets --all-features`, `cargo test --workspace --all-targets --all-features`, and finally `cargo clippy --workspace --all-targets --all-features -- -D warnings` before the release build.
- Clippy warnings are never accepted as release output; unfixable warnings fail closed before install and the qualification log is routed to the owning project.

### Global file/source routing

- Indexed all recoverable OS and first-party chat/library artifacts under a project-aware routing authority instead of duplicating historical packages into the active release.
- The newest canonical donor available for each project is embedded when recoverable; older patches, installers, designs, checksums, logs, and recovery helpers remain addressable as lineage evidence.
- Clippy failures are routed back to the owning project, canonical source locator, implementation lineage, and project-specific qualification log before the global release is allowed to commit.

### Explicit project exclusions

- ForgeCord, ForgeStream, StreamForge, and the standalone Rust OBS ReForge project (`rust-obs-reforge`) are **excluded from global version synchronization** and from the active first-party control plane.
- The Rust OBS ReForge aliases `Rust OBS ReForge`, `OBS ReForge`, and `ReForge OBS` resolve to that same excluded standalone project. OpenDeck OBS Studio integration remains active and is not part of this exclusion.
- Historical files for those projects may remain searchable as lineage/history only. They must never be promoted to an active donor, routed through global Clippy qualification, assigned the ecosystem release version, packaged, installed, or automatically rediscovered as active projects.
- Automatic file discovery must honor the exclusion list before project classification.


## 10.2.2

### Rust control plane

- Replaced the canonical ecosystem verifier, version/status tooling, Rust qualification driver, install transaction, and rollback control surfaces with one native Rust workspace and `aetherforge-control` binary.
- Removed Python and Bash executable logic from the active 10.2.2 ecosystem payload. Required metadata/configuration formats remain data, not executable implementations.
- Added a Rust-only bootstrap source that builds the native control plane, verifies the immutable package, qualifies a protected working copy, and invokes the native installer.
- Made transaction backups durable on disk so rollback records survive process failure instead of existing only in installer memory.

### Global Rust / Clippy QA

- Standardized every functional Rust project change on one protected-tree sequence: format, safe Clippy autofix, reformat, format check, full-workspace/all-target/all-feature check, tests, strict Clippy with `-D warnings`, then release build.
- Safe Clippy autofix is refused unless the workspace is explicitly marked protected/disposable.
- Qualification logs route project id, source lineage, source locator, failing step, stdout, stderr, and log path back to the owning project.
- Version-sync-only components are not falsely claimed to have been recompiled when their source was not functionally changed in this release.

### Aether Terminal

- Advanced Aether Terminal to 10.2.2 while retaining the 10.2.1 history-relative selection model: highlighted text stays selected while wheel, keyboard, scrollbar, jump-to-live, or selection-autoscroll changes the viewport.
- Removed Python package tests/verifier and shell activation logic from the Terminal release package; native Rust unit tests remain authoritative for selection, PTY, rendering, window-lifetime, settings, and persistent-session behavior.
- Retained the persistent PTY/session broker and transactional restart/reconnect architecture.

### File routing and migration truthfulness

- Removed legacy donor archives containing shell/Python helpers from the active release payload. Their exact Chat/Library file ids remain in the routing authorities as implementation lineage and recovery evidence.
- AetherForge Control Center, ForgeHX, ReForge Logitech, STO Linux Command, and OpenSanctuary continue from Rust application-runtime lineages; legacy helper/test/installer artifacts remain quarantined as history until their next functional migration.
- OpenDeck remains synchronized to 10.2.2, but its full native Rust UI port is explicitly blocked until the complete 1.2.1 source tree is recovered; the available Library artifact is a recovery helper, not sufficient source for a no-regression UI rewrite.
- OpenDeck's normal OBS Studio integration remains active.

### Exclusions

- ForgeCord, ForgeStream, StreamForge, and standalone Rust OBS ReForge remain outside the active first-party control plane, version sync, Clippy routing, packaging, and automatic source promotion.

## 10.2.3

### Rust bootstrap package-root correction

- Fixes the 10.2.2 native bootstrap failure that resolved the extracted package root as the compiled binary directory under `target/release`, which incorrectly produced a duplicated `target/release/target/release/aetherforge-control` path.
- `aetherforge-bootstrap` now resolves the canonical package root by walking upward for the required package markers `Cargo.toml`, `PROJECT-REGISTRY.json`, and `PACKAGE-CONTENTS.sha256`.
- Adds an explicit `AETHERFORGE_PACKAGE_ROOT` override for controlled recovery/debug use; invalid override paths fail closed.
- Adds native Rust regression tests for launching from `target/release`, resolving from the compiled bootstrap binary path, and rejecting directories without canonical package markers.
- The bootstrap still verifies the immutable package before qualification and performs Clippy autofix only on a protected/disposable copy.

### Global version synchronization

- The bootstrap correction triggers the next ecosystem version, so AetherForge OS and every active first-party release identity advance together to **10.2.3**.
- Aether Terminal, AetherForge Control Center, OpenDeck, ForgeHX, ReForge Logitech, STO Linux Command, OpenSanctuary, and Aether OpenAI synchronize to 10.2.3.
- Aether Terminal is version-sync-only in this release; its history-relative selection, persistent-session, update/reconnect, menu/settings, scrolling, focus, and Ctrl+C behavior are retained unchanged.
- Other applications without functional changes are recorded as version-sync-only; their implementation lineage remains separate from the active release identity.

### Global QoL and QA

- Bootstrap diagnostics now report the canonical package root rather than failing later with a misleading duplicated path.
- Package-root discovery is marker-based instead of layout-assumption-based, so `cargo run`, direct `target/release/aetherforge-bootstrap`, and nested launch locations resolve the same canonical package.
- Strict global `cargo fmt`, safe protected-tree Clippy autofix, `cargo check`, tests, strict `cargo clippy -- -D warnings`, and release build remain mandatory before live installation.
- The failed 10.2.2 bootstrap run occurs before ecosystem identity installation; 10.2.3 remains transactional and fail-closed.

### Exclusions

- ForgeCord, ForgeStream, StreamForge, and standalone Rust OBS ReForge remain excluded from version synchronization, Clippy routing, packaging, installation, and automatic source promotion.
- OpenDeck's normal OBS Studio integration remains active.


## 10.2.4

### Rust qualification-state correction

- Fixes the 10.2.3 `cargo test` failure in `aetherforge-qualify`: the bootstrap set `AETHERFORGE_RUST_SOURCE_PROTECTED=1`, `cargo test` inherited it, and the unprotected-request unit test therefore observed protected state even though its typed request said `protected: false`.
- Removes the ambient environment-variable fallback from qualification protection. Protected/disposable workspace authority is now explicit through the typed request / `--protected` argument.
- Removes the bootstrap environment injection, eliminating hidden global state from native Rust tests.
- Adds deterministic native Rust coverage that explicit `protected: false` is rejected and explicit `protected: true` is accepted.
- Adds native coverage for failure-output extraction.

### Global QoL and QA

- Cargo/check/test/Clippy failures now include a bounded stdout/stderr excerpt in the direct qualification error while retaining the durable per-project log path.
- The strict global Rust sequence remains unchanged: format, safe protected-copy Clippy autofix, reformat, format check, full-workspace/all-target/all-feature check, tests, strict Clippy with `-D warnings`, then release build.
- The qualification correction is fail-closed and occurs before live installation.

### Global version synchronization

- AetherForge OS and every active first-party release identity advance together to **10.2.4**.
- Aether Terminal, AetherForge Control Center, OpenDeck, ForgeHX, ReForge Logitech, STO Linux Command, OpenSanctuary, and Aether OpenAI synchronize to 10.2.4.
- Aether Terminal is version-sync-only; history-relative selection, persistent sessions, update/reconnect, scrolling, focus, menu/settings, and Ctrl+C behavior remain unchanged.
- ForgeCord, ForgeStream, StreamForge, and standalone Rust OBS ReForge remain excluded. OpenDeck's normal OBS Studio integration remains active.

## 10.2.5

### Aether Terminal installer / selection contract

- Aether Terminal triggers the 10.2.5 synchronized ecosystem release.
- Fixes the 10.2.4 installer failure `native terminal selection/context-menu contract missing: terminal_selection_is_revision_bound`.
- The Terminal installer now requires the canonical history-relative regression `terminal_selection_stays_bound_to_history_across_scrollback`.
- The obsolete revision-bound selection contract is explicitly rejected by pre-install verification rather than being allowed to re-enter release tooling.
- Runtime selection behavior is unchanged and remains history-relative: selected text stays highlighted while scrolling, remains selected off-screen, and is highlighted again when scrolled back into view.

### Native QA correction

- Corrects the Terminal update-manager semantic-version regression so `parse_version("10.2.5")` expects `Some((10, 2, 5))`.
- The Terminal Rust package verifier now checks installer/UI selection-contract alignment before the installer runs.
- The global Rust verifier also inspects the embedded Terminal installer, UI source, and update-manager source before the synchronized install transaction.
- This closes the path where stale static expectations could pass ecosystem qualification and fail only after Terminal source migration began.

### Global version synchronization

- AetherForge OS and all active first-party release identities synchronize to **10.2.5**.
- Aether Terminal is the functional-change component. AetherForge OS, Control Center, OpenDeck, ForgeHX, ReForge Logitech, STO Linux Command, OpenSanctuary, and Aether OpenAI are version-sync-only for this release.
- ForgeCord, ForgeStream, StreamForge, and standalone Rust OBS ReForge remain excluded from version sync, Clippy routing, packaging, and automatic promotion.

### QoL, packaging, rollback, and status

- Removes duplicate synchronized-release notes from the Terminal package README so the current release has one canonical change section.
- No live runtime change is committed until the embedded Terminal package passes the corrected static selection/version contracts and the host Rust qualification/build/diagnostic gates.
- Failure remains transactional: Terminal source/workspace changes and global ecosystem identities roll back rather than leaving a mixed-version release.

## 10.2.6

### Aether Terminal Clippy qualification correction

- Aether Terminal triggers the 10.2.6 synchronized ecosystem release.
- Fixes the host qualification failure `error: no VCS found for this package and cargo fix can potentially perform destructive changes`.
- The Terminal installer now passes `--allow-no-vcs` together with `--allow-dirty` and `--allow-staged` to the rollback-protected `cargo clippy --fix` phase.
- This matches the global AetherForge Rust qualification policy, where autofix runs only on an explicitly protected/disposable source tree.
- `VERIFY-PACKAGE.rs` fails closed if the Terminal installer no longer contains the no-VCS autofix contract.

### Native QA

- Corrects the Terminal update-manager regression so `parse_version("10.2.6")` expects `Some((10, 2, 6))`.
- History-relative selection remains mandatory through `terminal_selection_stays_bound_to_history_across_scrollback`; the obsolete revision-bound selection contract remains forbidden.
- The strict non-fix Clippy pass continues to use `-D warnings`.
- Full Cargo/Clippy logs remain preserved for failure analysis and rollback.

### Global version synchronization

- AetherForge OS and every active first-party release identity advance together to **10.2.6**.
- Aether Terminal is the functional-change component.
- AetherForge OS, AetherForge Control Center, OpenDeck, ForgeHX, ReForge Logitech, STO Linux Command, OpenSanctuary, and Aether OpenAI are version-sync-only for this release.
- ForgeCord, ForgeStream, StreamForge, and standalone Rust OBS ReForge remain excluded from version sync, Clippy routing, packaging, and automatic promotion.

### Rollback and status

- Terminal qualification remains fail-closed and rollback-protected.
- A failed formatter, Cargo check, test, Clippy, build, GPU smoke, diagnostic, or activation phase restores the prior Terminal source/runtime.
- The ecosystem transaction remains uncommitted until the corrected Terminal package passes host qualification.


## 10.2.7

### Functional change
- Aether Terminal triggers the synchronized 10.2.7 release.
- Fixes Rust 1.98 strict Clippy `clippy::field-reassign-with-default` in `aether-terminal-settings::SettingsProfile::settings` by constructing the `profile` field inside the initial `TerminalSettings` value.
- Corrects the same detected pattern in the settings model and store native test fixtures so strict all-target Clippy does not simply stop at the next occurrence.

### QA / verification
- Terminal package verification rejects the detected settings `Default::default()` immediate-field-reassignment pattern before source migration.
- Ecosystem Rust verification inspects the embedded Terminal settings sources for the same regression.
- Native update-manager semver QA verifies `parse_version("10.2.7") == Some((10, 2, 7))`.
- Protected-tree Clippy autofix retains `--allow-no-vcs`, `--allow-dirty`, and `--allow-staged`; strict Clippy retains `-D warnings`.

### Preserved behavior
- History-relative selection, persistent sessions, scrolling, Ctrl+C behavior, DragonGlass UI, transactional activation, diagnostics, rollback, and prior bootstrap/qualification fixes remain cumulative.

### Synchronization
- AetherForge OS and all active first-party identities advance to **10.2.7**. Aether Terminal is functionally changed; the other active components are version-sync-only.
- ForgeCord, ForgeStream, StreamForge, and standalone Rust OBS ReForge remain excluded. OpenDeck's normal OBS Studio integration remains active.
