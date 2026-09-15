#!/usr/bin/env python3
import json, pathlib, sys, zipfile, collections, math
ROOT=pathlib.Path(sys.argv[1]).resolve(); errors=[]
def err(x): errors.append(x)
if (ROOT/'VERSION').read_text().strip()!='3.0.5': err('VERSION is not 3.0.5')
for res,w,h in [('1920x1080',1920,1080),('2560x1440',2560,1440)]:
 p=ROOT/'obs'/f'AetherForge-v3.0.5-{res}.json'
 try: d=json.loads(p.read_text())
 except Exception as e: err(f'{p.name}: invalid json {e}'); continue
 if d.get('name')!=f'AetherForge v3.0.5 {res}': err(f'{p.name}: wrong name')
 if d.get('resolution')!={'x':w,'y':h}: err(f'{p.name}: wrong resolution')
 expected_rel_x=-(w/h)
 names=[s.get('name','') for s in d.get('sources',[])]
 if any(n.startswith('AF Placeholder') for n in names): err(f'{p.name}: placeholder source still present')
 uuids=[s.get('uuid') for s in d.get('sources',[]) if s.get('uuid')]
 for u,c in collections.Counter(uuids).items():
  if c!=1: err(f'{p.name}: duplicate uuid {u}')
 byuuid={s.get('uuid') for s in d.get('sources',[]) if s.get('uuid')}
 for s in d.get('sources',[]):
  if s.get('id')=='ffmpeg_source':
   lf=s.get('settings',{}).get('local_file','')
   if not lf.startswith('__MEDIA_ROOT__/media/'): err(f'{p.name}: nonportable media template {lf}')
  if s.get('id')=='scene':
   for i in s.get('settings',{}).get('items',[]):
    if i.get('source_uuid') not in byuuid: err(f'{p.name}: dangling item {i.get("name")}')
    if str(i.get('name','')).startswith('AF Placeholder'): err(f'{p.name}: placeholder item still present')
    if str(i.get('name','')).startswith('AF '):
     if i.get('pos')!={'x':0.0,'y':0.0} or i.get('align')!=5: err(f'{p.name}: package background not top-left anchored')
     pr=i.get('pos_rel',{})
     if not (math.isclose(pr.get('x',999), expected_rel_x, abs_tol=1e-6) and math.isclose(pr.get('y',999), -1.0, abs_tol=1e-6)): err(f'{p.name}: bad OBS relative coordinates for {i.get("name")}')
     if i.get('scale_ref')!={'x':float(w),'y':float(h)}: err(f'{p.name}: bad scale_ref for {i.get("name")}')
for fn in ['install.sh','launch-1080p.sh','launch-1440p.sh','diagnose.sh']:
 if not (ROOT/'scripts'/fn).exists(): err(f'missing scripts/{fn}')
for fn in ['install.sh','diagnose.sh','launch-1080p.sh','launch-1440p.sh','setup.sh']:
 if not (ROOT/fn).exists(): err(f'missing root wrapper {fn}')
prof=ROOT/'streamdeck'/'AetherForge-v3.0.5.streamDeckProfile'
if not prof.exists(): err('missing Stream Deck profile')
else:
 try:
  with zipfile.ZipFile(prof) as z:
   roots=sorted({n.split('/')[0] for n in z.namelist() if '/' in n})
   if len(roots)!=1: err('Stream Deck archive root invalid')
   elif f'{roots[0]}/manifest.json' not in z.namelist(): err('Stream Deck root manifest missing')
 except Exception as e: err(f'Stream Deck archive invalid: {e}')
if errors:
 print('FAIL'); [print(' -',x) for x in errors]; raise SystemExit(1)
print('PASS: v3.0.5 package structure validates, including OBS relative centering coordinates')
