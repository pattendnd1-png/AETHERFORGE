# Aether Browser v2.1.60
## DragonGlass transparency policy hotfix

- Fix the sole v2.1.59 host test failure by changing AetherBrowser DragonGlass tokens from 75% transparent / 25% smoky to the canonical 90% transparent / 10% smoky policy.
- Add a release-blocking regression that forbids the legacy 75/25 browser policy from returning.
- Preserve the complete AetherAI removal, rustfmt closure, observable staged packaging, browser takeover, creator/media integrations, and runtime behavior unchanged.

# Aether Browser v2.1.59
## Rustfmt closure hotfix

- Apply the exact `cargo fmt --check` deltas exposed by the v2.1.58 host gate: collapse the `aether_native_pages` import in `aether-engine-servo/src/live.rs` and remove the extra trailing blank line from `aether-native-pages/tests/pages.rs`.
- Add a release-blocking recurrence guard for those exact formatting defects.
- Preserve the v2.1.58 observable/staged packaging harness and the complete AetherAI excision unchanged.
- No browser runtime, UI, DragonGlass, creator/media, profile, takeover, or provider behavior changes.

# Aether Browser v2.1.58
## Release-package observability + staged promotion hotfix

- Fix the v2.1.57 silent abort at `AETHER_BROWSER_RELEASE_PACKAGE_BUILD=START` by streaming static/package gate output live instead of redirecting it exclusively to a file.
- Preserve the exact failing output in `STATIC-VERIFY.txt` and emit an explicit `AETHER_BROWSER_RELEASE_STATIC_VERIFY=FAIL:<rc>` marker before exiting.
- Stage release artifacts in a temporary output directory and promote them only after all package gates pass, so a failed check cannot delete or half-replace the downloaded source ZIP.
- Add a release-blocking regression that injects a failing `cargo fmt` and proves live diagnostics, labeled failure, retained diagnostics, and source-archive preservation.
- Preserve the complete AetherAI removal from v2.1.56/v2.1.57; no browser UI/runtime feature behavior changes.

# Aether Browser v2.1.57
## Release handoff hotfix

- Corrected the downloadable SHA256 manifest so the pre-run handoff verifies only artifacts actually distributed before HIT-IT execution.
- Preserved the v2.1.56 AetherAI excision unchanged; no browser runtime/UI feature changes are introduced in this hotfix.
- Generated install/static/package verification files remain produced by the HIT-IT/package flow and are no longer required to exist before that flow starts.

- Remove AetherAI from AetherBrowser end-to-end: Rust IPC bridge, native route/UI controls, credential/settings flow, separate browser service, build stage, wrapper, systemd unit, package payload, install/runtime gates, and positive AI contracts.
- Remove the browser-only OpenAI/libsecret credential dependency that existed solely for AetherAI.
- Compact the top chrome, Home quick actions, sidebar service rail, and utility dock so no dead AI slot remains.
- Add a release-blocking assistant-removal contract that rejects accidental reintroduction while permitting one migration-only cleanup of the retired user service.
- Preserve Chromium/X11 browsing, DragonGlass, creator/OBS/Streamlabs integrations, media playback, profiles, Velora, capture safety, library, and browser takeover behavior.

# Aether Browser 2.1.55

## Preinstall hard-verify stale-contract repair

- Retarget the `aether-engine-servo` shutdown regression from the preinstall `scripts/verify.sh` path to the installed-runtime visual probe in `scripts/install-current-tree.sh`.
- Preserve strict failure semantics: a nonzero installed live-frame probe exit is accepted only when the probe succeeds with exit 0 and all visual markers are present; otherwise `hard_fail=1` remains release-blocking.
- Add a release-blocking `current-v2-1-55-live-frame-gate-location.sh` regression so this verifier-location mismatch cannot recur.
- Advance canonical release identity to 2.1.55 without changing browser UI/layout, DragonGlass styling, Chromium-X11 runtime behavior, OBS/Streamlabs/StreamElements/Ground Control integration, capture safety, Velora, browser takeover, or known-good Twitch/YouTube/YouTube Music paths.

