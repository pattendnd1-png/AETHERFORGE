#!/usr/bin/env bash
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$root"

./BUILD-ON-ARCH.sh

bindir="${HOME}/.local/bin"
appdir="${HOME}/.local/share/applications"
mkdir -p "$bindir" "$appdir"
install -m755 target/release/opensanctuary-launcher "$bindir/opensanctuary-launcher"
install -m755 target/release/opensanctuary-engine "$bindir/opensanctuary-engine"

cat > "$appdir/org.opensanctuary.Launcher.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=OpenSanctuary
Comment=Battle.net-integrated Diablo III launcher and native runtime foundation
Exec=${bindir}/opensanctuary-launcher
Terminal=false
Categories=Game;
StartupNotify=true
DESKTOP

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$appdir" >/dev/null 2>&1 || true
fi

printf '\nOpenSanctuary v0.4.2 installed and activated at:\n  %s\n' "$bindir/opensanctuary-launcher"
exec "$bindir/opensanctuary-launcher"
