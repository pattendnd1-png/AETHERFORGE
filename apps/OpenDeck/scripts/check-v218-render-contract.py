from pathlib import Path
import sys
root=Path(__file__).resolve().parents[1]
files={
 'device': root/'apps/opendeck-studio/src/components/DeviceEditor.tsx',
 'dial': root/'apps/opendeck-studio/src/components/DialControl.tsx',
 'action': root/'apps/opendeck-studio/src/components/ActionLibrary.tsx',
 'inspector': root/'apps/opendeck-studio/src/components/PropertyInspector.tsx',
 'styles': root/'apps/opendeck-studio/src/styles.css',
 'app': root/'apps/opendeck-studio/src/app/App.tsx',
}
need={
 'device':['streamdeck-device','data-testid="touch-display"','data-qualify-element="device"'],
 'dial':['dial-ring','dial-cap','data-qualify-element="dial"'],
 'action':['Action Library','VirtualActionList','action-library-title'],
 'inspector':['Configure:','interaction-rail','Assigned Action'],
 'styles':['.streamdeck-device','.streamdeck-touch-display','.dial-ring','.interaction-rail','.action-library-title','--device-width: 735px'],
 'app':['data-layout-region="main"','<AppSidebar'],
}
errors=[]
for key, markers in need.items():
    text=files[key].read_text()
    for marker in markers:
        if marker not in text: errors.append(f'{key}:{marker}')
if errors:
    print('OPENDECK_V218_RENDER_CONTRACT=FAIL')
    for e in errors: print('MISSING='+e)
    sys.exit(1)
print('OPENDECK_V218_RENDER_CONTRACT=PASS')
