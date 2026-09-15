# ForgeClean v0.4.0 ColdPack Design

## Goal

Replace new ColdStore writes with a deduplicating ColdPack format while preserving exact byte restoration, the existing persistent organizer, and restore compatibility with v0.3.2 `.fcold.tar.zst` archives.

## Storage layout

The canonical shared object store is `~/Downloads/ForgeClean/ColdStorage/.coldpack-store/objects/`. Logical project/category archives remain small `.fcoldpack` manifest files in their existing `Cold` directories. Chunk objects are addressed by SHA-256 and compressed independently with Zstd level 19.

## Chunking

Files are split with deterministic content-defined chunking using a rolling Gear-style hash. Boundaries use 64 KiB minimum, approximately 256 KiB average, and 1 MiB maximum chunk sizes. Identical chunks across different files and archives resolve to the same object path and are written once.

## Commit safety

ForgeClean hashes the source tree before packing, rejects symlinks, writes missing chunks atomically, verifies every referenced chunk by decompression + SHA-256, writes and fsyncs the manifest plus SHA-256 sidecar, re-hashes the source, and only then removes the source. Any mismatch preserves the source.

## Manifest

Each `.fcoldpack` JSON manifest records format version, source name/type, source tree SHA-256, logical bytes, relative directory/file paths, Unix mode, file size, and ordered chunk hash/length references. No absolute restore paths or links are stored.

## Restore

Restore verifies the manifest sidecar, parses and validates relative paths, reads chunk objects from the shared store, verifies each decompressed chunk hash and length, reconstructs files into a staging directory, then atomically moves top-level restored entries into the selected restore directory. Existing destinations are never overwritten.

## Compatibility

`restore` continues to recognize `.fcold.tar.zst` archives and uses the v0.3.2 tar+Zstd path for them. New archive operations and the persistent organizer write `.fcoldpack` manifests.

## Observability

Archive output reports logical bytes, newly stored compressed bytes, chunks written, chunks reused, and the resulting physical-write ratio for that archive operation. Ratios are measurements, never promises.
