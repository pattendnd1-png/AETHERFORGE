# AETHERFORGE
My repo for my custom branded and themed Linux Distro

<!-- AETHERFORGE:AUTO:START -->
> [!IMPORTANT]
> **AETHERFORGE is a work-in-progress RUST reforge.** The post-reset OS tree uses **Garuda Linux / Arch Linux as its backbone** while AETHERFORGE replaces/reforges the user-facing OS layer, services, tooling, integration, and first-party applications. This repository tracks active development; it is not a claim of a finished or fully qualified distro release.

## Live development status

- **OS backbone:** Garuda Linux / Arch Linux
- **Development model:** Work-in-progress RUST reforge
- **Post-reset baseline:** `2026-08-18`
- **OS/source files tracked:** 232
- **Application source files tracked:** 3097
- **Current release/build artifacts indexed:** 108
- **Large build outputs:** GitHub prerelease `wip-post-reset`

## Active source tree

| Component | Version/status | Rust files |
| --- | --- | ---: |
| `AetherAI` | 0.3.7 | 123 |
| `AetherBrowser` | 2.1.60 | 81 |
| `Darkstone-RS` | 0.1.0 | 14 |
| `ForgeHX` | WIP | 67 |
| `OpenDeck` | 2.0.36 | 12 |
| `OpenSanctuary` | 0.4.2 | 40 |
| `ReForge-Logitech` | 0.6.1 | 37 |

## Repository layout

- `os/` — Garuda-backed AETHERFORGE OS overlay, system configuration, services, integration and reproducibility manifests.
- `apps/` — current post-reset first-party application source trees.
- `artifacts/` — checksums/index for evolving builds; large runtime/model/toolchain assets are externalized to the WIP GitHub prerelease with `LARGE-FILES.tsv` and a restore helper.
- `archive/post-reset-chat-and-build-files/` — current-era scripts, patches, verification files and handoff artifacts recovered from local development output.
- `migration/` — public migration/source mapping without private machine paths.

## Migration policy

Current post-reset source wins. Pre-reset RC3/v4/v5/v6/v7 hostile-takeover and clean-rebase lineages are not imported into the active tree. Transient caches and private credentials are never committed.
<!-- AETHERFORGE:AUTO:END -->
