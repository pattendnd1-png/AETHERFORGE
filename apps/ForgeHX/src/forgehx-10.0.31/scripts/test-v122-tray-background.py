#!/usr/bin/env python3
from pathlib import Path
import sys
root=Path(__file__).resolve().parents[1]
main=(root/'crates/forgehx-gui/src/main.rs').read_text()
app=(root/'crates/forgehx-gui/src/app.rs').read_text()
gui_cargo=(root/'crates/forgehx-gui/Cargo.toml').read_text()
tray_main=(root/'crates/forgehx-tray/src/main.rs').read_text()
tray_cargo=(root/'crates/forgehx-tray/Cargo.toml').read_text()
errors=[]
def req(c,m):
    if not c: errors.append(m)
req('mod tray;' not in main,'GUI must not own tray module')
req('ksni' not in gui_cargo,'GUI must not own ksni')
req('ksni' in tray_cargo,'standalone tray must own ksni')
req('ViewportCommand::CancelClose' not in app,'GUI X must not cancel close')
req('ViewportCommand::Visible(false)' not in app,'GUI X must not hide/minimize window')
req('forgehx-gui.service' in tray_main,'tray must reopen managed GUI service')
req('Open ForgeHX' in tray_main,'tray must expose Open ForgeHX')
req('Quit ForgeHX tray' in tray_main,'tray must expose explicit tray quit')
req((root/'packaging/systemd/forgehx-tray.service').exists(),'tray service required')
req((root/'packaging/systemd/forgehx-gui.service').exists(),'GUI service required')
if errors:
    print('FAIL: ForgeHX 10.0.6 split tray/background contract')
    for e in errors: print(' -',e)
    sys.exit(1)
print('PASS: ForgeHX 10.0.6 split tray/background contract')
