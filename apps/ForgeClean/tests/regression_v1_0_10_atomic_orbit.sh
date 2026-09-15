#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

grep -Eq '^version[[:space:]]*=[[:space:]]*"1\.0\.10"' Cargo.toml
grep -Fq 'use crate::orbital::{local_storage_summary, read_status};' src/orbital_ui.rs
grep -Fq 'use crate::orbital_monitor_model::' src/orbital_ui.rs
grep -Fq 'pub mod orbital_ui;' src/lib.rs
grep -Fq 'Self::Orbital => "Orbital Sync"' src/gui.rs
grep -Fq 'Page::Orbital => crate::orbital_ui::show(ui)' src/gui.rs

python3 - . <<'PY'
from pathlib import Path
import sys
root=Path(sys.argv[1])
bad=[]
for p in root.joinpath("src").rglob("*.rs"):
    rel=p.relative_to(root/"src")
    if rel in (Path("main.rs"),Path("gui_main.rs")) or (rel.parts and rel.parts[0]=="bin"):
        continue
    for i,line in enumerate(p.read_text(errors="ignore").splitlines(),1):
        if line.lstrip().startswith("use forgeclean::"):
            bad.append(f"{rel}:{i}:{line}")
if bad:
    print("\n".join(bad))
    raise SystemExit(1)
PY

grep -Fq 'use forgeclean::' src/main.rs
grep -Fq 'use forgeclean::' src/gui_main.rs
grep -Fq 'use forgeclean::' src/bin/forgeclean-system.rs

echo 'FORGECLEAN_V1_0_10_ATOMIC_ORBIT_REGRESSION=PASS'
