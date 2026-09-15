# OpenSanctuary v0.3.4 UI + Bridge Integration Polish Design

## Goal
Make the proven v0.3.3 Battle.net/Diablo III bridge feel like one cohesive launcher while reducing UI maintenance risk, without changing CASC decoding or native-engine behavior.

## Scope
v0.3.4 is presentation/refactor only. It preserves the v0.3.3 bridge-health, self-healing, session-supervisor, update, indexing, and official fallback behavior.

## Architecture
Keep `LauncherApp` orchestration and asynchronous task handling in `apps/launcher/src/main.rs`. Move stateless visual/theme helpers into focused modules so presentation can evolve without touching bridge/session code. The first split is intentionally conservative: theme configuration in `theme.rs`, reusable chrome/status/card widgets in `widgets.rs`, and existing motion math remains in `ui_motion.rs`.

## Battle.net-style hierarchy
- Keep global launcher chrome and Diablo III favorites/last-played navigation.
- Strengthen the selected-game identity and status hierarchy.
- Make the primary action area communicate Install/Updating/Verifying/Indexing/Play/Running as one lifecycle.
- Add a compact Bridge Health surface that shows runner, Battle.net, Agent, game install, content/index, and last recovery.
- Make Downloads & Activity visually reflect bridge/session work instead of appearing like an unrelated log.
- Keep Local Account and Social surfaces local-only; do not add Blizzard credential or login UI.

## Components
### `theme.rs`
Own dark launcher palette constants and `configure_style()`.

### `widgets.rs`
Own stateless egui widgets used by multiple pages: navigation buttons, status badges, chips, activity rows, featured/news cards, launcher menu rows, game-strip rendering, and other painter-only helpers. Functions that need page/model state stay in `main.rs`.

### `main.rs`
Own `LauncherApp`, page state, bridge/session orchestration, event processing, page composition, and user actions. No bridge behavior is moved during this release.

## Bridge health presentation
The Diablo control column adds a compact status stack:
- Bridge Health: Healthy / Degraded / Recovering / Broken
- Battle.net client: Running / Available / Missing
- Agent: Running / Idle
- Runner: Ready / Missing
- Diablo III: Installed / Missing
- Content: Indexed / Needs Index
- Last recovery message when present

The status stack is display-only and derives from existing model/discovery/supervisor data.

## Downloads integration
The bottom tray remains expandable. Active bridge/install/index/update work receives stronger progress treatment; completed/failed history remains visible. No new background worker is introduced.

## Motion/accessibility
All decorative hover/selection/progress animation respects reduced-motion. Supervision continues at its existing functional polling cadence even when decorative motion is disabled.

## Safety / legal boundaries
- No Blizzard assets, executables, credentials, or copyrighted launcher artwork are bundled.
- Battle.net/Wine handling remains isolated to `sanctuary-battlenet`.
- Native engine/render/assets crates remain compatibility-layer-free.
- v0.3.4 does not modify Blizzard files.

## Verification
- Existing launcher/model tests remain authoritative.
- New pure helper tests cover status-label mapping where appropriate.
- `cargo fmt --all`, workspace tests, strict Clippy with `-D warnings`, release build, shell verifier, and package integrity are required on the Arch host.
- Sandbox static checks cover TOML/manifests, shell syntax, source structure, version consistency, forbidden compatibility-layer leakage, and archive integrity.
