from pathlib import Path
root=Path(__file__).resolve().parents[1]
create=(root/'scripts/create-source-package.sh').read_text()
build=(root/'scripts/build-arch-package.sh').read_text()
checks={
 'source package version':'VERSION="10.0.26"' in create,
 'build package archive':'forgehx-10.0.26.tar.gz' in build,
 'no previous fixed build archive':'forgehx-10.0.25.tar.gz' not in build,
}
failed=[k for k,v in checks.items() if not v]
if failed: raise SystemExit('FAIL: '+', '.join(failed))
print('PASS: ForgeHX 10.0.26 packaging version contract')
