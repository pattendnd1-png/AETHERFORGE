#!/usr/bin/env python3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
read=lambda p:(root/p).read_text()
checks={
 'workspace version':'version = "10.0.31"' in read('Cargo.toml'),
 'pkgbuild version':'pkgver=10.0.31' in read('PKGBUILD'),
 'source package version':'VERSION="10.0.31"' in read('scripts/create-source-package.sh'),
 'build package archive':'forgehx-10.0.31.tar.gz' in read('scripts/build-arch-package.sh'),
 'verify identity':(root/'scripts/verify-10.0.31.sh').exists(),
 'source check identity':(root/'scripts/check-v10.0.31-source.py').exists(),
}
failed=[k for k,v in checks.items() if not v]
if failed: raise SystemExit('FORGEHX_10_0_31_PACKAGING_VERSION=FAIL '+','.join(failed))
print('FORGEHX_10_0_31_PACKAGING_VERSION=PASS')
