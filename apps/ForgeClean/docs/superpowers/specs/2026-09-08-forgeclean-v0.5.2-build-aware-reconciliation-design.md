# ForgeClean v0.5.2 Build-Aware Reconciliation Design

## Goal
ForgeClean must keep all recognizable project/build artifacts together by project and version, for both existing files and files that arrive later, while preserving old paths used by install/build commands.

## Canonical precedence
1. Project/build identity is authoritative.
2. Project identity without a version routes to project staging.
3. Generic type/category storage is fallback only when no safe project/build identity exists.
4. Active project source trees remain separate and are never cold-compressed.
5. Incomplete `.part`, `.partial`, and `.tmp` downloads are never promoted into build bundles.

## Canonical layout
`~/Downloads/ForgeClean/Projects/<Project>/Builds/<Version>/` contains all recognized artifacts for that build, including source archives, HIT-IT/install scripts, checksums, verification logs, packages, and supporting release files.

## Continuous reconciliation
Every `organize_once` and every persistent `watch` cycle performs two passes:
- reconcile existing live artifacts from legacy ForgeClean category buckets and project `Downloads/`/`Releases/` staging areas;
- process stable top-level Downloads entries.

The initial daemon cycle therefore migrates old misplaced live files; subsequent cycles capture newly arriving artifacts.

## Compatibility
Every moved build artifact gets a guarded symlink at its previous path under Downloads. ForgeClean never overwrites a real file or conflicting symlink. This allows old one-line build/install commands to keep resolving the same filename.

## Safety
- Never follow symlink candidates during reconciliation.
- Never overwrite a canonical destination.
- Respect the existing stable-file interval before moving an artifact.
- Leave transient/incomplete downloads outside build bundles.
- Do not infer a project/build identity when the version pattern is ambiguous.
- ColdPack, GC, external offload, Active project handling, and package-cleanup safety remain unchanged.

## Verification
Host gates must prove: cross-project identity parsing, top-level build-bundle routing, old category migration, later-arriving project staging migration, compatibility aliases, incomplete-download exclusion, full organizer suite, Clippy, all tests, release build, persistent service install/restart, and verification TXT generation.
