# OpenSanctuary v0.2.2 Verification Status

Branch: `feature/v0.2.2-ui`

## Verified in the packaging sandbox

- all 13 Cargo manifests parse as TOML;
- all 12 workspace members resolve to a Cargo manifest;
- workspace version is `0.2.2` and Arch `pkgver` is `0.2.2`;
- `BUILD-ON-ARCH.sh`, `scripts/verify.sh`, and `packaging/arch/make-source.sh` pass `bash -n`;
- `git diff --check` is clean;
- obsolete pre-egui-0.35 panel/style symbols are absent from production source;
- production source/manifests contain no Wine/Proton/DXVK/VKD3D/Winetricks/Bottles/Lutris dependencies or references;
- the launcher contains the v0.2.2 HOME/GAMES/SHOP chrome, favorites strip, selected Diablo control column, local notification/social drawer, and downloads/activity strip.

## Must be verified on the Rust-equipped Arch target

This sandbox does not contain `cargo`, `rustc`, or `rustfmt`, so it cannot truthfully claim Rust compilation, tests, Clippy, or the release ELF build passed. Run:

```bash
./BUILD-ON-ARCH.sh
```

That command normalizes formatting and then runs workspace tests, Clippy with `-D warnings`, the release build, ELF checks, and the compatibility-layer scan.
