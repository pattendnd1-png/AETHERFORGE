# On Device profile contract — v0.1.23

## Authority rule

A successful read of BEACN Mic memory activates `On Device` as a **hardware-authoritative, read-only** state. The imported parameter model is for UI/state provenance. It is not immediately replayed into the private DSP path.

This release intentionally avoids a duplicate chain:

`BEACN onboard DSP -> Linux BEACN capture -> same profile again in private DSP`

Instead, On Device mode is:

`BEACN onboard DSP -> Linux BEACN capture`

with the private AetherForge DSP worker bypassed.

## Getter-only read path

The reader enumerates the BEACN Mic, opens the vendor interface through `beacn-lib` v0.4.3, generates fetch/getter messages for the detected firmware, and maps supported replies into `SoftwareDspState`. No setter path is intentionally used by this module.

## Same-mic cache fallback

A cached snapshot may be used only when its stored serial matches the connected mic identity, including the prefixed Linux representation. Cache fallback is visibly labeled and does not authorize private-DSP replay of the hardware profile.

## Local overlay

`CREATE LOCAL OVERLAY` exits hardware-authoritative mode without writing the microphone. It starts from a flat AetherForge `SoftwareDspState::default()` baseline and then starts the isolated private DSP source. This is an additive local processing lane, not a second copy of the imported hardware chain.

## Failure behavior

If no mic-memory snapshot can be obtained and no same-mic cache is valid, the app retains the current local profile and may start the private DSP path. It never invents an On Device snapshot.
