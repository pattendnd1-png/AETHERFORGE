#!/usr/bin/env python3
import json, math, sys

if len(sys.argv) != 2:
    raise SystemExit('usage: test-10.0.29-runtime-state.py STATE.json')
doc = json.load(open(sys.argv[1]))
if doc.get('reply') != 'mic_dsp_state':
    raise SystemExit('FORGEHX_10_0_29_RUNTIME_STATE=FAIL:reply_kind')
state = doc.get('state') or {}
cfg = state.get('config') or {}
monitor_cfg = cfg.get('extraneous_noise_monitor') or {}
noise = state.get('noise_scene_telemetry') or {}
classes = noise.get('classes') or {}
bands = noise.get('band_floor_dbfs') or {}

errors=[]
def require(cond, name):
    if not cond: errors.append(name)
def number(v): return isinstance(v,(int,float)) and math.isfinite(float(v))

require(state.get('applied') is True, 'applied')
require(cfg.get('enabled') is True, 'dsp_enabled')
require(monitor_cfg.get('enabled') is True, 'monitor_enabled')
require(monitor_cfg.get('auto_adapt') is True, 'auto_adapt')
raw = state.get('raw_source_node_name') or ''
require(bool(raw) and ('SoloCast' in raw or raw.startswith('alsa_input.')), 'raw_source')
require(state.get('processed_source_node_name') == 'aetherstream.system.microphone', 'processed_source')
for key in ['white_like','pink_like','broadband','hum_rumble','narrowband_whine','transient','speaker_leak']:
    v=classes.get(key)
    require(number(v) and 0.0 <= float(v) <= 1.0, 'class_'+key)
for key in ['sub_rumble_dbfs','low_dbfs','low_mid_dbfs','mid_dbfs','presence_dbfs','high_dbfs','air_dbfs']:
    v=bands.get(key)
    require(number(v) and -90.0 <= float(v) <= -12.0, 'band_'+key)
for key,lo,hi in [('adaptive_suppression_db',0.0,72.0),('sonora_noise_target_percent',0.0,100.0)]:
    v=noise.get(key)
    require(number(v) and lo <= float(v) <= hi, key)
required_unavailable=[p for p in (state.get('unavailable_processors') or []) if p]
require(not required_unavailable, 'unavailable_processors')
if errors:
    raise SystemExit('FORGEHX_10_0_29_RUNTIME_STATE=FAIL:'+','.join(errors))
print('FORGEHX_10_0_29_RUNTIME_STATE=PASS')
