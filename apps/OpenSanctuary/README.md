# OpenSanctuary v0.4.2

OpenSanctuary is an open-source Linux launcher plus a clean-room native runtime foundation for a **user-owned Diablo III installation**. v0.4.2 is the second Rust 1.98 strict-Clippy compatibility correction for the native CASC foundation: it preserves the v0.4.1 `as_chunks` index parser fix and collapses the BLTE optional-size validation into a let-chain required by `clippy::collapsible_if`. The native CASC transport behavior is unchanged.

## What v0.4.2 adds

The new `sanctuary-casc` transport is strictly read-only against the Diablo III installation and is split into focused Rust modules:

```text
crates/sanctuary-casc/src/
├── lib.rs       # existing inventory/cache compatibility API + re-exports
├── build.rs     # .build.info + local build configuration
├── key.rs       # 16-byte EKeys + 9-byte local-index prefixes
├── index.rs     # local *.idx bucket selection + EKey lookup
├── archive.rs   # bounded data.NNN reads + 30-byte CASC envelope
├── blte.rs      # bounded BLTE N/Z/F decoding + chunk MD5 checks
└── storage.rs   # read-only CascStorage facade
```

The native read path is now:

```text
.build.info
    ↓
local build configuration
    ↓
Encoding Key (EKey)
    ↓
local Data/data/*.idx
    ↓
archive index + offset + encoded size
    ↓
data.NNN
    ↓
30-byte CASC envelope
    ↓
BLTE
    ↓
decoded logical bytes
```

### Local index

- selects the lexicographically newest `.idx` file for each `00`–`0f` bucket;
- parses 18-byte local EKey records;
- maps the first 9 EKey bytes to archive index, offset, and encoded size;
- uses first-entry-wins behavior for duplicate prefixes;
- rejects truncated, misaligned, non-positive, or overflowing records.

### Archive reader

- opens `Data/data/data.NNN` read-only;
- validates offset + encoded-size bounds before allocation;
- caps one encoded record at 512 MiB;
- validates the 30-byte CASC envelope and index size;
- accepts direct or byte-reversed envelope EKey representation;
- returns only the contained BLTE blob.

### BLTE decoder

Supported in v0.4.2:

- `N` — uncompressed chunks;
- `Z` — zlib chunks via the pure-Rust default `flate2` backend;
- `F` — nested BLTE with a recursion-depth limit;
- chunk-table MD5 verification;
- maximum decoded-output limits and checked size arithmetic.

Explicitly unsupported:

- `E` encrypted BLTE chunks. OpenSanctuary will not guess, scrape, or bypass keys. Encrypted data remains unavailable until a legitimate key source is available.

### CascStorage

`CascStorage::open()` composes the build config, local index, archive reader, and BLTE decoder. It can:

- read a known local EKey;
- read a build-config-addressed blob such as the stored `encoding` object;
- expose build metadata and selected index files;
- preserve the existing v0.3.x inventory/probe API for launcher compatibility.

## Battle.net integration retained

v0.4.2 keeps the existing canonical Battle.net integration:

- in-app software bridge repair;
- independent DNS/TCP/TLS network bridge health;
- X11 hosted Battle.net surface with managed Wayland companion mode;
- one session-preparation policy for INSTALL, SIGN IN, OPEN BATTLE.NET, and PLAY;
- Diablo III install/update/authentication still handled by Blizzard-controlled Battle.net surfaces;
- no raw Blizzard password, 2FA code, recovery code, or captcha capture.

The official Battle.net download handoff remains:

```text
https://download.battle.net/en-us/desktop
```

Typical custom runtime overrides remain:

```bash
export OPENSANCTUARY_WINE=/usr/bin/wine
export WINEPREFIX="$HOME/Games/battlenet"
export OPENSANCTUARY_BATTLENET_EXE="$WINEPREFIX/drive_c/Program Files (x86)/Battle.net/Battle.net Launcher.exe"
```

Compatibility-runtime implementation remains isolated to `crates/sanctuary-battlenet`; the native engine/CASC/assets/render/audio crates remain independent from it.

## What v0.4.2 does not do yet

v0.4.2 does **not** parse the decoded encoding table, map CKeys to EKeys, parse the Diablo III root manifest, resolve logical asset names, render Diablo III assets, implement gameplay, bypass Blizzard account/license checks, scrape Blizzard CDNs, or automate Battle.net credentials.

Planned native-data sequence:

```text
v0.4.2  local index + archive + BLTE transport (Rust 1.98 compiler correction)
v0.4.3  encoding table / CKey → EKey
v0.4.4  Diablo III root manifest
v0.4.5  native asset catalog
v0.4.6  first decoded asset consumed by the engine
```

## Build and activate on Arch Linux

Install native development requirements:

```bash
sudo pacman -S --needed base-devel rust cargo libx11 libxcb libxkbcommon wayland vulkan-icd-loader pipewire xdg-utils
```

Build and run the strict verifier:

```bash
./BUILD-ON-ARCH.sh
```

Install the release binaries into your user account, register the desktop entry, and launch OpenSanctuary:

```bash
./INSTALL-ACTIVATE-ON-ARCH.sh
```

The verifier runs:

```text
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --release
```

and then checks ELF outputs, compatibility boundaries, credential boundaries, Battle.net bridge contracts, and the v0.4.2 native CASC transport contract.

## Safety and ownership boundary

OpenSanctuary does not bundle Blizzard executables, installers, artwork, credentials, or game data. Native CASC reads are local and read-only. Users must own and install Diablo III through legitimate Blizzard channels.

## License

GPL-3.0-or-later. See `LICENSE`.
