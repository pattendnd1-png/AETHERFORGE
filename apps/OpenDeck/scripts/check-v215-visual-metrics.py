#!/usr/bin/env python3
import json, math, sys
from pathlib import Path

root=Path(__file__).resolve().parents[1]
path=Path(sys.argv[1]) if len(sys.argv)>1 else root/'OpenDeck-v2.0.15-VISUAL-METRICS.json'
if not path.exists():
    print(f'OPENDECK_V215_VISUAL_GEOMETRY=FAIL:MISSING:{path}')
    raise SystemExit(2)
data=json.loads(path.read_text())
vp=data.get('viewport',{})
errors=[]
if round(vp.get('width',0)) != 1536 or round(vp.get('height',0)) != 1024:
    errors.append(f'viewport={vp}')
regions=data.get('regions',{})
# Target boxes from the approved render/spec. Tolerance is 2% of reference axis.
targets={
 'header': (0,0,1536,92),
 'sidebar': (0,92,238,932),
 'device': (335,110,735,565),
 'configuration': (245,698,895,302),
 'actions': (1148,92,372,922),
}
tolx=1536*.02; toly=1024*.02
for name,(x,y,w,h) in targets.items():
    box=regions.get(name)
    if not box:
        errors.append(f'missing:{name}'); continue
    for field,actual,expected,tol in [('x',box.get('x',0),x,tolx),('y',box.get('y',0),y,toly),('width',box.get('width',0),w,tolx),('height',box.get('height',0),h,toly)]:
        if abs(actual-expected)>tol:
            errors.append(f'{name}.{field}={actual:.2f} expected={expected:.2f}±{tol:.2f}')

elements=data.get('elements',{})
def check_sizes(name,target_w,target_h,tolerance=.08):
    boxes=elements.get(name,[])
    if not boxes:
        errors.append(f'missing-elements:{name}'); return
    for i,box in enumerate(boxes):
        for field,target in [('width',target_w),('height',target_h)]:
            actual=box.get(field,0)
            if abs(actual-target) > target*tolerance:
                errors.append(f'{name}[{i}].{field}={actual:.2f} target={target:.2f}')
check_sizes('key',124,124)
check_sizes('touch',160,68)
check_sizes('dial',160,104) # interactive cell; ring diameter is separately fixed by CSS contract

if errors:
    print('OPENDECK_V215_VISUAL_GEOMETRY=FAIL')
    for error in errors: print('VISUAL_GEOMETRY_ERROR='+error)
    raise SystemExit(1)
print('OPENDECK_V215_VISUAL_GEOMETRY=PASS')
