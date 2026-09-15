#!/usr/bin/env python3
from pathlib import Path
import sys
root = Path(sys.argv[1] if len(sys.argv) > 1 else '.')
path = root / 'crates/forgehx-dsp/src/noise_monitor.rs'
s = path.read_text()
assert 'let spectral_flux = spectral_flux(' not in s, 'local spectral_flux binding shadows spectral_flux() function'
assert 'let _scene_spectral_flux = spectral_flux(' in s, 'expected non-shadowing scene spectral flux binding'
assert 'let local_spectral_flux = spectral_flux(' in s, 'local post-subtraction spectral flux call must remain'
print('FORGEHX_10_0_30_SPECTRAL_FLUX_SHADOWING=PASS')
