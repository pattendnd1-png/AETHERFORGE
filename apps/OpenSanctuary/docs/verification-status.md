# OpenSanctuary v0.2.0 Verification Status

Date: 2026-08-14
Branch: `feature/v0.2.0`

## Compatibility baseline

The user's Arch Linux/Rust 1.97.x environment verified the corrected v0.1 baseline after applying:

- derived `Default` for `InstallState`;
- boxed large `LauncherEvent::ProbeComplete` payload;
- egui 0.35 `Panel` API;
- egui 0.35 `set_theme` + `style_mut_of` styling;
- Clippy-clean cache-root closure and install-candidate discovery logic.

v0.2 carries these corrections forward and adds boxed inventory result events as well.

## v0.2 static verification in the build sandbox

The build sandbox does not contain `cargo`, `rustc`, or `rustfmt`, so authoritative Rust compilation is deferred to the user's Arch host. Before packaging, the sandbox performs:

- TOML parsing for every Cargo manifest;
- workspace-member existence checks;
- shell syntax checks for verification and Arch packaging scripts;
- version consistency checks for `0.2.0`;
- source scans for removed egui APIs;
- source/package scans for compatibility-layer dependencies;
- brace/interface structural checks on the rewritten Rust modules;
- `git diff --check`;
- source archive generation and required-file inspection.

## Authoritative host verification

On the user's Rust-equipped Arch system, run:

```bash
cargo fmt --all
./scripts/verify.sh
```

`./scripts/verify.sh` then performs:

```bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --release
file target/release/opensanctuary-launcher target/release/opensanctuary-engine
```

It also verifies that both application binaries are ELF files and scans production manifests/source for forbidden compatibility-layer dependencies.