# Aether Browser 2.1.54

## Loop-break verification consolidation

- Applied the rustfmt changes exposed by the v2.1.53 full diagnostic so the canonical source is formatting-clean before verification.
- Canonical `verify.sh` no longer mutates the source with `cargo fmt`; formatting is now checked as-is.
- Canonical verification and release packaging dynamically execute every remaining `current-*.sh` regression, preventing hidden stale tests from accumulating outside the release gate.
- Removed 15 obsolete current-contract scripts whose assertions targeted retired Servo-era, provider-probe, and old host-gate locations already superseded by newer Chromium-X11 and installed-runtime regressions.
- Heavy temporary package/AetherAI staging now defaults to persistent user cache storage instead of `/tmp`, preventing the `No space left on device` failure that caused v2.1.53 diagnostic test/build/package false negatives.
- No browser UI/layout, OBS, Streamlabs, StreamElements, Ground Control, capture safety, takeover, Velora, or known-good Twitch/YouTube/YouTube Music behavior changes.

## 2.1.53

- Remove the final orphaned `prewarm_elgato_751_cache` installer call left after Stream Deck software removal.
- Add a negative regression that forbids removed Stream Deck/Elgato controller-software hooks from returning to the installer.
- No browser UI, GX layout, provider, OBS, Streamlabs, StreamElements, Ground Control, takeover, or capture-safety behavior changes.

# Aether Browser 2.1.52

## Rust 1.98 hard-verify repair

- Fix two GX shell `egui::Stroke::new` width literals by making the required `f32` type explicit, eliminating Rust 1.98 `float_literal_f32_fallback` failures under `-D warnings`.
- Replace the brittle one-line Opera GX SHA source assertion with symbol + exact-value assertions that survive `cargo fmt` line wrapping.
- Add a release regression for the explicit GX `f32` stroke widths.
- Preserve the v2.1.51 shutdown repair and all v2.1.50 restructuring behavior unchanged.

# Aether Browser 2.1.51

## Host bootstrap shutdown repair

- Fixes the v2.1.50 host-gate early abort where raw `pgrep -x aether-browser` treated already-dead zombie processes as live and unkillable.
- Preverify and promotion shutdown now inspect only the current user's `aether-browser` processes and ignore `Z` zombie states while still TERM/KILL stopping genuinely live browser processes.
- Adds explicit live-PID/zombie diagnostics and a release regression preventing raw `pgrep` gating from returning.
- No browser UI, Opera GX refit, OBS/Streamlabs/StreamElements/Ground Control, capture-safety, takeover, or provider behavior changes.

# Aether Browser 2.1.50

## Stream Deck software removal + Opera GX Linux-reference shell restructure

- Remove the Stream Deck software integration end-to-end: dedicated Rust crate and binaries, embedded routes/pages, daemon/monitor/Studio wrappers, systemd unit, udev rule, desktop entry, package importer, installer hooks, host runtime acceptance, and release requirements.
- Keep the remaining creator stack intact: OBS Studio, OBS+StreamElements, Streamlabs Desktop, Ground Control, capture-safety enforcement, Velora, and AetherAI.
- Reduce the authoritative vendor registry to the four retained creator packages and keep all vendor installers data-only/reference-only.
- Refit the existing AetherForge shell against Opera GX Linux `135.0.5973.135` / SHA-256 `960bbce3c7a993d481568f0b3a32a70417b159f902b6083dcfa43b416ccab668` for sidebar/workspace/tab/service organization while retaining original DragonGlass visuals and assets.
- Preserve the v2.1.49 visible acceptance repair and dependency-safe browser takeover/rollback behavior.
- Add release-blocking negative gates that reject reintroduced Stream Deck software paths and reject bundled vendor `.deb`, `.pkg`, `.msi`, or `.exe` installers.

