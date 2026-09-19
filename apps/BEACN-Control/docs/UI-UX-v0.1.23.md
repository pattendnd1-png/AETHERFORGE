# UI/UX contract — v0.1.23

The DragonGlass MIC MEMORY strip remains directly under the Live Profile bar.

## On Device state

When a live or same-mic cached On Device snapshot is loaded, the strip shows:

- `MIC MEMORY`
- `ON DEVICE ACTIVE` or `ON DEVICE CACHED`
- `READ ONLY`
- `HARDWARE DSP ACTIVE`
- `PRIVATE DSP BYPASSED`
- `SERIAL VERIFIED` for cache fallback
- firmware/import counts/serial
- `CREATE LOCAL OVERLAY`
- `RELOAD MIC`

Mic EQ/secondary processing controls are disabled while hardware-authoritative On Device mode is active. The app preserves the imported state for display and rejects accidental edit commits by restoring the imported baseline.

## Local overlay state

`CREATE LOCAL OVERLAY` changes the context to `On Device - Local Overlay`, changes the MIC MEMORY state to `LOCAL OVERLAY`, resets local DSP to a flat baseline, and starts the app-private DSP worker. The microphone's onboard memory remains unchanged.

## Local profiles

Loading a saved local profile leaves mic memory unchanged and starts or updates the private DSP worker. This closes the older gap where a profile could update the UI model without immediately synchronizing the running private-DSP state.

## Responsive behavior

The authority/status badges must wrap rather than overflow. Compact sizing and scrolling behavior from the existing adaptive UI contract remain unchanged.
