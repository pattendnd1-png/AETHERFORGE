# Aether Browser 2.1.60

## v2.1.60 DragonGlass transparency closure

This hotfix corrects the host-reported AetherBrowser DragonGlass policy mismatch to the canonical 90% transparency / 10% smoky glass target. AetherAI remains fully removed, and the v2.1.59 rustfmt/package observability repairs remain unchanged.

# Aether Browser 2.1.59

## v2.1.59 rustfmt closure

This hotfix applies the exact host-reported `cargo fmt --check` changes from v2.1.58 and adds a recurrence guard. The observable/staged release harness and complete AetherAI removal remain unchanged.

# Aether Browser 2.1.58

## v2.1.58 fast/observable release harness

- Keeps AetherAI fully removed from AetherBrowser.
- Streams release-package verification output live so a failure can never stop at a bare `RELEASE_PACKAGE_BUILD=START` marker.
- Preserves failed diagnostics in `STATIC-VERIFY.txt` with an explicit failure marker.
- Stages source/install/static/package/checksum outputs and promotes them only after package verification succeeds, protecting the downloaded source artifact on failure.
- No browser runtime, DragonGlass UI, creator/media, profile, Velora, capture-safety, or takeover behavior changes.

# Aether Browser 2.1.55

## v2.1.55 preinstall hard-verify repair

- Fixes the v2.1.54 host-gate false failure caused by a stale Rust regression still inspecting `scripts/verify.sh` after visual probes moved to the installed-runtime acceptance path.
- Retargets that regression to `scripts/install-current-tree.sh`, where nonzero live-frame exits remain hard failures.
- Adds a dedicated release contract for the verifier-location boundary.
- Preserves the existing DragonGlass browser shell, Chromium-X11 runtime, creator integrations, capture safety, Velora, takeover/rollback safety, and known-good Twitch/YouTube/YouTube Music behavior unchanged.

# Aether Browser 2.1.54

## v2.1.54 Loop-break verification consolidation

- Makes source formatting a hard inspect-only gate instead of silently normalizing before verification.
- Runs every remaining `current-*.sh` contract automatically in both verify and package-release gates.
- Retires 15 superseded contracts that targeted pre-Chromium/pre-installed-runtime gate locations.
- Moves heavy temporary build/package staging to `$HOME/.cache/aetherforge/aether-browser/tmp` by default to avoid `/tmp` tmpfs exhaustion.
- Preserves current DragonGlass UI, Chromium-X11 web engine, OBS containment, capture safety, Velora, browser takeover, and known-good Twitch/YouTube/YouTube Music behavior.

## v2.1.54 Rust 1.98 hard-verify repair

- Fixes the two Opera-GX-refit `egui::Stroke::new` widths to use explicit `1.0_f32`, satisfying Rust 1.98 `-D warnings` without changing rendering behavior.
- Makes the Opera GX reference SHA contract rustfmt-safe by validating the symbol and exact SHA-256 value independently instead of requiring one source line.
- Adds a release-blocking regression for the Rust 1.98 GX stroke-width issue.
- No browser layout, provider, capture-safety, takeover, or integration behavior changes.

## v2.1.51 controller-software removal + Opera GX reference refit

- Removes the Stream Deck software stack from AetherBrowser rather than hiding it: no Deck crate, embedded Studio/monitor routes, daemon/service, udev rule, desktop entry, package importer, installer hook, host-gate runtime, or release payload remains.
- Keeps OBS Studio, OBS+StreamElements, Streamlabs Desktop, Ground Control, Velora, browser profiles, capture safety, and external-app containment intact.
- Refits the existing Rust/egui browser shell against the user-supplied Opera GX Linux package `135.0.5973.135` as a structure/interaction reference only: sidebar-first navigation, workspace switching, compact top chrome, service/player rail, and left-tab emphasis are translated into original AetherForge DragonGlass controls.
- Uses the exact Opera GX reference SHA-256 `960bbce3c7a993d481568f0b3a32a70417b159f902b6083dcfa43b416ccab668`; no Opera binaries, artwork, `.pak` resources, or installer payload are redistributed.
- Preserves the visible-test readiness repair: acceptance windows must become visible and blank test frames are hard failures rather than warnings.
- Preserves the dependency-safe browser takeover and rollback path so AetherBrowser can become the only user-facing browser without blindly breaking packages that other system components still depend on.
- Current authoritative creator-package registry contains OBS Studio 32.2.2, OBS+StreamElements, Streamlabs Desktop 1.21.9, and Ground Control 2.1.20; the former Stream Deck package baseline is intentionally removed from the active registry.