# Aether Browser 2.1.49

- Repaired visual acceptance sequencing so test windows are visible before capture and blank frames fail hard.
- Kept Stream Deck Studio visible throughout physical input acceptance.
- Added Opera GX 135.0.5973.135 layout-reference markers with AetherForge DragonGlass-only styling refinements; no vendor assets are redistributed.
- Expanded dependency-safe browser takeover/rollback associations while preserving installed browser packages/profiles by default.
- Preserved the v2.1.48 authoritative vendor package registry and OBS/Streamlabs capture safety behavior.

# Aether Browser 2.1.48

## Authoritative OBS / Streamlabs / StreamElements / Ground Control / Stream Deck package alignment

- Add a typed five-package compatibility registry covering the exact user-owned vendor installers used for AetherBrowser creator integration.
- Hash-pin OBS Studio 32.2.2, Streamlabs Desktop 1.21.9, OBS+StreamElements, Ground Control 2.1.20, and Stream Deck 7.5.1 build 22901.
- Surface the vendor baselines directly in AetherBrowser Creator/Studio pages and integrate Ground Control action naming from the inspected MSI.
- Add `vendor-package-normalize.py`, a data-only verifier that checks package hash/container/evidence without executing vendor code.
- Do not redistribute `.pkg`, `.exe`, `.msi`, Mach-O, PE, DLL, Electron, or plugin runtime payloads.

## Elgato Stream Deck 7.5.1 package integration

- Consume the user-owned Stream Deck 7.5.1 build 22901 macOS package as a data-only compatibility source on AetherForge/Linux.
- Verify exact package SHA-256 and exact 7.5.1 content contract (25 plugins, 34 actions, Stream Deck+ model 20GBD9901, schema 2.0).
- Safely extract PNG/JPEG/SVG/JSON PageIcon, TouchBackground, and InfobarLayout resources to the user-local browser cache; never execute or redistribute vendor macOS code.
- Merge imported action definitions into the embedded Stream Deck Studio and translate supported Elgato UUIDs to native Rust action handlers.
- Preserve real Stream Deck+ default profile page/slot layout, titles, and settings in editable AetherForge copies.
- Render imported TouchBackground assets inside the browser-embedded Stream Deck+ workspace and expose the remaining imported UI asset catalog for layout fidelity.
- Preserve OBS/Streamlabs capture-safety, hosted Twitch login, omnibox editing, theme restore, and external-app reintegration behavior.

# Aether Browser 2.1.47

## Host-clean Stream Deck Clippy repair

- Collapse the Twitch application setup button guard in Stream Deck Studio to satisfy Rust 1.98 `clippy::collapsible_if` without changing behavior.
- Add a release-blocking regression for this exact Twitch setup nested-`if` pattern.
- Preserve v2.1.46 capture-safety, OBS/Streamlabs no-recursion, hosted Twitch login, omnibox editing, Stream Deck theme restore, and external-app reintegration unchanged.

# Aether Browser 2.1.46

## Host-clean repair

- Fix Rust 1.98 Clippy `items_after_test_module` in `aether-capture` by keeping production items before the `#[cfg(test)]` module.
- Add a release regression that fails if production capture functions are ever placed after the test module again.
- Preserve v2.1.45 strict OBS/Streamlabs no-recursion and scene-clipping behavior unchanged.


## Capture safety + v2.1.44 host repair

- Fixed the Rust 1.98 Clippy blocker from the omnibox caret work by removing the unused `omni_text_rect` binding without changing caret behavior.
- Fixed the OBS in-window workspace regression to validate the real `known_external_app_targets()` restart-adoption registry instead of relying on source-line adjacency.
- Added strict `STRICT_NO_RECURSION` capture safety for embedded OBS and Streamlabs workflows. Whole-display capture sources that can recursively see the browser are blocked while embedded; window capture aimed at AetherBrowser/OBS/Streamlabs is blocked.
- Added OBS WebSocket safety enforcement that disables recursive-risk scene items and clamps overflowing scene items to the configured OBS canvas.
- Added runtime verification markers for infinity-mirror blocking and scene-bound clipping in both OBS and Streamlabs integration paths.
- Preserved hosted Twitch login, omnibox editing, Stream Deck theme restore, external-app containment, restart reintegration, and the v2.1.43 host-summary contract.

