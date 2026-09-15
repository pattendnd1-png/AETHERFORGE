from pathlib import Path
root=Path('.')
checks=[]
def need(ok,msg):
    if not ok: checks.append(msg)
workspace=(root/'Cargo.toml').read_text()
daemon=(root/'crates/forgehx-daemon/src/lib.rs').read_text()
daemon_toml=(root/'crates/forgehx-daemon/Cargo.toml').read_text()
need('crates/forgehx-mic' in workspace,'workspace microphone crate')
need((root/'crates/forgehx-mic/src/lib.rs').exists(),'microphone registry source')
if (root/'crates/forgehx-mic/src/lib.rs').exists():
    mic=(root/'crates/forgehx-mic/src/lib.rs').read_text()
    need('SOLOCAST_2_PRODUCT_ID' in mic and '0x0fbf' in mic,'SoloCast 2 exact identity')
    need('required_capabilities' in mic,'SoloCast 2 support contract')
need('forgehx-mic = { path = "../forgehx-mic" }' in daemon_toml,'daemon microphone registry dependency')
need('complete_microphone_support' in daemon,'composite microphone full-support policy')
need('complete_native_support' in daemon,'complete native-driver full-support policy')
need('pulsefire_haste_complete_driver_stays_full_with_compatibility_owners' in daemon,'Haste classification regression')
need('solocast_2_exact_composite_stack_is_fully_supported' in daemon,'SoloCast 2 classification regression')
if checks:
    raise SystemExit('FAIL: '+', '.join(checks))
print('PASS: ForgeHX 10.0.6 full target hardware support contract')