## v2.1.48 authoritative creator-package alignment

- Treats the user's exact Stream Deck 7.5.1, OBS Studio 32.2.2, OBS+StreamElements, Streamlabs Desktop 1.21.9, and StreamElements Ground Control 2.1.20 installers as hash-pinned, data-only compatibility inputs.
- Adds one typed browser-visible vendor baseline registry so OBS, Streamlabs, StreamElements, Ground Control, and Stream Deck report the package version they are aligned to instead of drifting independently.
- Integrates Ground Control naming from the inspected MSI (`Mute Alerts`, `UnMute Alerts`, `Pause Alerts`, `Resume Alerts`, `Skip Alert`, `Toggle Alerts`) into the Creator Hub compatibility surface.
- Adds a generic data-only package normalizer that verifies SHA-256/container/product evidence and always reports `vendor_code_executed=false`; Windows/macOS installers are never launched by AetherBrowser.
- Keeps Linux-native OBS WebSocket, persistent browser provider sessions, Stream Deck HID/runtime, no-infinity-mirror capture safety, and in-window containment authoritative at runtime.

## v2.1.48 Elgato 7.5.1 package integration

- Uses the user-owned `~/Downloads/Stream_Deck_7.5.1.22901.pkg` as the authoritative Stream Deck 7.5.1 compatibility source without executing macOS binaries.
- Imports the exact 7.5.1 build 22901 contract: 25 built-in plugins, 34 actions, Stream Deck+ model `20GBD9901`, profile schema `2.0`, Keypad + Encoder controllers, and the two-page default profile.
- Materializes 748 safe browser-renderable PageIcon, TouchBackground, and InfobarLayout assets from the local package into a user-local cache; vendor executables, dylibs, scripts, fonts, and the original package are never redistributed.
- Merges imported Elgato actions into the embedded Stream Deck Studio Action Library and translates supported vendor action UUIDs to native Rust/Linux execution adapters.
- Creates an editable native Stream Deck+ copy that preserves the real package page/slot layout, titles, and settings.
- Renders the imported package TouchBackground inside the embedded Stream Deck+ workspace and exposes PageIcon/Infobar resources through the in-browser Elgato asset catalog.


## v2.1.47 host-clean repair

Focused Rust 1.98/Clippy repair only: capture test-module ordering is corrected and regression-gated; v2.1.45 capture-safety behavior is preserved.


## v2.1.47 integrated workstation cut

- Embeds the real installed OBS Studio X11 workspace inside an AetherBrowser tab by default, preserving the full native OBS interface and the existing OBS WebSocket v5 control plane.
- Keeps integrated app pop-outs session-only and re-adopts known OBS/Stream Deck windows into the main browser on the next launch.
- Adds caret-correct omnibox editing with Left/Right/Home/End, Backspace/Delete, Ctrl+A, UTF-8-safe insertion/deletion, and in-place typo correction.
- Reworks Twitch account connection so Twitch's hosted sign-in/2FA page opens inside the existing AetherBrowser window; AetherBrowser never captures the Twitch password.
- Adds local Stream Deck Marketplace/theme package indexing, editable-copy linkage, and most-recent compatible theme restoration on Twitch CONNECTED. Vendor package originals remain untouched.
- Retains the strict host summary contract (CHECK + CLIPPY + TEST + BUILD) and the v2.1.43 aggregate test marker repair.

Focused host-compile repair for the v2.1.41 external-app restart reintegration release. Detached integrated apps remain session-only and are re-integrated into the main AetherBrowser window on the next launch. Browser Twitch, YouTube, and YouTube Music runtime paths remain outside this focused patch test.

# Aether Browser 2.1.41

Aether Browser 2.1.41 makes external-app detachment session-only. Integrated Stream Deck/Elgato surfaces may be explicitly detached during a run, but the next AetherBrowser launch discovers known detached Aether app windows and reparents them back into normal in-window tabs. Detached mode is never restored as persistent tab state.

## 2.1.41 changes

- Re-integrate surviving Stream Deck Studio and Stream Deck+ Monitor pop-outs into the main AetherBrowser window on the next launch.
- Preserve explicitly detached external-app windows across browser shutdown only long enough for restart adoption.
- Normalize adopted tabs to canonical embedded `aether://external-app/open?id=...` URLs; `mode=detached` remains transient.
- Reuse an already-running detached app when its embedded route is opened instead of spawning a duplicate process.
- Keep Twitch, YouTube, and YouTube Music runtime testing excluded from this focused patch.
- Preserve v2.1.40 Clippy-clean and v2.1.39 in-window containment regression gates.

