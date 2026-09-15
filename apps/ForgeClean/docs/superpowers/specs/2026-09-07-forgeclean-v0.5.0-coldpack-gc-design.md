# ForgeClean v0.5.0 ColdPack Store Maintenance Design

## Goal

Add fail-closed, reference-aware maintenance for the shared ColdPack object store so unreferenced deduplicated chunks do not accumulate indefinitely, without risking any object still required by a `.fcoldpack` manifest.

## Safety model

ForgeClean v0.5.0 uses mark-and-sweep with a fixed 7-day quarantine. Garbage collection never deletes an object directly from the active `objects/` tree. An apply cycle first performs a complete integrity audit, then moves only unreferenced objects into `quarantine/<unix-seconds>/...`. A later apply cycle permanently removes quarantined objects only after they are at least 7 days old and a fresh complete audit still proves they are not required.

Any unreadable/malformed `.fcoldpack` manifest, checksum mismatch, invalid chunk reference, missing referenced object, corrupt referenced object, symlink in a protected store path, or malformed active object path blocks mutation for the entire GC cycle. Preview and audit report the failure but never mutate the store.

## Concurrency

The persistent organizer and GC share one store lock at `ColdStorage/.coldpack-store/maintenance.lock` using Rust's standard `std::fs::File` locking API (available since Rust 1.89; the host baseline is Rust 1.98). ColdPack archive creation takes an exclusive lock from object creation through manifest commit and source deletion. GC apply takes the same exclusive lock for audit + quarantine/purge. ColdPack restore and read-only audit/status take a shared lock. This prevents GC from moving a chunk in the archive commit window.

## Manifest mark phase

The authoritative manifest search root is `~/Downloads/ForgeClean`, recursively, excluding `.coldpack-store`. Every file ending in `.fcoldpack` is treated as a live manifest. Each manifest must pass its `.sha256` sidecar check and schema/path validation. The mark phase collects the unique `(sha256, uncompressed length)` chunk references and total logical bytes. If one hash appears with conflicting lengths, the audit fails closed.

## Object sweep phase

Active objects must use the existing layout:

`ColdStorage/.coldpack-store/objects/<first-two-hex>/<64-hex-sha256>.zst`

Every active object path is validated. Referenced objects are decompressed and checked against both expected SHA-256 and expected uncompressed length during full audit/GC. Active objects not present in the live mark set are GC candidates.

Apply renames candidates to:

`ColdStorage/.coldpack-store/quarantine/<unix-seconds>/<first-two-hex>/<64-hex-sha256>.zst`

Renames are same-filesystem and directory metadata is fsynced. No candidate is unlinked from the active tree.

## Quarantine purge

Quarantine age is derived from the numeric top-level timestamp directory. An object becomes purge-eligible at age >= 604800 seconds (7 days). A fresh complete mark/audit is mandatory before purge. If a hash is live again, its quarantined duplicate may be deleted only when a valid active copy exists and passes verification; otherwise the cycle fails closed. Empty quarantine directories may be removed after successful object purges.

## CLI

New commands:

- `forgeclean coldpack-status [--downloads PATH]`: read-only usage summary; manifest count, unique references, active objects/bytes, orphan candidates/bytes, quarantine objects/bytes, expired quarantine count/bytes, logical bytes, dedup/storage ratio. It validates manifest checksums/schema and store path structure but does not fully decompress every referenced object.
- `forgeclean coldpack-audit [--downloads PATH]`: shared-lock, full integrity audit of every manifest reference and referenced active object. No mutation.
- `forgeclean coldpack-gc --preview [--downloads PATH]`: full audit plus exact quarantine/purge candidate report; no mutation.
- `forgeclean coldpack-gc --apply [--downloads PATH]`: exclusive-lock full audit, quarantine new orphans, purge eligible quarantine objects, and report bytes/objects moved or deleted.

`coldpack-gc` requires exactly one of `--preview` or `--apply`.

## Persistent maintenance

The existing `forgeclean-organizer.service` remains the single persistent service. The `watch` loop runs an automatic GC apply at low priority no more than once per 24 hours, with the first maintenance cycle occurring after startup. GC failures are logged to stderr and do not stop normal download organization; because GC is fail-closed, a maintenance error causes no object mutation past the point guaranteed safe by the module.

## Compatibility

- Active project trees remain uncompressed.
- Existing v0.4.2 `.fcoldpack` manifests and object layout remain unchanged and fully compatible.
- Existing legacy `.fcold.tar.zst` restore support remains unchanged.
- External-drive offload and Pacman cleanup behavior remain unchanged.
- Verification TXT precreation/failure trapping from v0.4.2 remains mandatory.

## Verification

Rust tests cover: live/orphan accounting, preview non-mutation, quarantine-only apply, 7-day purge, corrupt manifest fail-closed, corrupt referenced object fail-closed, conflicting chunk lengths fail-closed, and store lock exclusion. Host E2E creates real ColdPack archives, removes one manifest to orphan unique chunks, confirms preview does not mutate, confirms apply moves only orphan chunks into quarantine, and confirms remaining live archive restores exactly. Static regression checks the fixed 7-day policy, shared lock integration in archive/restore/GC, persistent 24-hour maintenance hook, CLI surface, and v0.4.2 verification TXT guarantee.
