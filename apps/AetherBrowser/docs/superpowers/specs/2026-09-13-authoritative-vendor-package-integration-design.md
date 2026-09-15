# AetherBrowser v2.1.48 Authoritative Vendor Package Integration Design

## Goal
Stop UI/capability drift between AetherBrowser and the creator applications it integrates by treating the user's exact vendor installers as authoritative local compatibility inputs while keeping the running implementation Linux-native, Rust-first, and embedded inside AetherBrowser.

## Authoritative package set
AetherBrowser v2.1.48 recognizes this exact user-owned reference set:

| Integration | Canonical local package | Version identity | Inspected SHA-256 |
| --- | --- | --- | --- |
| Elgato Stream Deck | `Stream_Deck_7.5.1.22901.pkg` | 7.5.1 build 22901 | `8cc1f0b875839e2d50618a37cad2f46b689cad1d3fe1df1af1e373303515ffe8` |
| OBS Studio | `OBS-Studio-32.2.2-Windows-x64-Installer.exe` | 32.2.2 | `c3a0b880adbe64dc4bcb68f93016916ab5b55ae43fd227115287bf80257d92dc` |
| OBS + StreamElements | `obs-streamelements-setup-latest.exe` | installer channel `latest`; version is not inferred when the package does not expose it safely | `79be650ee593f79727e94c098a7d9a60fbe16ef73db2c7c54472aa7753a29ad3` |
| Streamlabs Desktop | `Streamlabs+Desktop+Setup+1.21.9-5qLbAShV5RGxPpP.exe` | 1.21.9 | `57e0c280bc4a85e66411a1ed0f874d56b8f2a33590d42146c6d07bab96ca9ceb` |
| StreamElements Ground Control | `Ground Control_x64_en-US.msi` | 2.1.20 | `b374dbf545d00626fe134b4e23e6e542eba64ebb8b9f66f4b056c11233da0ed5` |

Browser-added duplicate suffixes such as `(1)` are accepted during discovery; the canonical unsuffixed filename is preferred.

## Safety / licensing contract
- Vendor `.pkg`, `.exe`, `.msi`, Mach-O, PE, DLL, Electron, plugin runtime, and installer code is never executed by the importer.
- AetherBrowser does not redistribute the uploaded installers or vendor executable payloads in release artifacts.
- Import is data-only: hashes, safe metadata, layout/capability records, and explicitly allowlisted browser-renderable assets/configuration where the existing per-package importer supports them.
- The Linux/Rust AetherBrowser, OBS WebSocket bridge, Stream Deck HID daemon, and browser compatibility surfaces remain authoritative at runtime.
- Unknown or opaque vendor content is recorded as unsupported/reference-only rather than guessed.

## Browser integration mapping
- **Stream Deck 7.5.1** feeds the embedded Stream Deck Studio package/profile/action/layout compatibility model, including Stream Deck+ Keypad/Encoder behavior and safe package visuals.
- **OBS Studio 32.2.2** pins the Full OBS Workspace compatibility target/version in Aether Studio. The real locally installed Linux OBS remains the executable runtime; the Windows installer is metadata/reference only.
- **OBS + StreamElements** pins the StreamElements-in-OBS integration contract. StreamElements remains a browser-authoritative account/overlay surface plus OBS compatibility bridge; the Windows installer is never launched.
- **Streamlabs Desktop 1.21.9** pins the embedded Streamlabs workspace/capability baseline while the browser's persistent Streamlabs web session remains the runtime surface.
- **Ground Control 2.1.20** pins StreamElements Ground Control/Stream Deck action naming and resources. The inspected MSI exposes Stream Deck-facing alert controls including mute, unmute, pause, resume, skip and toggle alerts; those names are surfaced as vendor-aligned compatibility actions without executing the MSI application.

## Normalized package registry
`aether-creator-integrations` owns a typed registry for the five vendor package identities, discovery paths, hashes, versions, package formats, browser targets, and execution policy. The registry is used by native browser pages and release/runtime diagnostics so every integration reports which vendor baseline it is aligned to.

A generic data-only normalizer writes user-local JSON summaries under:
`~/.local/share/aetherforge/aether-browser/vendor-packages/`

It computes SHA-256 and extracts only safe identity/capability evidence. Elgato keeps its richer dedicated importer because it already parses the XAR/profile/plugin resource formats.

## Acceptance
- All five exact package identities are represented in one browser-visible registry.
- AetherBrowser reports missing/found/hash-matched/hash-mismatched without executing packages.
- OBS, Streamlabs, StreamElements, Ground Control and Stream Deck browser surfaces display their authoritative vendor baseline.
- Ground Control alert action names come from the inspected MSI evidence rather than invented labels.
- No vendor installer/binary is present in the canonical AetherBrowser release payload.
- Existing no-infinity-mirror/no-overflow capture safety, external-app containment/reintegration, Twitch login, and host-Clippy gates remain intact.
