# ForgeClean v0.2.0 Conditional Offload Design

## Goal
ForgeClean remains an automatic permanent package-cache cleaner when no mounted external storage is detected. When a mounted external drive is detected, superseded package archives and signature sidecars are offloaded to that drive instead of discarded; stale partial downloads remain cleanup-only junk: copy, SHA-256 verify, fsync, revalidate source identity, then direct-unlink the source.

## Mode selection
- `forgeclean auto --yes` scans `/var/cache/pacman/pkg` using the existing retention rules and installed-version pinning.
- If no eligible external filesystem is mounted, mode is `AUTO_CLEAN` and the verified batch is permanently deleted with the existing manifest guards.
- If an eligible external filesystem is mounted, mode is `AUTO_OFFLOAD` and batch entries are copied beneath `<mount>/AetherForge/ForgeClean/Pacman/`.
- External detection is Linux/AetherForge-specific and uses `lsblk --pairs` data. A mounted filesystem is eligible when its device or ancestor block device is removable/hotplug or reports an external transport such as USB/Thunderbolt/FireWire.
- The selected destination must reside on a different filesystem device than the Pacman cache.

## Transfer transaction
For every batch entry:
1. Verify the source still matches the manifest identity and remains under the approved cache root.
2. Reject destination symlinks and never overwrite a conflicting destination file.
3. Copy into a temporary file in the final destination directory while hashing the source.
4. `sync_all` the temporary file, rename it to the final filename, then hash the final destination.
5. Require source and destination SHA-256 to match and sync the destination directory.
6. Reverify the source identity after the copy.
7. Permanently unlink the source only after all checks pass.
8. On any failure, preserve the source and report the failure. Temporary files are removed best-effort.

## Scope
v0.2.0 offloads only superseded package archives and their signature sidecars. Stale partial package downloads remain cleanup-only junk. It does not move active packages, installed-version-pinned archives, arbitrary home files, Rust source trees, or user documents. Multi-drive pairing, restore, GUI, and background event services are future increments.

## CLI
- `forgeclean storage-status` reports whether eligible external storage is detected.
- `forgeclean auto --yes [--keep N] [--partial-age-days N]` performs one automatic run.
- `auto` refuses to execute without `--yes` because both branches can remove source files.
- Existing `scan-system`, `scan`, `verify`, and `purge` behavior remains available.

## Verification
Host build must prove no-external mode cleans a disposable fake cache; external mode offloads a disposable fake cache to a fake external destination through the library API; checksum mismatch/source mutation preserve the source; and legacy package-cache tests remain green.
