#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
core=(root/'crates/forgehx-core/src/lib.rs').read_text()
daemon=(root/'crates/forgehx-daemon/src/lib.rs').read_text()
runtime=(root/'crates/forgehx-dsp/src/runtime.rs').read_text()
dsp=(root/'crates/forgehx-dsp/src/lib.rs').read_text()
mic=(root/'crates/forgehx-gui/src/microphone.rs').read_text()
app=(root/'crates/forgehx-gui/src/app.rs').read_text()
device=(root/'crates/forgehx-gui/src/device_page.rs').read_text()
checks={
 'ipc_v11':'pub const IPC_PROTOCOL_VERSION: u32 = 11;' in core,
 'monitor_state':'pub struct MicrophoneMonitorState' in core,
 'monitor_command':'MicMonitorSet {' in core,
 'wired_selector':'fn select_wired_monitor_sink' in daemon,
 'reject_bluez':'bluez' in daemon.lower() and 'bluetooth' in daemon.lower(),
 'runtime_monitor':'MonitorRuntimeConfig' in runtime and 'forgehx-mic-monitor' in runtime,
 'processed_source_monitor':'processed_source' in runtime and 'monitor_target' in runtime,
 'dsp_set_monitor':'pub fn set_monitor' in dsp,
 'gui_toggle':'Live Headphone Monitor' in mic,
 'gui_no_bluetooth':'Bluetooth is never used for mic monitoring' in mic,
 'gui_action':'MicMonitorSet' in app and 'MicMonitor' in device,
}
failed=[name for name,ok in checks.items() if not ok]
if failed:
    raise SystemExit('FORGEHX_10_0_5_WIRED_MIC_MONITOR=FAIL ' + ','.join(failed))
print('FORGEHX_10_0_5_WIRED_MIC_MONITOR=PASS')
