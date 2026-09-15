# OpenSanctuary v0.2.3 Verification Status

Branch: `feature/v0.2.3-ui`

## Verified in the build sandbox

- all Cargo manifests parse as TOML;
- all declared workspace members exist;
- shell build/package scripts pass `bash -n`;
- `git diff --check` reports no whitespace errors;
- obsolete pre-egui-0.35 panel/style calls are absent;
- production Rust/Cargo manifests contain no Wine, Proton, DXVK, VKD3D, Winetricks, Bottles, or Lutris dependency references;
- workspace version, Arch `pkgver`, source-archive version, and build/verifier banners are `0.2.3`;
- launcher source delimiters are structurally balanced;
- v0.2.3 contains the procedural selected-game tile, featured carousel, notification badge, separate Friends/Local Account flyouts, and reduced-motion-aware downloads pulse.

## Not executable in this sandbox

This environment does not provide `cargo`, `rustc`, or `rustfmt`, so Rust formatting, unit tests, Clippy, and release compilation cannot be truthfully marked as passing here.

Run `./BUILD-ON-ARCH.sh` on the Rust-equipped Arch target. It normalizes formatting and then runs the strict verifier: workspace tests, Clippy with `-D warnings`, release build, ELF checks, and compatibility-layer scan.