---

# Aether Browser 2.1.40

Aether Browser 2.1.40 clears the Rust 1.98 Clippy blockers found by the v2.1.39 host gate while preserving the v2.1.39 in-window external-app containment model, Stream Deck Windows-parity editor, Twitch OAuth account flow, and Velora.tv integration.

## 2.1.40 changes

- Collapse the Stream Deck daemon action-render condition into the Rust 1.98 Clippy-approved let-chain form.
- Collapse the six Stream Deck Studio nested condition chains flagged by `clippy::collapsible-if` without changing behavior.
- Explicitly consume the three intentionally non-actionable egui `Response` values flagged by `unused_must_use`.
- Add release-blocking `current-v2-1-40-deck-clippy-clean.sh` coverage for the exact v2.1.39 host failures.
- Preserve integrated external applications inside the main AetherBrowser window by default, with explicit detach/reattach only.
- Keep browser Twitch, YouTube, and YouTube Music runtime probes excluded as known-good; focused runtime validation remains Velora.tv + Stream Deck/Elgato.

# Aether Browser 2.1.39

Aether Browser 2.1.39 keeps integrated external applications inside the main AetherBrowser tab by default. Stream Deck Studio and the Stream Deck+ diagnostic monitor now use an X11 child-window containment host with explicit detach/reattach support; standalone windows are created only through an explicit detached launch. Velora.tv and the focused Stream Deck/Elgato test scope remain unchanged.

## 2.1.39 changes

- Add a generic X11 external-app host that matches child windows by `_NET_WM_PID` and reparents them into the active AetherBrowser content rectangle.
- Launch Stream Deck Studio and Stream Deck+ Monitor in-window by default from Deck Studio.
- Add explicit detached launch routes plus Ctrl+Shift+D detach and Ctrl+Shift+R reattach controls.
- Update Stream Deck+ runtime acceptance to require the in-window containment PASS marker instead of directly spawning the Studio as a top-level window.
- Preserve the v2.1.38 host-compile repair, Windows-parity editor, Twitch OAuth account flow, Velora integration, and the Twitch/YouTube/YouTube Music runtime exclusions.

# Aether Browser 2.1.38

Aether Browser 2.1.38 clears the v2.1.37 Stream Deck Studio host compile and Rust 1.98 Clippy blockers while preserving the Windows-parity Stream Deck/Elgato editor, Twitch OAuth account flow, direct Stream Deck+ HID path, and Velora.tv integration.

## 2.1.38 changes

- Implement the required eframe 0.34 `App::ui` contract in `aether-streamdeck-studio`.
- Migrate the Stream Deck Studio panel API usage away from the deprecated top-level panel aliases/methods reported by the host compiler.
- Clear the `only_used_in_recursion`, `manual_range_contains`, and `manual_is_multiple_of` Clippy failures reported by Rust 1.98.
- Add `current-v2-1-38-deck-host-compile.sh` as a release-blocking regression contract.
- Keep host runtime acceptance focused on Velora.tv + Stream Deck/Elgato. Browser Twitch, YouTube, and YouTube Music runtime probes remain skipped as known-good.

# Aether Browser 2.1.35

Aether Browser 2.1.35 adds Velora.tv as a first-class browser-authoritative streaming provider and fixes the stale OBS WebSocket v5 transition verifier expectation that blocked the v2.1.34 preinstall gate.

## 2.1.35 changes

- Add Velora.tv to the canonical streaming-provider registry.
- Route Velora through the persistent Chromium/X11 compatibility profile for real login, browsing, chat, and playback without undocumented private API scraping.
- Preserve Velora provider cookies during Servo-to-Chromium compatibility migration.
- Open Velora provider actions in a normal provider tab and keep account/session state persistent in the normal profile.
- Correct the OBS protocol mock from obsolete `GetTransitionList` to official `GetSceneTransitionList`.
- Add a release-blocking `current-v2-1-35-velora.sh` contract.
- Preserve Twitch, YouTube Music, OBS, Streamlabs, StreamElements, Stream Deck+, AetherAI, and Chromium/X11 ownership.

# Aether Browser 2.1.35

Aether Browser 2.1.35 hardens the Chromium/X11 tab lifecycle, corrects the OBS v5 transition/virtual-camera verifier, and turns Stream Deck+ verification into a dedicated native monitor window with real JPEG HID output plus Twitch device-code account binding.

## 2.1.35 changes

