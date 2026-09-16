#!/usr/bin/env python3
import json, sys
from pathlib import Path

path=Path(sys.argv[1]) if len(sys.argv)>1 else Path.home()/'Downloads/OpenDeck-v2.0.12-PERFORMANCE-METRICS.json'
if not path.exists():
    print(f'OPENDECK_V212_PERFORMANCE_METRICS=FAIL:MISSING:{path}'); raise SystemExit(2)
data=json.loads(path.read_text())
ui=data.get('interaction',{}); rust=data.get('rust',{}); frame=data.get('framePacing',{})
errors=[]

def require_metric(source,name,p95,p99=None,min_count=100):
    m=source.get(name)
    if not m:
        errors.append(f'{name}:missing'); return
    if int(m.get('count',0)) < min_count: errors.append(f'{name}:count={m.get("count",0)}<{min_count}')
    if float(m.get('p95',1e9)) > p95: errors.append(f'{name}:p95={m.get("p95")}>{p95}')
    if p99 is not None and float(m.get('p99',1e9)) > p99: errors.append(f'{name}:p99={m.get("p99")}>{p99}')

require_metric(ui,'sidebarSelectionMs',35,60)
require_metric(ui,'controlToConfigurationMs',50,80)
require_metric(ui,'keysDialsSwitchMs',35,60)
require_metric(ui,'assignmentMs',60,100)
require_metric(ui,'profileSwitchMs',120,200)
require_metric(ui,'search5000Ms',70,110)
require_metric(ui,'persistenceEnqueueMs',10,None)
require_metric(rust,'persistenceWriteMs',150,None)
require_metric(rust,'hidDecodeDispatchMs',10,16)
require_metric(ui,'hidBridgeToVisibleMs',25,40)

# A conservative end-to-end estimate: decoded-report dispatch plus browser-to-visible frame completion.
hid_r=rust.get('hidDecodeDispatchMs',{}); hid_u=ui.get('hidBridgeToVisibleMs',{})
if hid_r and hid_u:
    end95=float(hid_r.get('p95',1e9))+float(hid_u.get('p95',1e9))
    end99=float(hid_r.get('p99',1e9))+float(hid_u.get('p99',1e9))
    print(f'OPENDECK_V212_HID_DECODE_VISIBLE_ESTIMATE_P95_MS={end95:.3f}')
    print(f'OPENDECK_V212_HID_DECODE_VISIBLE_ESTIMATE_P99_MS={end99:.3f}')
    if end95>25: errors.append(f'hidDecodeVisible:p95={end95:.3f}>25')
    if end99>40: errors.append(f'hidDecodeVisible:p99={end99:.3f}>40')

total=int(frame.get('total',0)); within16=int(frame.get('within16_7',0)); within33=int(frame.get('within33_3',0)); over50=int(frame.get('over50',0)); maxms=float(frame.get('maxMs',1e9))
if total < 500: errors.append(f'framePacing:total={total}<500')
if total:
    r16=within16/total; r33=within33/total
    print(f'OPENDECK_V212_FRAME_WITHIN_16_7_RATIO={r16:.6f}')
    print(f'OPENDECK_V212_FRAME_WITHIN_33_3_RATIO={r33:.6f}')
    if r16 < .95: errors.append(f'frame16.7={r16:.4f}<.95')
    if r33 < .99: errors.append(f'frame33.3={r33:.4f}<.99')
if over50 != 0 or maxms > 50: errors.append(f'frame>50 count={over50} max={maxms:.3f}')


startup=data.get('startup',{})
cold=float(startup.get('coldMs',1e9)); warm95=float(startup.get('warmP95Ms',1e9)); warm_count=int(startup.get('warmCount',0))
print(f'OPENDECK_V212_COLD_START_MS={cold:.3f}')
print(f'OPENDECK_V212_WARM_START_P95_MS={warm95:.3f}')
if cold>1200: errors.append(f'coldStart={cold:.3f}>1200')
if warm_count<5: errors.append(f'warmStart:count={warm_count}<5')
if warm95>650: errors.append(f'warmStart:p95={warm95:.3f}>650')

idle=data.get('idle',{})
idle_cpu=float(idle.get('cpuAveragePercent',1e9)); rss=float(idle.get('rssMaxMiB',1e9))
print(f'OPENDECK_V212_IDLE_CPU_PERCENT={idle_cpu:.3f}')
print(f'OPENDECK_V212_RSS_MAX_MIB={rss:.3f}')
if idle_cpu>1.5: errors.append(f'idleCpu={idle_cpu:.3f}>1.5')
if rss>300: print('OPENDECK_V212_MEMORY_TARGET=REQUIRES_USER_APPROVAL')
else: print('OPENDECK_V212_MEMORY_TARGET=PASS')

if errors:
    print('OPENDECK_V212_PERFORMANCE_METRICS=FAIL')
    for error in errors: print('PERFORMANCE_ERROR='+error)
    raise SystemExit(1)
print('OPENDECK_V212_PERFORMANCE_METRICS=PASS')
