# HyperX Microphone Firmware in ForgeHX 0.5.0

ForgeHX treats microphone firmware as a privileged, model-specific operation. Detection is not permission to flash.

## Support levels

- **InventoryOnly** — ForgeHX recognizes the microphone family and can report firmware-support metadata. No updater is enabled.
- **ValidatedPackages** — ForgeHX can parse and validate a model-specific package format, but cannot write it.
- **UpdateCapable** — an exact model adapter has a qualified update transport and post-flash verification path.
- **RecoveryCapable** — the adapter additionally has a verified recovery procedure for its update-mode identity.

The first ForgeHX 0.5.0 firmware registry is deliberately conservative. Known HyperX microphone families can be inventoried and user-supplied firmware files can be staged immutably and hashed, but firmware writes remain disabled until each model has an exact, reproducible updater and recovery qualification.

## Safety contract

ForgeHX never bundles HyperX firmware images. Use an official HyperX package or a firmware file you explicitly supply. Staging copies a regular file into a ForgeHX-owned immutable staging location and records SHA-256 and size. Editing or deleting the original file does not change the staged bytes.

A package is never considered flashable from its filename alone. A future update-capable adapter must validate the exact target model, normal/update USB identities, hardware revision when required, package/container structure, integrity information, firmware version, and any vendor signature exposed by the format.

Firmware transactions are journaled through explicit states. A daemon restart may recover status but must never silently resume a `Flashing` state. The daemon is the sole authority; there is no raw firmware packet, raw feature-report, or arbitrary bootloader-write IPC command.

## CLI

```bash
forgehx mic firmware get '<device-id>'
forgehx mic firmware stage '<device-id>' ./firmware-file
forgehx mic firmware validate '<device-id>' '<staged-id>'
forgehx mic firmware begin '<device-id>' '<staged-id>'
forgehx mic firmware status '<transaction-id>'
forgehx mic firmware forget '<staged-id>'
```

The GUI exposes the same model under a HyperX microphone's **Firmware** tab. It displays the detected model, installed version when available, hardware revision when available, support level, staged package SHA-256, validation result, and update eligibility.

## Qualification rule

A microphone only becomes `UpdateCapable` after its real hardware path has been verified end-to-end: normal identity -> updater preflight -> update-mode identity -> bounded transfer -> finalize -> normal re-enumeration -> installed-version verification -> profile/DSP restoration. Until then ForgeHX refuses the write instead of guessing.