- Stream Deck+ testing runs in a separate `AetherForge Stream Deck+ Monitor` native X11 window so hardware/firmware/display/input progress stays visible independently of browser tabs, OBS, and Streamlabs.
- Stream Deck+ display verification now uses real 1024-byte chunked JPEG HID output reports for the 8 keys, full LCD, and touch window; the test image remains visible until explicitly confirmed, retried, restored, or aborted.
- Twitch account connection is available from the Stream Deck+ monitor through Twitch Device Code OAuth. Client IDs are user/AetherForge configuration, tokens are kept in the desktop Secret Service, and tokens are never printed in verifier logs.
- Inactive Chromium/X11 child windows are moved offscreen, unmapped, flushed, and map-state verified before a sibling tab becomes authoritative; this fixes YouTube Music failing to disengage.
- OBS verification uses the correct `GetSceneTransitionList` request and treats an unavailable virtual camera as `NOT_AVAILABLE` instead of a control-plane failure.
- OBS, Streamlabs, and Stream Deck+ remain three independent runtime acceptance tracks.
- Chromium/X11 remains authoritative for all external HTTP/HTTPS content; working YouTube, YouTube Music, Twitch, Streamlabs, login persistence, AetherAI, and Studio behavior are preserved.

# Aether Browser 2.1.35

Aether Browser 2.1.35 separates OBS, Streamlabs, and Stream Deck+ into independent runtime acceptance tracks and adds full Stream Deck+ hardware/firmware management to the browser package while preserving the Chromium/X11 web-engine cutover from v2.1.30.

## 2.1.35 changes

- Fix malformed Stream Deck+ firmware-status HTML format strings in `aether-native-pages` by using Rust raw strings, clearing the host `cargo check` / Clippy / test / build syntax failure.
- Add a release-blocking native-page syntax regression for those firmware status strings.
- Fix `aether-deck/Cargo.toml` so `sha2 = "0.10"` is correctly declared under `[dependencies]`, clearing the v2.1.31 host `cargo-fetch` manifest failure.
- Add a mandatory manifest-structure regression gate so Deck dependencies cannot be emitted under `[lints]` again.

- OBS now has a dedicated runtime verifier with OBS WebSocket v5 handshake, capability queries, scratch-scene creation, scene switching, restoration, and cleanup. It never starts a public stream as part of verification.
- Streamlabs now has its own Chromium/account/session verifier with explicit `PASS`, `NOT_CONNECTED`, and `FAIL` states and no dependency on OBS.
- Stream Deck+ now has its own physical hardware verifier covering PID `0x0084`, direct HID, firmware reads, display/brightness output, all 8 keys, all 4 dial rotations, all 4 dial presses, all 4 touch-strip regions, profile/action integration, and reconnect.
- Adds an in-browser Stream Deck+ Software Stack / Firmware Manager surface and installs the native verifier tooling with the browser package.
- Firmware packages are accepted only when device IDs and SHA-256 match a verified official-vendor package; firmware writes require explicit confirmation and undocumented blind flashing is not enabled.
- Keeps all external HTTP/HTTPS browsing on Chromium/X11 and keeps Servo internal/non-HTTP only.

## Provider compatibility architecture

- Aether/native internal non-HTTP content remains on the Rust-native/Servo path.
- All external HTTP/HTTPS content, including YouTube, YouTube Music, Twitch, Google Accounts, Streamlabs, and StreamElements, routes to the persistent Chromium/X11 surface.
- Normal compatibility state persists across close/restart/upgrade; Private mode stays ephemeral.
- Existing Servo profile data is preserved; migration into Chromium is copy-only.

## Aether Studio

AetherBrowser remains the authoritative Studio UI. OBS is supervised as a minimized background compatibility backend over OBS WebSocket v5, while Streamlabs and StreamElements are integrated through their account/import/cloud boundaries and OpenDeck exposes stable Studio command IDs.

## Verification

The hard host gate runs formatting, workspace `cargo check`, Clippy with `-D warnings`, browser regression tests, full workspace tests, first-party release build, current static contracts, package promotion, installed identity checks, persistent-login checks, interactive provider playback validation, then independent OBS and Streamlabs runtime gates.

The handoff environment does not have Cargo/Rustc, so host Rust compile/Clippy/install, actual Chromium provider playback, audible media, login persistence, and creator-service runtime remain authoritative on the AetherForge host.

## Release contract

One canonical semantic version, one self-contained installer, hard host verification before promotion, version-specific VERIFY / INSTALL-VERIFY / HOST-GATE files in `~/Downloads`, and no RC/revision suffixes.