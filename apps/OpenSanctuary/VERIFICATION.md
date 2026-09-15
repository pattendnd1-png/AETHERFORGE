# OpenSanctuary v0.4.2 Verification

The authoritative release gate is `./BUILD-ON-ARCH.sh` on the target Arch Linux system. It normalizes Rust formatting and then executes `scripts/verify.sh`.

## Compiler gate

The verifier requires:

```text
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --release
```

Both `opensanctuary-launcher` and `opensanctuary-engine` must be ELF binaries.

## Existing bridge/security gates

v0.4.2 retains the established v0.3.x contracts:

- compatibility-runtime code remains isolated to `crates/sanctuary-battlenet`;
- no raw Battle.net password/2FA/recovery-code capture;
- software-bridge persistence/repair remains client-first and Diablo-install independent;
- network bridge independently probes DNS, TCP/443, and certificate-validated TLS;
- the seamless preparation policy remains the only launcher policy path for INSTALL, SIGN IN, OPEN BATTLE.NET, and PLAY;
- X11 Battle.net host storage remains boxed to avoid `clippy::large-enum-variant`.

## v0.4.2 native CASC gate

The release must contain:

```text
crates/sanctuary-casc/src/build.rs
crates/sanctuary-casc/src/key.rs
crates/sanctuary-casc/src/index.rs
crates/sanctuary-casc/src/archive.rs
crates/sanctuary-casc/src/blte.rs
crates/sanctuary-casc/src/storage.rs
```

Required public surfaces include `EncodingKey`, `EncodingKeyPrefix`, `BuildConfig`, `LocalIndex`, `ArchiveReader`, `BlteDecodeOptions`, `decode_blte`, and `CascStorage`.

The transport modules must not write, rename, remove, or truncate files in the Diablo III installation. Existing inventory cache writing in `lib.rs` remains allowed because it targets OpenSanctuary's XDG cache, not the game installation.

BLTE must support `N`, `Z`, and recursive `F`; `E` must return an explicit unsupported-build error. Output and recursion limits must be present, and table-based chunks must support MD5 verification.

Synthetic tests must cover:

- key parsing;
- build-config sharding and field parsing;
- deterministic local-index file selection;
- archive index/offset/size derivation;
- archive bounds and envelope validation;
- raw, zlib, recursive, digest-failure, encrypted, output-limit, and recursion-limit BLTE cases;
- end-to-end `CascStorage` local EKey read.

## Packaging gate

The canonical source identity is `0.4.2`. Revision/RC suffixes are forbidden. Release artifacts are one ZIP, one TAR.GZ, and a direct `0.4.1 → 0.4.2` patch.

The ChatGPT build container used to assemble the package does not include Cargo/Rust, so its verification is limited to static source/package checks. The release is compiler-qualified only after `BUILD-ON-ARCH.sh` exits successfully on the target Arch system.
