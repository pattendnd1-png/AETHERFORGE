# ForgeHX 10.0.31 — Playback Priority / False Speech Protection Fix

ForgeHX 10.0.31 fixes the live Voice-Only Capture failure where correlated music/game playback could be mislabeled as local speech and therefore bypass hard rejection. The physical endpoint remains `HyperX SoloCast 2 Analog Stereo`; AetherStream remains the sole app-facing system microphone publisher at `aetherstream.system.microphone`.

## 10.0.31 changes

- Treats spectral `speech_protected` as advisory when the dedicated playback guard or correlated subtraction has already confirmed system playback.
- Keeps an enrolled-speaker match authoritative so the user's confirmed voice remains open during real double-talk.
- Adds regressions for music-only false speech protection, confirmed user voice + playback, and weak playback evidence.
- Preserves the existing 2000 ms effective playback history and automatic acoustic delay alignment.
- Keeps AetherStream as the system audio graph/default-routing owner and ForgeHX as mic DSP owner.
- Uses a Downloads-backed temporary workspace for host build/verification so a full `/tmp` cannot block the release gate.

## Routing invariants

- Preserved physical endpoint: `HyperX SoloCast 2 Analog Stereo`
- Processed source: `aetherstream.system.microphone`
- No default source/sink mutation.
- No additional app-facing virtual microphone.
- Existing `AFXHXM01` ForgeHX → AetherStream bridge remains intact.
- DSP errors remain fail-closed to silence per frame.

The installer writes `~/Downloads/ForgeHX-10.0.31-VERIFY.txt`. Native Rust tests/build and live runtime state remain blocking on the target host.