# Aether Browser 2.1.44

## Integrated OBS / Twitch / Stream Deck workstation

- OBS Studio now has a full in-window workspace target: AetherBrowser launches the real installed OBS Studio with the X11/XWayland backend and reparents its native window into the active browser tab. Explicit detach remains available, and restart re-integration treats OBS as a known external-app target.
- The existing typed OBS WebSocket v5 integration remains available for AetherForge/Stream Deck commands while the embedded native OBS UI supplies full preview, Scenes, Sources, Mixer, Transitions, Studio Mode, filters/properties, docks/plugins, profiles, scene collections, settings, recording, streaming, replay buffer, and virtual-camera surfaces.
- Twitch account setup no longer exposes the application Client ID in the normal Accounts UI. Twitch's hosted sign-in/2FA page is requested through the parent AetherBrowser session, while OAuth tokens remain in Secret Service. Missing app configuration reports `TWITCH_ACCOUNT=APP_NOT_CONFIGURED` rather than an opaque HTTP 400.
- Omnibox editing now has a real caret with Left/Right/Home/End, Backspace/Delete at the caret, Ctrl+A, UTF-8-safe insertion/deletion, and caret-position rendering.
- Stream Deck Studio now indexes local Marketplace/profile packages, associates editable AetherForge profile copies without altering vendor originals, tracks last-used compatible packages, and restores the most recent compatible linked profile on the first Twitch `CONNECTED` transition.
- Added v2.1.44 structural regression gates for omnibox editing, hosted Twitch login, native OBS in-window workspace, and Stream Deck theme restoration.
- Preserved browser Twitch/YouTube/YouTube Music runtime exclusions as user-confirmed known-good paths; v2.1.44 runtime focus is OBS + Velora + Stream Deck/Elgato + Twitch auth + omnibox.

# Aether Browser 2.1.43

- Fixed the consolidated host-gate false negative by restoring the canonical `AETHER_BROWSER_TEST=PASS|FAIL` aggregate marker after the focused Deck, native-home, and Velora provider test gates.
- Preserved the strict host summary contract (`CHECK`, `CLIPPY`, `TEST`, `BUILD`) rather than weakening host verification.
- Preserved v2.1.42 external-app containment/reintegration behavior, Stream Deck Windows parity, Twitch account integration, and the Velora-focused runtime scope.
- Added `current-v2-1-43-host-test-summary.sh` so future releases cannot silently drop the aggregate test marker again.

## 2.1.42

- Fixed Rust 1.98 Clippy promotion blockers in the external-app reintegration engine.
- Removed the unused `active_external_app_surface` helper.
- Collapsed the external-app attached visibility guard without changing detach/reintegrate behavior.
- Added a release-blocking v2.1.42 engine Clippy regression while preserving v2.1.41 restart reintegration.

## 2.1.41

- Made external-app detach state session-only.
- Added startup discovery/adoption of detached Stream Deck Studio and Stream Deck+ Monitor X11 windows.
- Restored adopted apps as canonical embedded tabs rather than persistent detached windows.
- Preserved detached external-app processes at browser shutdown for next-launch adoption.
- Added `current-v2-1-41-external-app-reintegration.sh` as a release-blocking regression contract.

## 2.1.40

- Fix all Rust 1.98 Clippy blockers reported by the v2.1.39 host verification in `aether-deck-daemon` and `aether-streamdeck-studio`.
- Add a release-blocking Deck Clippy regression contract for the daemon render path, profile/page selection, Multi Action editing, Delete handling, and ignored egui responses.
- Preserve the v2.1.39 in-window external-app containment architecture and explicit detach/reattach behavior.
- Keep host runtime acceptance focused on Velora.tv and Stream Deck/Elgato; Twitch/YouTube/YouTube Music browser runtime tests remain excluded as known-good.

