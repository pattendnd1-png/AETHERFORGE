# OpenSanctuary v0.2.1 Verification Status

Branch: `feature/v0.2.1-ui`

## Verified in the packaging sandbox

- `git diff --check feature/v0.2.0..HEAD`
- all 13 Cargo manifests parse as TOML;
- all 12 workspace members have a Cargo manifest;
- workspace version is `0.2.1`;
- `BUILD-ON-ARCH.sh`, `scripts/verify.sh`, and `packaging/arch/make-source.sh` pass `bash -n`;
- obsolete pre-egui-0.35 `TopBottomPanel`, `SidePanel`, and `ctx.style_mut(` calls are absent from production source;
- production Cargo/Rust source contains no Wine/Proton/DXVK/VKD3D/Winetricks/Bottles/Lutris dependency references;
- Rust source delimiter scan is balanced;
- `packaging/arch/OpenSanctuary-0.2.1.tar.gz` is generated and contains the required launcher, workspace, build helper, and PKGBUILD files.

## Must be verified on the Rust-equipped Arch target

This packaging sandbox does not contain `cargo` or `rustc`, so it cannot prove compilation, tests, rustfmt, or Clippy status. Run:

```bash
./BUILD-ON-ARCH.sh
```

That executes formatting normalization followed by the strict workspace verifier.
