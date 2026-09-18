#!/usr/bin/env python3
import json, sys
from pathlib import Path

root = Path(__file__).resolve().parents[1]
path = Path(sys.argv[1]) if len(sys.argv) > 1 else root / 'OpenDeck-v2.0.55-VISUAL-METRICS.json'
if not path.exists():
    print(f'OPENDECK_V255_VISUAL_GEOMETRY=FAIL:MISSING:{path}')
    raise SystemExit(2)
data = json.loads(path.read_text())
vp = data.get('viewport', {})
errors = []
if round(vp.get('width', 0)) != 1536 or round(vp.get('height', 0)) != 1024:
    errors.append(f'viewport={vp}')
regions = data.get('regions', {})
tolx = 1536 * .02
toly = 1024 * .02
# Layout regions are containers. The previous v2.0.29 gate incorrectly compared
# the device-stage container against the physical Stream Deck card itself.
region_targets = {
    'header': (0, 0, 1536, 76),
    'sidebar': (6, 82, 182, 932),
    'actions': (1216, 82, 310, 932),
}
for name, (x, y, w, h) in region_targets.items():
    box = regions.get(name)
    if not box:
        errors.append(f'missing:{name}')
        continue
    for field, actual, expected, tol in [
        ('x', box.get('x', 0), x, tolx), ('y', box.get('y', 0), y, toly),
        ('width', box.get('width', 0), w, tolx), ('height', box.get('height', 0), h, toly),
    ]:
        if abs(actual - expected) > tol:
            errors.append(f'{name}.{field}={actual:.2f} expected={expected:.2f}±{tol:.2f}')
configuration = regions.get('configuration')
if not configuration:
    errors.append('missing:configuration')
else:
    if configuration.get('height', 0) < 240:
        errors.append(f'configuration.height={configuration.get("height", 0):.2f} minimum=240')
    if configuration.get('width', 0) < 820:
        errors.append(f'configuration.width={configuration.get("width", 0):.2f} minimum=820')

stage = regions.get('device')
if not stage:
    errors.append('missing:device')
else:
    for field, expected, tol in [('x', 188, tolx), ('width', 1028, tolx)]:
        actual = stage.get(field, 0)
        if abs(actual - expected) > tol:
            errors.append(f'device-stage.{field}={actual:.2f} expected={expected:.2f}±{tol:.2f}')
    if stage.get('height', 0) < 620:
        errors.append(f'device-stage.height={stage.get("height", 0):.2f} minimum=620')

elements = data.get('elements', {})
def check_sizes(name, target_w, target_h, tolerance=.08, count=None):
    boxes = elements.get(name, [])
    if not boxes:
        errors.append(f'missing-elements:{name}')
        return
    if count is not None and len(boxes) != count:
        errors.append(f'{name}.count={len(boxes)} expected={count}')
    for i, box in enumerate(boxes):
        for field, target in [('width', target_w), ('height', target_h)]:
            actual = box.get(field, 0)
            if abs(actual - target) > target * tolerance:
                errors.append(f'{name}[{i}].{field}={actual:.2f} target={target:.2f}')
check_sizes('device', 621, 508.5, .06, 1)
check_sizes('key', 108, 100.8, .08, 8)
check_sizes('touch', 141.75, 63, .08, 4)
check_sizes('dial', 147.15, 106.2, .08, 4)

if errors:
    print('OPENDECK_V255_VISUAL_GEOMETRY=FAIL')
    for error in errors:
        print('VISUAL_GEOMETRY_ERROR=' + error)
    raise SystemExit(1)
print('OPENDECK_V255_VISUAL_GEOMETRY=PASS')