## 2.1.39

- Keep integrated external applications contained inside the main AetherBrowser window by default.
- Add reusable X11 external-app child-window hosting with PID-based window discovery, hide/show, resize, detach, reattach, and close behavior.
- Route Stream Deck Studio and Stream Deck+ Monitor through the in-window host; detached launch remains explicit.
- Add release-blocking v2.1.39 external-app containment regression coverage and focused runtime verification.

# Changelog

## 2.1.38

- Fix the Stream Deck Studio host compile blocker by implementing the eframe 0.34 `App::ui` contract and migrating its panel calls off deprecated top-level APIs.
- Clear the three Rust 1.98 Clippy blockers in the Stream Deck editor/action path (`only_used_in_recursion`, `manual_range_contains`, and `manual_is_multiple_of`).
- Add a release-blocking v2.1.38 Deck host-compile regression contract so these exact failures cannot return.
- Keep the patch runtime scope focused on Velora.tv and Stream Deck/Elgato; Twitch/YouTube/YouTube Music browser runtime tests remain excluded as known-good.
- Preserve the v2.1.37 Windows-parity Stream Deck Studio, Twitch OAuth account flow, Velora provider integration, and direct Stream Deck+ HID architecture.

## 2.1.36

- Fixed the Rust 1.98 `clippy::collapsible-if` host blocker in the Stream Deck+ Twitch device-code monitor error path.
- Replaced the formatting-fragile `device.write` Stream Deck static check with a rustfmt-safe HID output-report assertion.
- Added a release-blocking v2.1.36 host-gate regression contract covering both failures observed in the v2.1.35 host verification.
- Preserved the v2.1.35 Velora.tv first-class provider integration and the existing separated OBS / Streamlabs / Stream Deck architecture.

## 2.1.35

- Added Velora.tv as a first-class browser-authoritative streaming provider with persistent Chromium/X11 session handling.
- Added Velora provider routing and scoped cookie migration without inventing an undocumented native API.
- Corrected the stale OBS WebSocket v5 mock request from `GetTransitionList` to `GetSceneTransitionList`, matching the production verifier.
- Added a release-blocking Velora/OBS contract while preserving the existing streaming and Stream Deck architecture.

## 2.1.34

- Added a standalone AetherForge Stream Deck+ Monitor window with live verifier state and explicit Retry/Confirm/Restore/Abort controls.
- Replaced the temporary Stream Deck+ color-fill probe with real chunked JPEG HID output for keys, full LCD, and touch window; output is held until user confirmation.
- Added Twitch Device Code OAuth support to the Deck monitor using a configured Twitch client ID and desktop Secret Service token storage.
- Hardened Chromium X11 child disengagement with offscreen move + unmap + map-state verification, preventing YouTube Music from remaining attached over another tab.
- Corrected OBS transition enumeration to GetSceneTransitionList and made unavailable virtual camera capability non-fatal.
- Preserved independent OBS, Streamlabs, and Stream Deck+ verification artifacts and host gates.

# Aether Browser 2.1.33

- Fix the v2.1.32 `aether-native-pages` Rust syntax failure in the Stream Deck+ firmware status HTML strings by switching them to raw string literals.
- Add a release-blocking native-page syntax regression so malformed firmware HTML format strings cannot ship again.
- Preserve the independent OBS, Streamlabs, and physical Stream Deck+ runtime verification architecture unchanged.
- Preserve Chromium/X11 as the authoritative external web engine.

# Aether Browser 2.1.32

- Fix the v2.1.31 `aether-deck` Cargo manifest regression that placed `sha2` under `[lints]` instead of `[dependencies]`.
- Add a release-blocking semantic manifest regression test for the Stream Deck+ crate.
- Preserve the independent OBS, Streamlabs, and physical Stream Deck+ verification architecture from v2.1.31 unchanged.
- Preserve Chromium/X11 as the authoritative external web engine.

