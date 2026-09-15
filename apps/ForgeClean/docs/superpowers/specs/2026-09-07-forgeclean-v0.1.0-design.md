# ForgeClean v0.1.0 Design

## Goal
Build a Rust-native AetherForge cleanup utility that safely identifies superseded Pacman package archives and stale partial package downloads, groups them into a reviewable cleanup batch, and permanently deletes only the exact approved files without using the desktop Trash.

## Scope
v0.1.0 intentionally covers `/var/cache/pacman/pkg` style package caches only. It does not uninstall orphan packages, remove kernels, clean application data, or recursively delete arbitrary directories.

## Safety contract
- Scan only regular files; symlinks are rejected.
- Recognize Pacman archives ending in `.pkg.tar.zst`, `.pkg.tar.xz`, `.pkg.tar.gz`, or `.pkg.tar.lz4`, plus stale `.part`/`.partial` downloads.
- Group archives by package name and retain two versions by default.
- Prefer Arch `vercmp` for version ordering; when unavailable, fail closed rather than guessing for package-version deletion.
- Write an immutable manifest containing root, path, size, Unix device/inode, and modified timestamp.
- Purge re-canonicalizes the root and candidate parent, verifies candidate remains beneath the approved root, rejects symlinks, rechecks identity metadata, then calls direct filesystem unlink.
- No Trash integration and no recursive directory deletion.
- A changed or replaced file is skipped with an error instead of deleted.
- Dry-run/preview is the default workflow; purge requires an explicit `--yes` flag.

## CLI
- `forgeclean scan [--cache-dir PATH] [--keep N] [--manifest PATH]`
- `forgeclean purge --manifest PATH --yes`
- `forgeclean verify --manifest PATH`

## Output
Human-readable totals plus machine-friendly status lines such as `FORGECLEAN_SCAN=PASS`, `RECOVERABLE_BYTES=...`, and `FORGECLEAN_PURGE=PASS`.

## Testing
Use temporary cache directories. Tests cover filename parsing, retention selection, stale partial detection, manifest round-trip, path confinement, metadata-change refusal, symlink refusal, and successful direct deletion.
