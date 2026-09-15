# ForgeHX HyperX Microphone Firmware Manager Design

## Status

Approved architecture, written 2026-08-23 for ForgeHX 0.4.0.

## Goal

Add safe, first-class HyperX microphone firmware management to ForgeHX without creating a generic raw firmware flasher. ForgeHX must identify the exact microphone and hardware revision, report installed firmware, validate a staged update package, isolate audio/DSP state before update, invoke only a model-specific verified updater, wait for USB re-enumeration, verify the resulting firmware identity, and restore the user's microphone profile.

## Release identity

This work ships under the canonical semantic version `ForgeHX 0.4.0` with Arch `pkgrel=1`.

Release names and artifacts must not use RC, revision, r-number, candidate, corrected, hotfix-copy, or duplicate release suffixes. Canonical artifacts are:

```text
ForgeHX-0.4.0-source.tar.gz
ForgeHX-0.4.0-Arch-BuildKit.tar.gz
forgehx-0.4.0-1-x86_64.pkg.tar.zst
```

## Scope

The firmware manager applies only to HyperX microphones recognized by ForgeHX's microphone registry. Initial model families are the current microphone matrix already defined by the HyperX microphone full-stack design:

- FlipCast
- SoloCast 2
- QuadCast 2 S
- SoloCast
- DuoCast
- QuadCast S
- QuadCast 2

Future HyperX microphone models enter through the same registry and firmware-policy mechanism.

## Capability levels

Firmware support is split into independently truthful levels:

1. **Inventory** — ForgeHX can identify the microphone, hardware revision when exposed, and installed firmware version when exposed through standard USB descriptors or a verified read-only vendor query.
2. **Package validation** — ForgeHX can parse a model-specific update manifest/container and prove that a selected payload targets the exact supported model/revision.
3. **Staging** — ForgeHX can hash and stage the selected package without writing device firmware.
4. **Update capable** — ForgeHX has a verified model-specific transport, boot/update identity, chunk/ack protocol, completion semantics, re-enumeration behavior, and post-flash verification path.
5. **Recovery capable** — ForgeHX additionally has a verified recovery-mode identity and safe recovery procedure for interrupted/failed updates.

A model may expose inventory and validation while remaining non-flashable. The UI must never label such a model `Update Capable`.

## Non-goals and hard prohibitions

- No generic raw `.bin`, `.hex`, `.ufw`, `.dfu`, or arbitrary byte-stream flasher.
- No firmware write path through generic HID IPC.
- No firmware flashing based only on product name or broad HyperX/HP vendor ID.
- No guessed bootloader commands, guessed packet framing, guessed CRC, or guessed completion status.
- No downloading or redistributing proprietary HyperX firmware binaries inside ForgeHX release artifacts.
- No automatic downgrade.
- No background automatic flashing.
- No update attempt while the microphone capture source is in active use.
- No update attempt while an active ForgeHX DSP graph is attached to the target microphone.
- No update if the target's current identity or post-transition identity is ambiguous.

## Firmware package source policy

ForgeHX owns the updater and validation logic, not the proprietary firmware payload.

A firmware payload may enter ForgeHX through:

- a file explicitly selected/imported by the user;
- an official HyperX package URL that ForgeHX records as source metadata when a future official machine-readable feed is supported; or
- a future Linux firmware service such as fwupd/LVFS if HyperX publishes compatible metadata.

ForgeHX release tarballs/packages never bundle HyperX firmware images.

Every staged payload records:

```text
source_kind
source_label
original_filename
sha256
size_bytes
parsed_format
claimed_model
claimed_hardware_revision
claimed_firmware_version
validation_result
```

## Architecture

### New `forgehx-firmware` crate

Responsibilities:

- firmware package hashing and staging;
- typed firmware identities and state machine;
- exact device/adapter registry;
- package validators;
- model-specific updater traits;
- update transaction journal;
- re-enumeration matching;
- post-flash verification;
- recovery metadata.

Suggested module layout:

```text
crates/forgehx-firmware/
  src/lib.rs
  src/package.rs
  src/staging.rs
  src/transaction.rs
  src/registry.rs
  src/adapters/
    mod.rs
    inventory_only.rs
    solocast.rs
    solocast2.rs
    quadcast_s.rs
    quadcast2.rs
    quadcast2s.rs
    duocast.rs
    flipcast.rs
```

Adapters remain inventory-only until their write protocol is proven.

