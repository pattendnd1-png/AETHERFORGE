#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
CONFIG_HOME="${XDG_CONFIG_HOME:-$HOME/.config}"
ICON_THEME=AetherForgeDragonGlass
SRC="$ROOT/payload/share/icons/$ICON_THEME"
DST="$DATA_HOME/icons/$ICON_THEME"
[[ -f "$SRC/index.theme" ]] || { echo "ERROR: icon payload missing: $SRC" >&2; exit 3; }
mkdir -p "$DATA_HOME/icons" "$CONFIG_HOME"
# Remove stale scalable entries so curated fixed-size assets win icon lookup after upgrades.
if [[ -d "$DST/scalable/apps" ]]; then
  for n in aether-terminal aetherforge-browser aetherforge-chat aetherforge-chrome aetherforge-code aetherforge-discord aetherforge-documents aetherforge-files aetherforge-firedragon aetherforge-firefox aetherforge-games aetherforge-generic-app aetherforge-gimp aetherforge-music aetherforge-obs aetherforge-photos aetherforge-settings aetherforge-spotify aetherforge-steam aetherforge-video aetherforge-vlc aetherforge-vscode application-default-icon applications-games applications-graphics applications-multimedia applications-other brave-browser celluloid chromium chromium-browser code codium com.brave.Browser com.discordapp.Discord com.google.Chrome com.heroicgameslauncher.hgl com.obsproject.Studio com.spotify.Client com.valvesoftware.Steam com.visualstudio.code com.vscodium.codium dev.vencord.Vesktop digikam discord dolphin elisa firedragon firedragon-browser firefox gimp google-chrome google-chrome-stable gwenview heroic internet-web-browser kate libreoffice-writer lutris mpv net.lutris.Lutris obs obs-studio org.aetherforge.AetherTerminal org.garuda.FireDragon org.garuda.firedragon org.garudalinux.FireDragon org.garudalinux.firedragon org.gimp.GIMP org.gimp.GIMP-3.0 org.gnome.Loupe org.gnome.Totem org.kde.digikam org.kde.dolphin org.kde.elisa org.kde.gwenview org.kde.kate org.kde.systemsettings org.libreoffice.LibreOffice.writer org.mozilla.firefox org.telegram.desktop org.videolan.VLC preferences-system rhythmbox shotwell signal-desktop slack spotify steam system-file-manager system-settings systemsettings unknown vesktop visual-studio-code vlc web-browser; do
    rm -f "$DST/scalable/apps/$n.svg" "$DST/scalable/apps/$n.svgz" "$DST/scalable/apps/$n.png" "$DST/scalable/apps/$n.webp"
  done
fi
cp -a "$SRC/." "$DST/"
if command -v kwriteconfig6 >/dev/null 2>&1; then
  kwriteconfig6 --file kdeglobals --group Icons --key Theme "$ICON_THEME"
else
  echo "ERROR: kwriteconfig6 is required to select the KDE icon theme" >&2
  exit 4
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$DST" >/dev/null 2>&1 || true
fi
rm -f "$HOME/.cache/icon-cache.kcache" "$HOME/.cache/ksycoca6_"* 2>/dev/null || true
command -v kbuildsycoca6 >/dev/null 2>&1 && kbuildsycoca6 --noincremental >/dev/null 2>&1 || true
# Refresh Plasma so dock/Kickoff cached pixmaps are reloaded.
if systemctl --user list-unit-files plasma-plasmashell.service >/dev/null 2>&1; then
  systemctl --user restart plasma-plasmashell.service || true
elif command -v kquitapp6 >/dev/null 2>&1 && command -v plasmashell >/dev/null 2>&1; then
  kquitapp6 plasmashell >/dev/null 2>&1 || true
  nohup plasmashell --replace >/dev/null 2>&1 &
fi
printf '%s\n' \
  "ICON_THEME=$ICON_THEME" \
  'AETHERFORGE_GENERATED_ICON_PACK=AETHER_TERMINAL_SYSTEM_R13' \
  'THEMED_APP_ICON_MASTERS=22' \
  'AETHERFORGE_THEMED_ICONS=APPLIED'
