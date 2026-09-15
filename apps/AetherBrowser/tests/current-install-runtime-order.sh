#!/usr/bin/env bash
set -euo pipefail
fail(){ echo "AETHER_BROWSER_INSTALL_RUNTIME_ORDER=FAIL:$1"; exit 1; }
VERIFY=scripts/verify.sh
INSTALL=scripts/install-current-tree.sh
HOST=scripts/install-host.sh
PACKAGE=scripts/package-release.sh
grep -qF 'gate AETHER_BROWSER_CLIPPY cargo clippy' "$VERIFY" || fail no-real-clippy-gate
if grep -qF 'current-clippy-structure.sh' "$VERIFY"; then fail handwritten-clippy-still-in-host-verify; fi
if grep -qF 'current-clippy-structure.sh' "$PACKAGE"; then fail handwritten-clippy-still-in-package-suite; fi
grep -qF 'AETHER_BROWSER_RUNTIME_TEST_EXCLUSIONS=TWITCH+YOUTUBE+YOUTUBE_MUSIC' "$VERIFY" || fail no-focused-exclusion-marker
grep -qF 'AETHER_BROWSER_INSTALLED_TWITCH_RUNTIME=SKIPPED:KNOWN_GOOD_OUT_OF_SCOPE' "$INSTALL" || fail twitch-not-skipped
grep -qF 'AETHER_BROWSER_INSTALLED_YOUTUBE_RUNTIME=SKIPPED:KNOWN_GOOD_OUT_OF_SCOPE' "$INSTALL" || fail youtube-not-skipped
grep -qF 'AETHER_BROWSER_INSTALLED_YOUTUBE_MUSIC_RUNTIME=SKIPPED:KNOWN_GOOD_OUT_OF_SCOPE' "$INSTALL" || fail youtube-music-not-skipped
python3 - "$INSTALL" <<'PY'
from pathlib import Path
import sys
s=Path(sys.argv[1]).read_text()
need=['sudo -n pacman -U --noconfirm "$PACKAGE_OUT"','AETHER_BROWSER_INSTALLED_VERSION_IDENTITY=PASS','AETHER_BROWSER_INSTALLED_TWITCH_RUNTIME=SKIPPED:KNOWN_GOOD_OUT_OF_SCOPE']
pos=[]
for token in need:
    i=s.find(token)
    if i < 0: raise SystemExit('AETHER_BROWSER_INSTALL_RUNTIME_ORDER=FAIL:missing:'+token)
    pos.append(i)
if pos != sorted(pos): raise SystemExit('AETHER_BROWSER_INSTALL_RUNTIME_ORDER=FAIL:wrong-post-install-order')
print('AETHER_BROWSER_INSTALL_RUNTIME_ORDER=PASS')
PY
grep -qF 'bash scripts/verify.sh' "$HOST" || fail host-no-preinstall-verify
grep -qF 'bash scripts/install-current-tree.sh' "$HOST" || fail host-no-native-install
echo 'AETHER_BROWSER_INSTALL_RUNTIME_ORDER=PASS'