### Core types

`forgehx-core` adds firmware-specific capabilities and state rather than overloading Diagnostics.

```rust
pub enum Capability {
    // existing variants...
    MicFirmwareInventory,
    MicFirmwareUpdate,
    MicFirmwareRecovery,
}

pub enum FirmwareSupportLevel {
    InventoryOnly,
    ValidatedPackages,
    UpdateCapable,
    RecoveryCapable,
}

pub struct FirmwareIdentity {
    pub device_id: DeviceId,
    pub model: String,
    pub hardware_revision: Option<String>,
    pub firmware_version: Option<String>,
    pub bootloader_version: Option<String>,
    pub support_level: FirmwareSupportLevel,
}

pub struct FirmwarePackageInfo {
    pub original_filename: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub parsed_format: String,
    pub claimed_model: Option<String>,
    pub claimed_hardware_revision: Option<String>,
    pub claimed_firmware_version: Option<String>,
    pub validation: FirmwareValidation,
}

pub enum FirmwareValidation {
    Valid,
    UnsupportedFormat,
    ModelMismatch,
    HardwareRevisionMismatch,
    VersionUnknown,
    HashMismatch,
    SignatureInvalid,
    AmbiguousTarget,
}
```

Exact serde representations are stable IPC data and therefore must be versioned with the protocol.

### Model-specific updater interface

Firmware transports implement a sealed internal trait. No IPC call accepts a raw endpoint, report ID, packet, or arbitrary byte buffer.

```rust
pub trait MicFirmwareAdapter {
    fn identity(&self, device: &DeviceInfo) -> Result<FirmwareIdentity>;
    fn validate_package(&self, device: &DeviceInfo, package: &StagedFirmware) -> Result<FirmwarePackageInfo>;
    fn prepare(&self, device: &DeviceInfo, package: &StagedFirmware) -> Result<PreparedUpdate>;
    fn flash(&self, prepared: PreparedUpdate, progress: &dyn FirmwareProgressSink) -> Result<FlashResult>;
    fn verify(&self, expected: &ExpectedFirmware, rediscovered: &DeviceInfo) -> Result<FirmwareIdentity>;
}
```

Only adapters whose `flash` protocol is verified may advertise `MicFirmwareUpdate`.

## Transaction state machine

The firmware manager uses an explicit persisted transaction state:

```text
Idle
  -> PackageStaged
  -> Validated
  -> PreflightPassed
  -> AudioDetached
  -> EnteringUpdateMode
  -> AwaitingUpdateDevice
  -> Flashing
  -> Finalizing
  -> AwaitingNormalDevice
  -> Verifying
  -> RestoringProfile
  -> Completed
```

Terminal failure states record the last verified stage:

```text
Rejected
PreflightFailed
UpdateDeviceMissing
FlashFailed
NormalDeviceMissing
VerificationFailed
RecoveryRequired
```

A daemon restart may read the journal and report the interrupted state, but it must not automatically resume firmware writes.

## Preflight

Before any update-capable adapter is allowed to write:

1. Exact logical device identity is resolved to one physical USB parent.
2. Exact VID/PID and required interface/usage match the adapter registry.
3. Hardware revision matches the package validator when revision data exists.
4. Package SHA-256 is recomputed from the staged bytes.
5. Package format/manifest is parsed by that model's validator.
6. Target microphone is not used by active capture clients.
7. ForgeHX input DSP graph is bypassed and detached from the target source.
8. No second matching target creates ambiguous re-enumeration.
9. The updater has a known post-transition update/boot identity.
10. Downgrade is rejected unless a model policy explicitly declares that exact transition safe and the user has enabled downgrade.

## Audio and DSP isolation

Firmware updates must not leave a processed virtual microphone pointing at a disappearing source.

Before entering update mode, the daemon:

- captures the active ForgeHX mic profile name;
- bypasses and tears down the `ForgeHX Processed Mic` graph for that device;
- refuses the update if active capture clients remain after the user-requested update preflight;
- records the raw PipeWire/USB association information needed for rediscovery.

After successful verification, the daemon rediscovers the physical source and reapplies the previous ForgeHX microphone DSP/profile only after the microphone is stable.

## Re-enumeration matching

An update adapter defines both normal and update-mode identities when the device changes identity during flashing. Matching may use exact combinations of:

- VID/PID;
- serial number;
- USB physical port/path;
- HID usage page/usage;
- interface number;
- bootloader descriptor/version.

