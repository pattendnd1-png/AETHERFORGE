# OpenSanctuary v0.2.0 Design

## Goal

Turn the v0.1 native launcher/content probe into a useful local Diablo III installation indexer with a persistent read-only inventory and a more Battle.net-like game page. v0.2.0 must remain a native Linux Rust application and must not execute Blizzard Windows binaries or depend on Wine, Proton, DXVK, VKD3D, Lutris, Bottles, or Winetricks.

## Release Scope

v0.2.0 adds five user-visible capabilities:

1. Parse `.build.info` into structured installation metadata instead of only extracting a build key.
2. Build a persistent local inventory of the installed CASC storage topology under `Data/`.
3. Cache that inventory as versioned JSON under the OpenSanctuary XDG cache directory and invalidate it when the installation fingerprint changes.
4. Expose build metadata, archive/index counts, total installed bytes, and inventory state in the launcher.
5. Add a Content page that lets the user inspect the indexed local storage entries without modifying or extracting Blizzard data.

## Non-Goals

v0.2.0 does not decode BLTE payloads, parse Diablo III root/encoding manifests, extract logical game assets, implement gameplay, connect to Battle.net game services, or install/update Blizzard content. Those capabilities depend on the stable inventory/cache boundary created here.

## Architecture

### `sanctuary-casc`

`sanctuary-casc` becomes the single owner of local CASC installation metadata and inventory construction.

It exposes:

- `BuildInfo` — structured fields parsed from `.build.info`.
- `StorageEntry` — one local file under the installation's `Data/` tree with relative path, byte length, and a coarse storage kind.
- `StorageKind` — `Index`, `Archive`, `Config`, or `Other`.
- `ContentInventory` — schema version, installation fingerprint, build metadata, aggregate counts, total bytes, and sorted storage entries.
- `build_inventory(install_path: &Path) -> Result<ContentInventory, SanctuaryError>`.
- `read_cached_inventory(cache_path: &Path) -> Result<Option<ContentInventory>, SanctuaryError>`.
- `write_cached_inventory(cache_path: &Path, inventory: &ContentInventory) -> Result<(), SanctuaryError>`.
- `inventory_is_current(inventory: &ContentInventory, install_path: &Path) -> Result<bool, SanctuaryError>`.

The fingerprint is deterministic and cheap: build-key metadata plus the relative path, byte length, and modification timestamp of `.build.info` and each discovered `Data/` file. The fingerprint is only for cache invalidation; it is not a cryptographic integrity proof.

### `sanctuary-launcher`

The launcher model gains an optional `ContentInventory` and an explicit inventory lifecycle:

- installation located
- inventory missing/stale
- indexing
- inventory ready
- indexing failed

The existing asynchronous task bus remains the mechanism for probe/index progress. Index construction runs off the UI thread and publishes one terminal result.

### Launcher application

The launcher keeps the v0.1 dark Battle.net-inspired structure but adds:

- a secondary navigation row for `Overview`, `Content`, and `Settings`;
- a build-information strip below the hero area;
- storage statistics cards for archives, index files, and installed size;
- a Content page with search/filter text, storage-kind filter, and a scrollable table-like list;
- a cache-state badge showing `Indexed`, `Stale`, or `Not indexed`;
- a `Re-index` action that never writes to the Diablo III installation.

The visual design remains original OpenSanctuary branding with no Blizzard logos or copied artwork.

## Build Information Parsing

`.build.info` is a pipe-delimited table whose header cells may contain type annotations after `!`. The parser must:

- normalize header names by removing the `!TYPE` suffix and trimming whitespace;
- accept common variants such as `Build Key` and `BuildKey`;
- preserve unknown columns in a string map;
- select the first non-empty data row;
- expose at least `branch`, `build_key`, `cdn_key`, `version`, and `product` when present;
- return a clear content-store error when the file has no header or data row.

The parser must not make network requests.

## Inventory Classification

Every regular file recursively discovered below `Data/` is indexed using a path relative to the Diablo III installation.

Classification rules are deterministic:

- extension `.idx` -> `Index`;
- basename beginning with `data.` followed by decimal digits -> `Archive`;
- paths containing `/config/` or basename `config` -> `Config`;
- everything else -> `Other`.

Entries are sorted lexicographically by relative path before caching or display so output is stable across runs.

## Cache Layout

The default cache path is:

`$XDG_CACHE_HOME/opensanctuary/content/inventory-v1.json`

or, when `XDG_CACHE_HOME` is unset:

`$HOME/.cache/opensanctuary/content/inventory-v1.json`

`ContentInventory::schema_version` is `1`. A cache with any other schema version is treated as absent, not as a fatal launcher error.

Cache writes are atomic at the process level: serialize to a sibling temporary file, flush it, then rename it over the final path.

## Data Flow

1. Launcher loads configuration and discovers the configured Diablo III installation.
2. Launcher probes the installation and parses `.build.info`.
3. Launcher attempts to load `inventory-v1.json`.
4. If the cache exists and its fingerprint matches the current installation, it is used immediately.
5. If the cache is missing or stale, the launcher marks content as needing indexing and offers `Index`/`Re-index`.
6. Indexing walks `Data/` read-only, creates `ContentInventory`, writes the XDG cache, and publishes the result to the launcher model.
7. The Overview and Content pages render only model data; they do not touch the filesystem directly.

## Error Handling

- Missing `.build.info` or `Data/` keeps the existing repair state.
- An unreadable file or directory during inventory construction fails the indexing task with the exact path in the error message.
- A corrupt JSON cache is treated as stale/absent and does not prevent the launcher from starting.
- Cache-write failure is reported in activity history but the freshly built in-memory inventory remains usable for the current session.
- Integer byte totals use saturating addition.
- No failure path writes into the Diablo III installation.

## Testing

`sanctuary-casc` unit tests cover:

- typed `.build.info` headers and common name variants;
- preservation of unknown columns;
- empty/malformed `.build.info` errors;
- storage-kind classification;
- deterministic inventory ordering and byte totals;
- cache round-trip;
- schema-version rejection;
- fingerprint invalidation when a Data file size changes.

`sanctuary-launcher` tests cover:

- cached/current inventory produces a ready indexed state;
- stale/missing cache produces an index action;
- completed index events update model statistics;
- failed index events preserve the installation path and surface the error.

The full verifier remains:

- `cargo fmt --all -- --check`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo build --workspace --release`

## Compatibility Baseline

v0.2.0 carries forward the fixes proven on the user's Arch system:

- Rust/Clippy-clean derived `Default` for `InstallState`;
- boxed large `LauncherEvent::ProbeComplete` payload;
- egui 0.35 `Panel` API instead of removed `TopBottomPanel`/`SidePanel` types;
- egui 0.35 `set_theme` + `style_mut_of` styling path;
- Clippy-clean closure and candidate-discovery code.

## Release Criteria

v0.2.0 is ready when:

- the workspace reports version `0.2.0`;
- a synthetic Diablo III installation can be indexed and cached deterministically;
- a changed synthetic Data file invalidates the cache;
- the launcher displays structured build/storage information and a searchable Content page;
- all v0.1 launcher/engine behavior remains available;
- the verifier passes on the user's Arch environment with Rust 1.97.x;
- production manifests and packaging contain no compatibility-layer dependencies.
