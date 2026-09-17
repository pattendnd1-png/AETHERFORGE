from pathlib import Path
import sys
root=Path(__file__).resolve().parents[1]
checks={
 'sidebar component': root/'apps/opendeck-studio/src/components/AppSidebar.tsx',
 'glyph component': root/'apps/opendeck-studio/src/components/Glyph.tsx',
}
errors=[]
for name,path in checks.items():
    if not path.exists(): errors.append(name)
app=(root/'apps/opendeck-studio/src/app/App.tsx').read_text()
for marker in ['<AppSidebar', 'data-layout-region="main"', "section, setSection"]:
    if marker not in app: errors.append('app '+marker)
top=(root/'apps/opendeck-studio/src/components/TopBar.tsx').read_text()
for marker in ['OpenDeck+', '2.0.27', 'Connected', 'Settings']:
    if marker not in top: errors.append('topbar '+marker)
css=(root/'apps/opendeck-studio/src/styles.css').read_text()
for marker in ['--sidebar-width: 238px', '--header-height: 92px', '.app-sidebar', '.main-editor']:
    if marker not in css: errors.append('css '+marker)
if errors:
    print('OPENDECK_V227_SHELL_CONTRACT=FAIL')
    for e in errors: print('MISSING='+e)
    sys.exit(1)
print('OPENDECK_V227_SHELL_CONTRACT=PASS')