The updater refuses to proceed if more than one candidate matches.

## Integrity and authenticity

Validation always includes SHA-256 of staged bytes.

If a HyperX package/container exposes a vendor signature, signed manifest, CRC, or checksum, the adapter verifies it before update. ForgeHX never strips or bypasses a known integrity field.

SHA-256 proves staging integrity, not vendor authenticity. The UI must distinguish `Hash verified` from `Vendor signature verified`.

## Installed firmware readback

Inventory first uses standard USB `bcdDevice`/descriptor values when they accurately encode device firmware. A model adapter may augment this with a verified read-only vendor query.

ForgeHX labels the origin:

```text
USB descriptor
HyperX vendor status report
Bootloader descriptor
Unknown
```

A numeric descriptor value must not be presented as a semantic firmware version unless the adapter defines that mapping.

## IPC and daemon authority

Firmware operations are daemon-only and privileged relative to normal controls.

Proposed IPC commands:

```rust
MicFirmwareGet { protocol_version, device_id }
MicFirmwareStage { protocol_version, device_id, path }
MicFirmwareValidate { protocol_version, device_id, staged_id }
MicFirmwareBegin { protocol_version, device_id, staged_id }
MicFirmwareStatus { protocol_version, transaction_id }
MicFirmwareForget { protocol_version, staged_id }
```

The client never supplies raw HID packets, report IDs, endpoint numbers, or boot commands.

Staging paths must resolve to regular files, reject symlinks that escape the selected file after open, and copy bytes into a ForgeHX-owned staging directory before validation/update.

## CLI

```text
forgehx mic firmware get <device-id>
forgehx mic firmware stage <device-id> <file>
forgehx mic firmware validate <device-id> <staged-id>
forgehx mic firmware update <device-id> <staged-id>
forgehx mic firmware status <transaction-id>
forgehx mic firmware forget <staged-id>
```

`update` remains unavailable when the adapter is not `UpdateCapable`.

## GUI

HyperX microphone devices gain a `Firmware` page with:

- microphone model;
- hardware revision;
- installed firmware version and source;
- updater support level;
- selected/staged package;
- SHA-256;
- claimed firmware version;
- model/revision compatibility;
- integrity/signature status;
- update button only for `UpdateCapable` adapters;
- explicit preflight failures;
- transaction progress;
- post-update verification result;
- recovery status when applicable.

The page must show `Firmware inventory only` instead of an enabled update button for unverified adapters.

## Recovery policy

Recovery is opt-in and model-specific. `MicFirmwareRecovery` is advertised only when the update-mode/recovery identity and safe recovery sequence are verified.

A failed update that leaves the microphone in an unknown state produces diagnostic instructions and preserves the transaction journal; ForgeHX must not improvise recovery writes.

## Firmware research lane

The 0.4.0 codebase includes the complete safe manager even if some model adapters are inventory-only. Actual `MicFirmwareUpdate` is enabled model-by-model only after all of the following are evidenced:

- update package format;
- exact target/revision constraints;
- update-mode transition;
- update-mode USB/HID identity;
- framing/chunk size;
- per-chunk acknowledgement/error behavior;
- checksum/CRC/signature semantics;
- finalize/reboot command;
- expected normal re-enumeration;
- post-flash version readback;
- interruption/recovery behavior.

## Testing

Pure/unit tests must cover:

- package SHA-256 and immutable staging;
- exact model/revision matching;
- rejection of broad `03f0` matches;
- downgrade policy;
- transaction state transitions;
- ambiguous re-enumeration rejection;
- inactive/update-capable adapter gating;
- protocol-version projection;
- serialization of firmware state;
- no raw-write firmware IPC.

Static package guards must verify:

- canonical version is `0.4.0` and `pkgrel=1`;
- no RC/r/revision artifact naming;
- firmware binaries are not bundled;
- no generic `flash_raw`/`write_firmware_bytes` IPC surface;
- microphone input-only DSP invariants remain intact;
- mouse support from the previous release remains present.

Real hardware qualification for an update-capable adapter requires a sacrificial or recovery-capable target before normal release qualification. Source/package tests alone never qualify a firmware writer as safe.

## Relationship to prior microphone design

This design supersedes only the earlier microphone design's `No firmware flashing in this release family` non-goal. All microphone input-only DSP, hardware capability truthfulness, and backend-ownership rules remain in force.
