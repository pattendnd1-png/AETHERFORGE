# AetherForge v3.0.5 — OBS Relative Centering Fix

Root cause fixed: OBS 31/32 scene collections store both absolute and relative scene-item transforms. The previous package used the correct absolute position `(0,0)` but an incorrect `pos_rel.x = -1.0`. On a 16:9 canvas OBS reconstructs the left edge from relative coordinates using `-1.777777...`, which caused the full-canvas visual to shift right.

Changes:
- Corrected `pos_rel` for every full-canvas package source in 1080p and 1440p collections.
- Reasserted matching `scale_ref` values for each canvas.
- Added regression validation for OBS relative transform coordinates.
- Carries forward v3.0.4 fixed visuals and v3.0.3 loader/install fixes.
- Warpfield remains removed.
