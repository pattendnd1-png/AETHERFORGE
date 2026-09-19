# Third-Party Notices

AetherForge BEACN Control v0.1.21 is a clean-room implementation. User-supplied BEACN Windows installers are static interoperability/UI reference material only and are not redistributed with this package. No proprietary BEACN binary, source, or artwork is bundled.

## beacn-lib

This release adds `beacn-lib` v0.4.3 from the community `beacn-on-linux/beacn-lib` project for read-only startup discovery and parameter fetches from supported BEACN Mic hardware. `beacn-lib` is MIT licensed. AetherForge v0.1.21 uses its generated fetch/get messages for the startup import path and does not intentionally issue setter messages from `src/on_device.rs`.

Project: https://github.com/beacn-on-linux/beacn-lib
License: MIT

The remaining runtime is built from Rust crates declared by `Cargo.toml` (including eframe/egui and their transitive dependencies) under their respective upstream licenses.