# Aether Browser 2.1.31

- Split OBS, Streamlabs, and Stream Deck+ into independent runtime verification artifacts and host-gate results.
- Add OBS WebSocket v5 scratch-scene/control-plane verification with state restoration and cleanup.
- Add independent Streamlabs Chromium/account/session verification with PASS / NOT_CONNECTED / FAIL states.
- Add Stream Deck+ physical HID coverage for 8 keys, 4 rotary encoders, 4 dial presses, 4 touch sections, output/display, firmware reads, profile/action path, and reconnect.
- Add browser-integrated Stream Deck+ Software Stack and Firmware Manager surfaces.
- Install the OBS verifier and all three runtime verification scripts with the package.
- Preserve the v2.1.30 Chromium/X11 authoritative external-web architecture.

# Aether Browser 2.1.30

- Made Chromium/X11 the authoritative engine for all external HTTP/HTTPS web content.
- Removed the provider-host allowlist: ordinary sites and media-heavy sites now follow the same Chromium path.
- Removed automatic external-web fallback to Servo and the native media shim.
- Preserved Servo only for internal/non-HTTP Aether surfaces.
- Preserved persistent normal/private profile separation and no-logout-on-close behavior.
- Updated runtime/status/UI engine markers to identify Chromium X11 ownership.
- Added mandatory `current-v2-1-30-chromium-web-cutover.sh` release verification.

# Aether Browser 2.1.30

- Force the Linux winit event loop onto X11/XWayland for compatibility child embedding.
- Force Chromium compatibility windows to `--ozone-platform=x11` so X11 reparenting is valid.
- Remove silent Servo fallback for compatibility-required provider pages; failures are explicit instead.
- Add a regression contract for the X11/Chromium cutover.
- Preserve persistent compatibility profiles, no-logout-on-close behavior, Aether Studio, OBS, Streamlabs, StreamElements, OpenDeck, AetherAI, and native media fallbacks.

# Aether Browser v2.1.28

- Replace the formatting-sensitive OBS Stream Studio static verifier with a semantic function-scope assertion check.
- Add a regression guard that rejects the old exact-line `stream.html.contains(...)` grep contract.
- Preserve the v2.1.27 dual-engine provider routing, persistent compatibility login state, browser-authoritative Aether Studio, OBS/Streamlabs/StreamElements/OpenDeck integration, AetherAI, DragonGlass UI, and native media fallbacks unchanged.

# Aether Browser v2.1.27

- Clear the five Rust 1.98 Clippy `collapsible_if` failures in compatibility-surface teardown inside `aether-engine-servo`.
- Add an executable regression guard for every compatibility cleanup shape reported by the v2.1.26 host gate.
- Preserve the v2.1.26 dual-engine provider routing, persistent compatibility login profile, browser-authoritative Aether Studio, OBS/Streamlabs/StreamElements/OpenDeck integration, AetherAI, DragonGlass UI, and native media fallbacks unchanged.

# Aether Browser v2.1.26

- Fix Rust 1.98 Clippy `collapsible_if` failure in the Chromium compatibility launch path.
- Align the Stream Studio regression test with the current browser-authoritative Aether Studio surface.
- Add an executable regression guard for both v2.1.25 host-gate blockers.
- Preserve dual-engine provider routing, persistent normal login state, OBS/Streamlabs/StreamElements/OpenDeck integration, AetherAI, DragonGlass UI, and native media fallbacks unchanged.

# Aether Browser v2.1.25

- Dual-engine provider compatibility cutover with persistent normal login state.
- Browser-authoritative Aether Studio integration for OBS/Streamlabs/StreamElements/OpenDeck.
- Existing Servo profile preserved; Private compatibility state remains ephemeral.
- Native yt-dlp/Streamlink/GStreamer transports retained as fallbacks.
