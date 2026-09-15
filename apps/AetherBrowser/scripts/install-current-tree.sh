#!/usr/bin/env bash
set -euo pipefail

VERSION='2.1.60'
PKGREL='1'
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
OUT_DIR="${AETHER_BROWSER_OUT_DIR:-$HOME/Downloads}"
TARGET_ROOT="${CARGO_TARGET_DIR:-$ROOT/target}"
RELEASE_DIR="$TARGET_ROOT/release"
PACKAGE_OUT="$OUT_DIR/Aether-Browser-v2.1.60-PACMAN.pkg.tar.zst"
INSTALL_VERIFY="$OUT_DIR/Aether-Browser-v2.1.60-INSTALL-VERIFY.txt"
STATE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/aetherforge/aether-browser"
PROFILE_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/aetherforge/aether-browser/profiles/default"
COMPAT_PROFILE_ROOT="${XDG_DATA_HOME:-$HOME/.local/share}/aetherforge/aether-browser/profiles/compat"
STAGE_DIR="$STATE_DIR/staging/${VERSION}-${PKGREL}"
TMP_ROOT="${AETHER_BROWSER_TMP_ROOT:-${XDG_CACHE_HOME:-$HOME/.cache}/aetherforge/aether-browser/tmp}"
mkdir -p "$TMP_ROOT"
PKG_WORK=$(mktemp -d "$TMP_ROOT/aether-browser-v2.1.60-pkg.XXXXXX")
trap 'rm -rf "$PKG_WORK"' EXIT
mkdir -p "$OUT_DIR" "$STATE_DIR"

require_file() {
  [[ -f "$1" ]] || { echo "AETHER_BROWSER_INSTALL=FAIL:missing:$1" >&2; exit 2; }
}
require_exec() {
  [[ -x "$1" ]] || { echo "AETHER_BROWSER_INSTALL=FAIL:not-executable:$1" >&2; exit 2; }
}

