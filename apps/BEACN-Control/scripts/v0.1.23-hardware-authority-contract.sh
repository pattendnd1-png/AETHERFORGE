#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
MAIN="$ROOT/src/main.rs"
UI="$ROOT/src/ui_ux.rs"
fail(){ echo "AETHERFORGE_BEACN_V0_1_23_HARDWARE_AUTHORITY=FAIL:$1"; exit 1; }

for needle in \
  'ON_DEVICE_DSP_AUTHORITY=HARDWARE' \
  'PRIVATE_DSP_START_POLICY=LOCAL_ONLY' \
  'fn activate_local_overlay(&mut self)' \
  'self.dsp = SoftwareDspState::default();' \
  'self.on_device_dsp_baseline = Some(self.dsp.clone());' \
  'if self.on_device_ui.is_loaded() {' \
  'use CREATE LOCAL OVERLAY'; do
  grep -Fq "$needle" "$MAIN" || fail "missing:${needle// /_}"
done
for needle in 'HARDWARE DSP ACTIVE' 'PRIVATE DSP BYPASSED' 'CREATE LOCAL OVERLAY'; do
  grep -Fq "$needle" "$UI" || fail "ui_missing:${needle// /_}"
done
python3 - "$MAIN" <<'PY'
from pathlib import Path
import sys
s=Path(sys.argv[1]).read_text()
a=s.index('Some(Ok(Ok(snapshot))) => {')
b=s.index('Some(Ok(Err(error))) => {', a)
block=s[a:b]
if 'self.start_private_dsp();' in block:
    raise SystemExit('startup_success_restarts_private_dsp')
a=s.index('fn activate_local_overlay(&mut self)')
b=s.index('fn activate_local_private_dsp(&mut self)', a)
block=s[a:b]
if block.index('self.dsp = SoftwareDspState::default();') > block.index('self.start_private_dsp();'):
    raise SystemExit('overlay_not_flat_before_private_dsp')
PY

echo 'AETHERFORGE_BEACN_V0_1_23_HARDWARE_AUTHORITY=PASS:HARDWARE_ON_DEVICE+LOCAL_FLAT_OVERLAY'