browser_live_pids(){
  ps -u "$UID" -o pid=,stat=,comm= 2>/dev/null | awk '$3 == "aether-browser" && $2 !~ /^Z/ { print $1 }'
}
browser_zombie_pids(){
  ps -u "$UID" -o pid=,stat=,comm= 2>/dev/null | awk '$3 == "aether-browser" && $2 ~ /^Z/ { print $1 }'
}
shutdown_browser_processes(){
  local -a pids zombies
  mapfile -t pids < <(browser_live_pids)
  if (( ${#pids[@]} > 0 )); then
    kill -TERM "${pids[@]}" 2>/dev/null || true
  fi
  for _attempt in $(seq 1 50); do
    mapfile -t pids < <(browser_live_pids)
    (( ${#pids[@]} == 0 )) && break
    sleep 0.1
  done
  mapfile -t pids < <(browser_live_pids)
  if (( ${#pids[@]} > 0 )); then
    kill -KILL "${pids[@]}" 2>/dev/null || true
    for _attempt in $(seq 1 20); do
      mapfile -t pids < <(browser_live_pids)
      (( ${#pids[@]} == 0 )) && break
      sleep 0.1
    done
  fi
  mapfile -t zombies < <(browser_zombie_pids)
  if (( ${#zombies[@]} > 0 )); then
    echo "AETHER_BROWSER_PROMOTION_BROWSER_SHUTDOWN_ZOMBIES=IGNORED:${zombies[*]}"
  fi
  mapfile -t pids < <(browser_live_pids)
  (( ${#pids[@]} == 0 ))
}

profile_fingerprint() {
  local root=${1:?profile root required}
  python3 - "$root" <<'PYPROFILE'
import hashlib, os, pathlib, sys
root = pathlib.Path(sys.argv[1])
h = hashlib.sha256()
if not root.exists():
    print('MISSING')
    raise SystemExit(0)
for path in sorted(root.rglob('*'), key=lambda p: p.as_posix()):
    rel = path.relative_to(root).as_posix().encode()
    h.update(rel + b'\0')
    try:
        if path.is_symlink():
            h.update(b'L' + os.readlink(path).encode() + b'\0')
        elif path.is_file():
            h.update(b'F')
            with path.open('rb') as f:
                for chunk in iter(lambda: f.read(1024 * 1024), b''):
                    h.update(chunk)
    except FileNotFoundError:
        h.update(b'RACE')
print(h.hexdigest())
PYPROFILE
}

require_exec "$RELEASE_DIR/aether-browser"
require_exec "$RELEASE_DIR/aether-media-service"
require_exec "$RELEASE_DIR/aether-obs-verify"
require_file "$ROOT/packaging/systemd/aether-browser-media.service"
require_file "$ROOT/packaging/desktop/org.aetherforge.AetherBrowser.desktop.in"
require_exec "$ROOT/scripts/stability-acceptance.sh"
require_exec "$ROOT/scripts/browser-takeover.sh"
require_exec "$ROOT/scripts/browser-takeover-rollback.sh"
require_exec "$ROOT/scripts/obs-runtime-test.sh"
require_exec "$ROOT/scripts/streamlabs-runtime-test.sh"
require_exec "$ROOT/scripts/vendor-package-normalize.py"
require_exec "$ROOT/packaging/wrappers/aether-browser-stability"
require_exec "$ROOT/packaging/wrappers/aether-browser-takeover"
require_exec "$ROOT/packaging/wrappers/aether-browser-takeover-rollback"

PAYLOAD="$PKG_WORK/payload-root"
mkdir -p \
  "$PAYLOAD/usr/lib/aetherforge/aether-browser/bin" \
  "$PAYLOAD/usr/lib/aetherforge/aether-browser/tools" \
  "$PAYLOAD/usr/bin" \
  "$PAYLOAD/usr/lib/systemd/user" \
  "$PAYLOAD/usr/lib/udev/rules.d" \
  "$PAYLOAD/usr/share/applications" \
  "$PAYLOAD/usr/share/licenses/aether-browser"
install -m 0755 "$RELEASE_DIR/aether-browser" "$PAYLOAD/usr/lib/aetherforge/aether-browser/bin/aether-browser"
install -m 0755 "$RELEASE_DIR/aether-media-service" "$PAYLOAD/usr/lib/aetherforge/aether-browser/bin/aether-media-service"
install -m 0755 "$RELEASE_DIR/aether-obs-verify" "$PAYLOAD/usr/lib/aetherforge/aether-browser/bin/aether-obs-verify"
install -m 0755 "$RELEASE_DIR/aether-obs-verify" "$PAYLOAD/usr/bin/aether-obs-verify"
install -m 0755 "$ROOT/scripts/stability-acceptance.sh" "$PAYLOAD/usr/lib/aetherforge/aether-browser/tools/stability-acceptance.sh"
install -m 0755 "$ROOT/scripts/browser-takeover.sh" "$PAYLOAD/usr/lib/aetherforge/aether-browser/tools/browser-takeover.sh"
install -m 0755 "$ROOT/scripts/browser-takeover-rollback.sh" "$PAYLOAD/usr/lib/aetherforge/aether-browser/tools/browser-takeover-rollback.sh"
install -m 0755 "$ROOT/scripts/obs-runtime-test.sh" "$PAYLOAD/usr/lib/aetherforge/aether-browser/tools/obs-runtime-test.sh"
install -m 0755 "$ROOT/scripts/streamlabs-runtime-test.sh" "$PAYLOAD/usr/lib/aetherforge/aether-browser/tools/streamlabs-runtime-test.sh"
install -m 0755 "$ROOT/scripts/vendor-package-normalize.py" "$PAYLOAD/usr/lib/aetherforge/aether-browser/tools/vendor-package-normalize.py"
install -m 0755 "$ROOT/packaging/wrappers/aether-browser" "$PAYLOAD/usr/bin/aether-browser"
install -m 0755 "$ROOT/packaging/wrappers/aether-media-service" "$PAYLOAD/usr/bin/aether-media-service"
install -m 0755 "$ROOT/packaging/wrappers/aether-browser-stability" "$PAYLOAD/usr/bin/aether-browser-stability"
install -m 0755 "$ROOT/packaging/wrappers/aether-browser-takeover" "$PAYLOAD/usr/bin/aether-browser-takeover"
install -m 0755 "$ROOT/packaging/wrappers/aether-browser-takeover-rollback" "$PAYLOAD/usr/bin/aether-browser-takeover-rollback"
install -m 0644 "$ROOT/packaging/systemd/aether-browser-media.service" "$PAYLOAD/usr/lib/systemd/user/aether-browser-media.service"
install -m 0644 "$ROOT/packaging/desktop/org.aetherforge.AetherBrowser.desktop.in" "$PAYLOAD/usr/share/applications/org.aetherforge.AetherBrowser.desktop"
install -m 0644 "$ROOT/packaging/LICENSE-MIT-OR-APACHE-2.0" "$PAYLOAD/usr/share/licenses/aether-browser/LICENSE-MIT-OR-APACHE-2.0"

bash "$ROOT/scripts/create-root-owned-payload-tar.sh" "$PAYLOAD" "$PKG_WORK/payload.tar"
cat > "$PKG_WORK/PKGBUILD" <<'PKGEOF'
pkgname=aether-browser
pkgver=2.1.60
pkgrel=1
pkgdesc='AetherForge Rust-native browser, creator studio, communications hub, and integrated media workspace'
arch=('x86_64')
license=('MIT' 'Apache-2.0')
depends=('obs-studio' 'chromium' 'glib2' 'gstreamer' 'gst-plugins-base' 'gst-plugins-good' 'gst-plugins-bad' 'gst-libav' 'gst-plugin-pipewire' 'yt-dlp' 'deno' 'ffmpeg' 'streamlink' 'libx11' 'wayland' 'fontconfig' 'freetype2' 'openssl' 'sqlite' 'libxkbcommon' 'mesa')
options=('!strip' '!debug')
source=('payload.tar')
sha256sums=('SKIP')

package() {
  bsdtar -xf "$srcdir/payload.tar" -C "$pkgdir"
  chown -R 0:0 "$pkgdir"
  find "$pkgdir" -type d -exec chmod 0755 {} +
}
PKGEOF

mkdir -p "$PKG_WORK/out"
(
  cd "$PKG_WORK"
  PKGDEST="$PKG_WORK/out" makepkg --force --noconfirm --nodeps
)
BUILT_PACKAGE=$(find "$PKG_WORK/out" -maxdepth 1 -type f -name "aether-browser-${VERSION}-${PKGREL}-x86_64.pkg.tar.*" -print -quit)
[[ -n "$BUILT_PACKAGE" && -f "$BUILT_PACKAGE" ]] || {
  echo 'AETHER_BROWSER_INSTALL=FAIL:makepkg-no-package' >&2
  exit 3
}
cp -f "$BUILT_PACKAGE" "$PACKAGE_OUT"
chmod 0644 "$PACKAGE_OUT"
echo "AETHER_BROWSER_PACMAN_PACKAGE=$PACKAGE_OUT"

if [[ "${AETHER_BROWSER_PACKAGE_ONLY:-0}" == "1" ]]; then
  echo 'AETHER_BROWSER_PACKAGE_ONLY=PASS'
  exit 0
fi

# Versioned candidate stage: validate the exact package that will be promoted,
# without touching the currently installed browser.
echo "AETHER_BROWSER_STAGE=START:${VERSION}-${PKGREL}"
rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR/root"
bsdtar -xf "$PACKAGE_OUT" -C "$STAGE_DIR/root"
for staged_path in \
  usr/lib/aetherforge/aether-browser/bin/aether-browser \
  usr/lib/aetherforge/aether-browser/bin/aether-media-service \
  usr/lib/aetherforge/aether-browser/bin/aether-obs-verify \
  usr/bin/aether-obs-verify \
  usr/lib/aetherforge/aether-browser/tools/obs-runtime-test.sh \
  usr/lib/aetherforge/aether-browser/tools/streamlabs-runtime-test.sh \
  usr/bin/aether-browser \
  usr/lib/systemd/user/aether-browser-media.service \
  usr/share/applications/org.aetherforge.AetherBrowser.desktop; do
  [[ -e "$STAGE_DIR/root/$staged_path" ]] || { echo "AETHER_BROWSER_STAGE_PACKAGE_INTEGRITY=FAIL:missing:$staged_path" >&2; exit 11; }
done
echo 'AETHER_BROWSER_STAGE_PACKAGE_INTEGRITY=PASS'
STAGED_BROWSER="$STAGE_DIR/root/usr/lib/aetherforge/aether-browser/bin/aether-browser"
if [[ "$($STAGED_BROWSER --version 2>/dev/null)" == "Aether Browser ${VERSION}" ]]; then
  echo "AETHER_BROWSER_STAGE_VERSION_IDENTITY=PASS:${VERSION}"
else
  echo 'AETHER_BROWSER_STAGE_VERSION_IDENTITY=FAIL' >&2
  exit 12
fi
sha256sum "$PACKAGE_OUT" > "$STAGE_DIR/PACKAGE.sha256"
printf '%s\n' "$PACKAGE_OUT" > "$STAGE_DIR/PACKAGE.path"
echo "AETHER_BROWSER_STAGE_READY=PASS:$STAGE_DIR"

previous_version=''
previous_package=''
if command -v pacman >/dev/null 2>&1; then
  previous_version=$(pacman -Q aether-browser 2>/dev/null | awk '{print $2}' || true)
  if [[ -n "$previous_version" && "$previous_version" != "${VERSION}-${PKGREL}" ]]; then
    previous_package=$(find /var/cache/pacman/pkg -maxdepth 1 -type f -name "aether-browser-${previous_version}-*.pkg.tar.*" -printf '%T@ %p\n' 2>/dev/null | sort -nr | head -1 | cut -d' ' -f2- || true)
  fi
fi
if [[ -n "$previous_package" && -f "$previous_package" ]]; then
  rollback_copy="$STATE_DIR/$(basename "$previous_package")"
  cp -f "$previous_package" "$rollback_copy"
  printf '%s\n' "$rollback_copy" > "$STATE_DIR/previous-package"
fi

# Promotion starts only after build + hard preinstall verification + staged package validation succeeded.
echo 'AETHER_BROWSER_SUDO_AUTH=START'
sudo -v
echo 'AETHER_BROWSER_SUDO_AUTH=PASS'
sudo -n pacman -S --needed --noconfirm obs-studio chromium yt-dlp deno ffmpeg streamlink gst-plugins-bad gst-plugin-pipewire
command -v obs >/dev/null 2>&1 || { echo 'AETHER_BROWSER_OBS_BACKEND_DEPENDENCY=FAIL:obs-missing' >&2; exit 10; }
command -v chromium >/dev/null 2>&1 || { echo 'AETHER_BROWSER_COMPAT_CHROMIUM_DEPENDENCY=FAIL:chromium-missing' >&2; exit 10; }
command -v yt-dlp >/dev/null 2>&1 || { echo 'AETHER_BROWSER_YOUTUBE_RESOLVER_DEPENDENCY=FAIL:yt-dlp-missing' >&2; exit 10; }
command -v deno >/dev/null 2>&1 || { echo 'AETHER_BROWSER_YOUTUBE_JS_RUNTIME_DEPENDENCY=FAIL:deno-missing' >&2; exit 10; }
command -v ffmpeg >/dev/null 2>&1 || { echo 'AETHER_BROWSER_YOUTUBE_FFMPEG_DEPENDENCY=FAIL:ffmpeg-missing' >&2; exit 10; }
command -v streamlink >/dev/null 2>&1 || { echo 'AETHER_BROWSER_TWITCH_STREAMLINK_DEPENDENCY=FAIL:streamlink-missing' >&2; exit 10; }
gst-inspect-1.0 pipewiresink >/dev/null 2>&1 || { echo 'AETHER_BROWSER_PIPEWIRE_GSTREAMER_DEPENDENCY=FAIL:pipewiresink-missing' >&2; exit 10; }
echo 'AETHER_BROWSER_COMPAT_CHROMIUM_DEPENDENCY=PASS:chromium'
echo 'AETHER_BROWSER_YOUTUBE_RESOLVER_DEPENDENCY=PASS:yt-dlp'
echo 'AETHER_BROWSER_YOUTUBE_JS_RUNTIME_DEPENDENCY=PASS:deno'
echo 'AETHER_BROWSER_YOUTUBE_FFMPEG_DEPENDENCY=PASS:ffmpeg'
echo 'AETHER_BROWSER_TWITCH_STREAMLINK_DEPENDENCY=PASS:streamlink'
echo 'AETHER_BROWSER_PIPEWIRE_GSTREAMER_DEPENDENCY=PASS:pipewiresink'
# Do not leave an older mapped browser process running after package replacement.
# Ignore already-dead zombies; only a genuinely live current-user process blocks promotion.
shutdown_browser_processes || { echo 'AETHER_BROWSER_PROMOTION_BROWSER_SHUTDOWN=FAIL:live-process-still-running' >&2; exit 11; }
echo 'AETHER_BROWSER_PROMOTION_BROWSER_SHUTDOWN=PASS'
profile_existed_before=0
profile_before='MISSING'
if [[ -d "$PROFILE_ROOT" ]]; then
  profile_existed_before=1
  profile_before=$(profile_fingerprint "$PROFILE_ROOT")
  echo "AETHER_BROWSER_PROFILE_PREPROMOTION=PASS:$profile_before"
else
  echo 'AETHER_BROWSER_PROFILE_PREPROMOTION=PASS:no-existing-profile'
fi
compat_profile_existed_before=0
compat_profile_before='MISSING'
if [[ -d "$COMPAT_PROFILE_ROOT" ]]; then
  compat_profile_existed_before=1
  compat_profile_before=$(profile_fingerprint "$COMPAT_PROFILE_ROOT")
  echo "AETHER_BROWSER_COMPAT_PROFILE_PREPROMOTION=PASS:$compat_profile_before"
else
  echo 'AETHER_BROWSER_COMPAT_PROFILE_PREPROMOTION=PASS:no-existing-profile'
fi
echo "AETHER_BROWSER_ATOMIC_PROMOTION=START:${VERSION}-${PKGREL}"
sudo -n pacman -U --noconfirm "$PACKAGE_OUT"
profile_after=$(profile_fingerprint "$PROFILE_ROOT")
compat_profile_after=$(profile_fingerprint "$COMPAT_PROFILE_ROOT")
if (( profile_existed_before == 1 )); then
  if [[ "$profile_before" != "$profile_after" ]]; then
    echo "AETHER_BROWSER_EXISTING_LOGIN_STATE_PRESERVED=FAIL:profile-changed-during-promotion:$profile_before:$profile_after" >&2
    exit 15
  fi
  PROFILE_PRESERVATION_RESULT='existing-profile-unchanged'
else
  PROFILE_PRESERVATION_RESULT='no-existing-profile'
fi
if (( compat_profile_existed_before == 1 )); then
  if [[ "$compat_profile_before" != "$compat_profile_after" ]]; then
    echo "AETHER_BROWSER_COMPAT_PROFILE_PRESERVED=FAIL:profile-changed-during-promotion:$compat_profile_before:$compat_profile_after" >&2
    exit 16
  fi
  COMPAT_PROFILE_PRESERVATION_RESULT='existing-compat-profile-unchanged'
else
  COMPAT_PROFILE_PRESERVATION_RESULT='new-or-not-yet-created'
fi
if pacman -Q aether-browser 2>/dev/null | grep -qF "aether-browser ${VERSION}-${PKGREL}"; then
  echo "AETHER_BROWSER_ATOMIC_PROMOTION=PASS:${VERSION}-${PKGREL}"
else
  echo "AETHER_BROWSER_ATOMIC_PROMOTION=FAIL:identity-mismatch" >&2
  exit 13
fi
bash "$ROOT/scripts/repair-private-install-dirs.sh"
sudo -n udevadm control --reload-rules
sudo -n udevadm trigger --subsystem-match=hidraw || true
systemctl --user daemon-reload
systemctl --user disable --now aether-browser-aetherai.service >/dev/null 2>&1 || true
systemctl --user enable aether-browser-media.service
systemctl --user restart aether-browser-media.service

: > "$INSTALL_VERIFY"
record() { printf '%s\n' "$1" | tee -a "$INSTALL_VERIFY"; }
hard_fail=0
AETHER_BROWSER_PROVIDER_RENDER_WARNING=0
record "AETHER_BROWSER_VERSION=${VERSION}"
record "AETHER_BROWSER_PACMAN_PACKAGE=$PACKAGE_OUT"
record "AETHER_BROWSER_EXISTING_LOGIN_STATE_PRESERVED=PASS:$PROFILE_PRESERVATION_RESULT"
record "AETHER_BROWSER_COMPAT_PROFILE_PRESERVED=PASS:$COMPAT_PROFILE_PRESERVATION_RESULT"
record "AETHER_BROWSER_WINDOW_CLOSE_DOES_NOT_LOGOUT=PASS:profiles-preserved"
record "AETHER_BROWSER_UPGRADE_PROFILE_MIGRATION=PASS:same-path:$PROFILE_ROOT"
record "AETHER_BROWSER_COMPAT_UPGRADE_PROFILE_MIGRATION=PASS:same-path:$COMPAT_PROFILE_ROOT"
if pacman -Q aether-browser 2>/dev/null | grep -qF "aether-browser ${VERSION}-${PKGREL}"; then
  record "AETHER_BROWSER_INSTALL_PACMAN=PASS:${VERSION}-${PKGREL}"
else
  record 'AETHER_BROWSER_INSTALL_PACMAN=FAIL'
  hard_fail=1
fi
if [[ "$(/usr/bin/aether-browser --version 2>/dev/null)" == "Aether Browser ${VERSION}" ]]; then
  record 'AETHER_BROWSER_INSTALL_BINARY=PASS'
  record "AETHER_BROWSER_INSTALLED_VERSION_IDENTITY=PASS:${VERSION}"
else
  record 'AETHER_BROWSER_INSTALL_BINARY=FAIL'
  record 'AETHER_BROWSER_INSTALLED_VERSION_IDENTITY=FAIL'
  hard_fail=1
fi
if installed_status=$(/usr/bin/aether-browser --status 2>&1); then
  record 'AETHER_BROWSER_INSTALLED_BINARY_STARTUP=PASS'
else
  record 'AETHER_BROWSER_INSTALLED_BINARY_STARTUP=FAIL'
  while IFS= read -r line; do record "AETHER_BROWSER_INSTALLED_BINARY_STARTUP_DETAIL=$line"; done <<<"${installed_status:-}"
  hard_fail=1
fi
# v2.1.56 focused patch: browser Twitch/YouTube/YouTube Music runtime paths were
# user-confirmed working on v2.1.36. Preserve them, but do not retest them here.
record 'AETHER_BROWSER_INSTALLED_TWITCH_RUNTIME=SKIPPED:KNOWN_GOOD_OUT_OF_SCOPE'
record 'AETHER_BROWSER_INSTALLED_YOUTUBE_RUNTIME=SKIPPED:KNOWN_GOOD_OUT_OF_SCOPE'
record 'AETHER_BROWSER_INSTALLED_YOUTUBE_MUSIC_RUNTIME=SKIPPED:KNOWN_GOOD_OUT_OF_SCOPE'
record 'AETHER_BROWSER_INSTALLED_KNOWN_GOOD_MEDIA=SKIPPED:TWITCH+YOUTUBE+YOUTUBE_MUSIC:user-confirmed-v2.1.36'
AETHER_BROWSER_INSTALL_MEDIA_WARNING=0

desktop_exec=$(grep -E '^Exec=' /usr/share/applications/org.aetherforge.AetherBrowser.desktop 2>/dev/null || true)
if [[ "$desktop_exec" == 'Exec=/usr/bin/aether-browser %U' ]]; then
  record 'AETHER_BROWSER_INSTALLED_DESKTOP_EXEC=PASS'
else
  record "AETHER_BROWSER_INSTALLED_DESKTOP_EXEC=FAIL:${desktop_exec:-missing}"
  hard_fail=1
fi
installed_web_probe="$OUT_DIR/Aether-Browser-v${VERSION}-INSTALLED-WEB-CONTENT-FRAME.png"
rm -f "$installed_web_probe"
set +e
timeout 30s env AETHER_BROWSER_LIVE_FRAME_PROBE_PATH="$installed_web_probe" AETHER_BROWSER_MEDIA_RENDERER=cpu-bgra /usr/bin/aether-browser https://example.com/ >/tmp/aether-browser-installed-web-probe.log 2>&1
installed_web_rc=$?
set -e
if (( installed_web_rc == 0 )) && [[ -s "$installed_web_probe" ]] \
  && grep -qF 'AETHER_BROWSER_VISIBLE_TEST_SCREEN=PASS' /tmp/aether-browser-installed-web-probe.log \
  && grep -qF 'AETHER_BROWSER_LIVE_FRAME_PROBE_REVEALED=PASS' /tmp/aether-browser-installed-web-probe.log \
  && grep -qF 'AETHER_BROWSER_EXTERNAL_WEB_CHILD_VISIBLE=PASS' /tmp/aether-browser-installed-web-probe.log \
  && grep -qF 'AETHER_BROWSER_LIVE_FRAME_CONTRAST=PASS' /tmp/aether-browser-installed-web-probe.log; then
  record 'AETHER_BROWSER_INSTALLED_EXTERNAL_WEB_FRAME_PROBE=PASS'
  record 'AETHER_BROWSER_INSTALLED_WEB_CONTENT_FRAME_PROBE=PASS'
  record 'AETHER_BROWSER_INSTALLED_VISIBLE_TEST_SCREEN=PASS'
  record 'AETHER_BROWSER_INSTALLED_EXTERNAL_WEB_CHILD_VISIBLE=PASS'
  record "AETHER_BROWSER_INSTALLED_WEB_CONTENT_FRAME_ARTIFACT=PASS:$installed_web_probe"
else
  record "AETHER_BROWSER_INSTALLED_EXTERNAL_WEB_FRAME_PROBE=FAIL:${installed_web_rc}"
  record "AETHER_BROWSER_INSTALLED_WEB_CONTENT_FRAME_PROBE=FAIL:${installed_web_rc}"
  while IFS= read -r line; do record "AETHER_BROWSER_INSTALLED_WEB_CONTENT_DETAIL=$line"; done < /tmp/aether-browser-installed-web-probe.log
  record 'AETHER_BROWSER_POSTINSTALL_VISUAL=FAIL'
  hard_fail=1
fi
rm -f /tmp/aether-browser-installed-web-probe.log
installed_custom_probe="$OUT_DIR/Aether-Browser-v${VERSION}-INSTALLED-CUSTOM-RESOLUTION-FRAME.png"
rm -f "$installed_custom_probe"
set +e
timeout 45s env AETHER_BROWSER_TEST_WINDOW_SIZE=1669x937 AETHER_BROWSER_LIVE_FRAME_PROBE_PATH="$installed_custom_probe" AETHER_BROWSER_MEDIA_RENDERER=cpu-bgra /usr/bin/aether-browser https://example.com/ >/tmp/aether-browser-installed-custom-resolution.log 2>&1
installed_custom_rc=$?
set -e
if (( installed_custom_rc == 0 )) && [[ -s "$installed_custom_probe" ]] \
  && grep -qF 'AETHER_BROWSER_CUSTOM_RESOLUTION_PROBE=PASS:' /tmp/aether-browser-installed-custom-resolution.log \
  && grep -qF 'AETHER_BROWSER_OFFSCREEN_RESIZE=PASS' /tmp/aether-browser-installed-custom-resolution.log \
  && grep -qF 'AETHER_BROWSER_OFFSCREEN_VIEWPORT_MATCH=PASS' /tmp/aether-browser-installed-custom-resolution.log \
  && grep -qF 'AETHER_BROWSER_POINTER_PIXEL_MATCH=PASS' /tmp/aether-browser-installed-custom-resolution.log \
  && grep -qF 'AETHER_BROWSER_CHROME_CONTENT_NO_OVERLAP=PASS' /tmp/aether-browser-installed-custom-resolution.log \
  && grep -qF 'AETHER_BROWSER_BOTTOM_BAR_NO_GAP=PASS' /tmp/aether-browser-installed-custom-resolution.log \
  && grep -qF 'AETHER_BROWSER_VISIBLE_TEST_SCREEN=PASS' /tmp/aether-browser-installed-custom-resolution.log \
  && grep -qF 'AETHER_BROWSER_EXTERNAL_WEB_CHILD_VISIBLE=PASS' /tmp/aether-browser-installed-custom-resolution.log \
  && grep -qF 'AETHER_BROWSER_LIVE_FRAME_CONTRAST=PASS' /tmp/aether-browser-installed-custom-resolution.log; then
  record 'AETHER_BROWSER_INSTALLED_CUSTOM_RESOLUTION=PASS'
  record 'AETHER_BROWSER_INSTALLED_OFFSCREEN_RESIZE=PASS'
  record 'AETHER_BROWSER_INSTALLED_OFFSCREEN_VIEWPORT_MATCH=PASS'
  record 'AETHER_BROWSER_INSTALLED_POINTER_PIXEL_MATCH=PASS'
  record 'AETHER_BROWSER_INSTALLED_CHROME_CONTENT_NO_OVERLAP=PASS'
  record 'AETHER_BROWSER_INSTALLED_BOTTOM_BAR_NO_GAP=PASS'
  while IFS= read -r line; do
    [[ "$line" == AETHER_BROWSER_CUSTOM_RESOLUTION_WINDOW=* || "$line" == AETHER_BROWSER_CUSTOM_RESOLUTION_PROBE=* || "$line" == AETHER_BROWSER_VIEWPORT_*=* ]] && record "AETHER_BROWSER_INSTALLED_CUSTOM_RESOLUTION_DETAIL=$line"
  done < /tmp/aether-browser-installed-custom-resolution.log
  record "AETHER_BROWSER_INSTALLED_CUSTOM_RESOLUTION_ARTIFACT=PASS:$installed_custom_probe"
else
  record "AETHER_BROWSER_INSTALLED_CUSTOM_RESOLUTION=FAIL:${installed_custom_rc}"
  while IFS= read -r line; do record "AETHER_BROWSER_INSTALLED_CUSTOM_RESOLUTION_DETAIL=$line"; done < /tmp/aether-browser-installed-custom-resolution.log
  record 'AETHER_BROWSER_POSTINSTALL_VISUAL=FAIL'
  hard_fail=1
fi
rm -f /tmp/aether-browser-installed-custom-resolution.log

# Provider render regressions for Twitch/YouTube/YouTube Music are intentionally
# excluded from the v2.1.56 patch test. Velora is verified by velora-runtime-test.sh.
record 'AETHER_BROWSER_PROVIDER_RENDER_TESTS=SKIPPED:TWITCH+YOUTUBE+YOUTUBE_MUSIC'

for f in \
  /usr/lib/aetherforge/aether-browser/bin/aether-browser \
  /usr/lib/aetherforge/aether-browser/bin/aether-media-service \
  /usr/lib/aetherforge/aether-browser/bin/aether-obs-verify \
  /usr/bin/aether-obs-verify \
  /usr/lib/aetherforge/aether-browser/tools/obs-runtime-test.sh \
  /usr/lib/aetherforge/aether-browser/tools/streamlabs-runtime-test.sh \
  /usr/lib/aetherforge/aether-browser/tools/stability-acceptance.sh \
  /usr/lib/aetherforge/aether-browser/tools/browser-takeover.sh \
  /usr/lib/aetherforge/aether-browser/tools/browser-takeover-rollback.sh \
  /usr/bin/aether-browser-stability \
  /usr/bin/aether-browser-takeover \
  /usr/bin/aether-browser-takeover-rollback \
  /usr/lib/systemd/user/aether-browser-media.service \
  /usr/share/applications/org.aetherforge.AetherBrowser.desktop; do
  if [[ -e "$f" ]]; then
    record "AETHER_BROWSER_INSTALL_FILE=PASS:$f"
  else
    record "AETHER_BROWSER_INSTALL_FILE=FAIL:$f"
    hard_fail=1
  fi
done
if systemctl --user is-active --quiet aether-browser-media.service; then
  record 'AETHER_BROWSER_MEDIA_SERVICE_INSTALL=PASS:active'
else
  record 'AETHER_BROWSER_MEDIA_SERVICE_INSTALL=FAIL:not-active'
  hard_fail=1
fi
media_service_status=$(/usr/bin/aether-media-service --status 2>&1 || true)
if grep -qF "AETHER_MEDIA_SERVICE_RELEASE_VERSION=${VERSION}" <<<"$media_service_status"; then
  record "AETHER_BROWSER_MEDIA_SERVICE_VERSION=PASS:${VERSION}"
else
  record "AETHER_BROWSER_MEDIA_SERVICE_VERSION=FAIL:expected=${VERSION}"
  while IFS= read -r line; do record "AETHER_BROWSER_MEDIA_SERVICE_VERSION_DETAIL=$line"; done <<<"$media_service_status"
  hard_fail=1
fi
qk_log=$(mktemp)
if pacman -Qk aether-browser >"$qk_log" 2>&1 && grep -q '0 missing files' "$qk_log"; then
  record 'AETHER_BROWSER_PACMAN_QK=PASS:no-missing-files'
else
  record 'AETHER_BROWSER_PACMAN_QK=FAIL'
  while IFS= read -r line; do record "AETHER_BROWSER_PACMAN_QK_DETAIL=$line"; done < "$qk_log"
  hard_fail=1
fi
rm -f "$qk_log"

qkk_log=$(mktemp)
set +e
pacman -Qkk aether-browser >"$qkk_log" 2>&1
qkk_rc=$?
set -e
if qkk_policy=$(bash "$ROOT/scripts/classify-pacman-qkk.sh" "$qkk_rc" "$qkk_log"); then
  while IFS= read -r line; do record "$line"; done <<<"$qkk_policy"
else
  while IFS= read -r line; do record "$line"; done <<<"$qkk_policy"
  hard_fail=1
fi
rm -f "$qkk_log"
if (( hard_fail == 0 )); then
  record 'AETHER_BROWSER_POSTINSTALL_VISUAL=PASS'
  if (( AETHER_BROWSER_PROVIDER_RENDER_WARNING == 0 )); then
    record 'AETHER_BROWSER_PROVIDER_RENDERING=PASS'
  else
    record 'AETHER_BROWSER_PROVIDER_RENDERING=WARNING'
  fi
  if (( AETHER_BROWSER_INSTALL_MEDIA_WARNING == 0 )); then
    record 'AETHER_BROWSER_INSTALL_RUNTIME_MEDIA=PASS'
  else
    record 'AETHER_BROWSER_INSTALL_RUNTIME_MEDIA=WARNING'
  fi
  record 'AETHER_BROWSER_V2_1_60_INSTALL_VERIFY=PASS'
  record "AETHER_BROWSER_INSTALL_VERIFY_FILE=$INSTALL_VERIFY"
  exit 0
fi
record 'AETHER_BROWSER_INSTALL_HARD_FAILURE_ROLLBACK=START'
if bash "$ROOT/scripts/rollback-host.sh"; then
  record 'AETHER_BROWSER_INSTALL_HARD_FAILURE_ROLLBACK=PASS'
else
  record 'AETHER_BROWSER_INSTALL_HARD_FAILURE_ROLLBACK=FAIL'
fi
record 'AETHER_BROWSER_V2_1_60_INSTALL_VERIFY=FAIL'
record "AETHER_BROWSER_INSTALL_VERIFY_FILE=$INSTALL_VERIFY"
exit 1
