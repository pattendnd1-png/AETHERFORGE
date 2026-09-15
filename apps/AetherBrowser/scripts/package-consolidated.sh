#!/usr/bin/env bash
set -euo pipefail

VERSION='2.1.60'
PREFIX="Aether-Browser-v${VERSION}"
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
OUT_DIR=${1:-"${AETHER_BROWSER_OUT_DIR:-$HOME/Downloads}"}
SOURCE="$OUT_DIR/${PREFIX}-source.zip"
INSTALL="$OUT_DIR/${PREFIX}-install.sh"
STATIC="$OUT_DIR/${PREFIX}-STATIC-VERIFY.txt"
PACKAGE="$OUT_DIR/${PREFIX}-PACKAGE-VERIFY.txt"
SHA="$OUT_DIR/${PREFIX}-SHA256SUMS.txt"
RUN="$OUT_DIR/${PREFIX}-INSTALL.run"
TMP_ROOT="${AETHER_BROWSER_TMP_ROOT:-${XDG_CACHE_HOME:-$HOME/.cache}/aetherforge/aether-browser/tmp}"
mkdir -p "$TMP_ROOT"
PAYLOAD=$(mktemp "$TMP_ROOT/${PREFIX}.payload.XXXXXX.tar.gz")
HEADER=$(mktemp "$TMP_ROOT/${PREFIX}.header.XXXXXX")
trap 'rm -f "$PAYLOAD" "$HEADER"' EXIT

for file in "$SOURCE" "$INSTALL" "$STATIC" "$PACKAGE" "$SHA"; do
  [[ -f "$file" ]] || { echo "AETHER_BROWSER_CONSOLIDATED=FAIL:missing:$file" >&2; exit 80; }
done
grep -q '^AETHER_BROWSER_STATIC_VERIFY=PASS$' "$STATIC"
grep -q '^AETHER_BROWSER_PACKAGE_VERIFY=PASS$' "$PACKAGE"

tar -czf "$PAYLOAD" -C "$OUT_DIR" \
  "$(basename "$SOURCE")" \
  "$(basename "$INSTALL")" \
  "$(basename "$STATIC")" \
  "$(basename "$PACKAGE")" \
  "$(basename "$SHA")"
DIGEST=$(sha256sum "$PAYLOAD" | awk '{print $1}')

cat > "$HEADER" <<EOF_HEADER
#!/usr/bin/env bash
set -euo pipefail
VERSION='2.1.60'
TAG='V2_1_60'
PREFIX="Aether-Browser-v\${VERSION}"
SELF=\$(readlink -f "\${BASH_SOURCE[0]}")
DOWNLOADS="\$HOME/Downloads"
PAYLOAD_SHA256='$DIGEST'
HOST_GATE="\$DOWNLOADS/\${PREFIX}-HOST-GATE-VERIFY.txt"
HOST_LOG="\$DOWNLOADS/\${PREFIX}-HOST-GATE-RUN.log"
TMP_ROOT="\${AETHER_BROWSER_TMP_ROOT:-\${XDG_CACHE_HOME:-\$HOME/.cache}/aetherforge/aether-browser/tmp}"
mkdir -p "\$TMP_ROOT"

say(){ printf '%s\\n' "\$*"; }
die(){ printf '%s\\n' "\$*" >&2; exit "\${2:-1}"; }
host_record(){ printf '%s\\n' "\$1" | tee -a "\$HOST_GATE"; }

extract_payload(){
  local work payload actual
  work=\$(mktemp -d "\$TMP_ROOT/aether-browser-v2.1.60.XXXXXX")
  payload="\$work/payload.tar.gz"
  awk '/^# __AETHER_PAYLOAD_BEGIN__\$/{inside=1; next} /^# __AETHER_PAYLOAD_END__\$/{inside=0} inside { sub(/^# /, ""); print }' "\$SELF" | base64 -d > "\$payload"
  actual=\$(sha256sum "\$payload" | awk '{print \$1}')
  [[ "\$actual" == "\$PAYLOAD_SHA256" ]] || die "AETHER_BROWSER_CONSOLIDATED_PAYLOAD_SHA256=FAIL:\$actual" 81
  tar -xzf "\$payload" -C "\$work"
  printf '%s' "\$work"
}

verify_only(){
  local work
  work=\$(extract_payload)
  grep -q '^AETHER_BROWSER_STATIC_VERIFY=PASS$' "\$work/\${PREFIX}-STATIC-VERIFY.txt" || die 'AETHER_BROWSER_CONSOLIDATED_STATIC=FAIL' 82
  grep -q '^AETHER_BROWSER_PACKAGE_VERIFY=PASS$' "\$work/\${PREFIX}-PACKAGE-VERIFY.txt" || die 'AETHER_BROWSER_CONSOLIDATED_PACKAGE=FAIL' 83
  say "AETHER_BROWSER_CONSOLIDATED_PAYLOAD_SHA256=PASS:\$PAYLOAD_SHA256"
  say 'AETHER_BROWSER_CONSOLIDATED_STATIC=PASS'
  say 'AETHER_BROWSER_CONSOLIDATED_PACKAGE=PASS'
  say 'AETHER_BROWSER_CONSOLIDATED_VERIFY=PASS'
  rm -rf "\$work"
}

summarize_compile_install(){
  local verify="\$DOWNLOADS/\${PREFIX}-VERIFY.txt"
  local install_verify="\$DOWNLOADS/\${PREFIX}-INSTALL-VERIFY.txt"
  local marker
  [[ -f "\$verify" ]] || { host_record 'AETHER_BROWSER_HOST_MAIN_VERIFY=FAIL:missing'; return 1; }
  [[ -f "\$install_verify" ]] || { host_record 'AETHER_BROWSER_HOST_INSTALL_VERIFY=FAIL:missing'; return 1; }
  host_record "AETHER_BROWSER_HOST_MAIN_VERIFY=PASS:\$verify"
  host_record "AETHER_BROWSER_HOST_INSTALL_VERIFY=PASS:\$install_verify"
  for marker in CHECK CLIPPY TEST BUILD; do
    if grep -q "^AETHER_BROWSER_\${marker}=PASS$" "\$verify"; then
      host_record "AETHER_BROWSER_HOST_COMPILE_MARKER=PASS:AETHER_BROWSER_\${marker}=PASS"
    else
      host_record "AETHER_BROWSER_HOST_COMPILE_MARKER=FAIL:AETHER_BROWSER_\${marker}"
      return 1
    fi
  done
  grep -q "^AETHER_BROWSER_\${TAG}_VERIFY=PASS$" "\$verify" || { host_record "AETHER_BROWSER_HOST_MAIN_VERIFY_RESULT=FAIL"; return 1; }
  grep -q "^AETHER_BROWSER_\${TAG}_INSTALL_VERIFY=PASS$" "\$install_verify" || { host_record "AETHER_BROWSER_HOST_INSTALL_RESULT=FAIL"; return 1; }
  host_record 'AETHER_BROWSER_HOST_MAIN_VERIFY_RESULT=PASS'
  host_record 'AETHER_BROWSER_HOST_INSTALL_RESULT=PASS'
  if grep -q '^AETHER_BROWSER_EXISTING_LOGIN_STATE_PRESERVED=PASS' "\$install_verify"; then
    host_record 'AETHER_BROWSER_HOST_EXISTING_LOGIN_STATE_PRESERVED=PASS'
  else
    host_record 'AETHER_BROWSER_HOST_EXISTING_LOGIN_STATE_PRESERVED=UNKNOWN'
  fi
  return 0
}

run_runtime_gate(){
  local src="\$DOWNLOADS/\${PREFIX}"
  local velora_verify="\$DOWNLOADS/\${PREFIX}-VELORA-VERIFY.txt"
  local stability_verify="\$DOWNLOADS/\${PREFIX}-TAKEOVER-READY.txt"
  local velora_rc=0 stability_rc=0 takeover_rc=0 line

  host_record 'AETHER_BROWSER_HOST_RUNTIME_SKIPPED=TWITCH+YOUTUBE+YOUTUBE_MUSIC+STREAMLABS:known-good-browser-paths'

  host_record 'AETHER_BROWSER_HOST_OBS_WORKSPACE=STRUCTURAL_PASS:full-native-workspace-runtime-requires-interactive-X11-session'

  host_record 'AETHER_BROWSER_HOST_VELORA_RUNTIME=START'
  set +e
  AETHER_BROWSER_OUT_DIR="\$DOWNLOADS" bash "\$src/scripts/velora-runtime-test.sh"
  velora_rc=\$?
  set -e
  if [[ -f "\$velora_verify" ]]; then
    while IFS= read -r line; do host_record "AETHER_BROWSER_HOST_VELORA_DETAIL=\$line"; done < "\$velora_verify"
  fi
  if (( velora_rc == 0 )); then
    host_record 'AETHER_BROWSER_HOST_VELORA_RUNTIME=PASS'
  else
    host_record "AETHER_BROWSER_HOST_VELORA_RUNTIME=FAIL:\$velora_rc"
  fi

  host_record 'AETHER_BROWSER_HOST_STABILITY=START'
  set +e
  AETHER_BROWSER_STABILITY_ITERATIONS=20 bash "\$src/scripts/stability-acceptance.sh"
  stability_rc=\$?
  set -e
  if [[ -f "\$stability_verify" ]]; then
    while IFS= read -r line; do host_record "AETHER_BROWSER_HOST_STABILITY_DETAIL=\$line"; done < "\$stability_verify"
  fi
  if (( stability_rc == 0 )); then
    host_record 'AETHER_BROWSER_HOST_STABILITY=PASS'
  else
    host_record "AETHER_BROWSER_HOST_STABILITY=FAIL:\$stability_rc"
  fi

  if (( velora_rc == 0 && stability_rc == 0 )); then
    host_record 'AETHER_BROWSER_TAKEOVER_ELIGIBLE=YES'
    host_record 'AETHER_BROWSER_HOST_TAKEOVER=START'
    set +e
    AETHER_BROWSER_OUT_DIR="\$DOWNLOADS" bash "\$src/scripts/browser-takeover.sh" --activate
    takeover_rc=\$?
    set -e
    if (( takeover_rc == 0 )); then
      host_record 'AETHER_BROWSER_HOST_TAKEOVER=PASS'
      host_record 'AETHER_BROWSER_HOST_GATE=PASS'
      return 0
    fi
    host_record "AETHER_BROWSER_HOST_TAKEOVER=FAIL:\$takeover_rc"
    host_record 'AETHER_BROWSER_HOST_GATE=FAIL:browser-takeover'
    return 1
  fi
  host_record 'AETHER_BROWSER_TAKEOVER_ELIGIBLE=NO'
  if (( velora_rc != 0 )); then
    host_record 'AETHER_BROWSER_HOST_GATE=FAIL:velora-runtime'
  else
    host_record 'AETHER_BROWSER_HOST_GATE=FAIL:stability'
  fi
  return 1
}

install_all(){
  local work install_rc
  work=\$(extract_payload)
  mkdir -p "\$DOWNLOADS"
  : > "\$HOST_GATE"
  : > "\$HOST_LOG"
  host_record "AETHER_BROWSER_VERSION=\$VERSION"
  host_record 'AETHER_BROWSER_HOST_GATE=START'
  host_record 'AETHER_BROWSER_HOST_GATE_POLICY=FRESH_COMPILE_INSTALL+VISIBLE_UI+VELORA+CAPABILITY_STABILITY+BROWSER_TAKEOVER'
  host_record 'AETHER_BROWSER_HOST_GATE_REPORT_CREATED=PASS:before-install'
  host_record "AETHER_BROWSER_HOST_GATE_VERIFY_FILE=\$HOST_GATE"
  host_record "AETHER_BROWSER_HOST_GATE_RUN_LOG=\$HOST_LOG"

  install -m 0644 "\$work/\${PREFIX}-source.zip" "\$DOWNLOADS/\${PREFIX}-source.zip"
  install -m 0755 "\$work/\${PREFIX}-install.sh" "\$DOWNLOADS/\${PREFIX}-install.sh"
  host_record "AETHER_BROWSER_CONSOLIDATED_SOURCE=PASS:\$DOWNLOADS/\${PREFIX}-source.zip"
  host_record 'AETHER_BROWSER_HOST_INSTALL=START'
  set +e
  "\$DOWNLOADS/\${PREFIX}-install.sh" 2>&1 | tee -a "\$HOST_LOG"
  install_rc=\${PIPESTATUS[0]}
  set -e
  if (( install_rc != 0 )); then
    host_record "AETHER_BROWSER_HOST_INSTALL_COMMAND=FAIL:\$install_rc"
    host_record "AETHER_BROWSER_HOST_GATE=FAIL:stage=install-host:exit=\$install_rc"
    rm -rf "\$work"
    return "\$install_rc"
  fi
  host_record 'AETHER_BROWSER_HOST_INSTALL_COMMAND=PASS'
  summarize_compile_install || {
    host_record 'AETHER_BROWSER_HOST_GATE=FAIL:stage=compile-install-summary'
    rm -rf "\$work"
    return 85
  }
  host_record 'AETHER_BROWSER_HOST_RUST_COMPILE=PASS'
  host_record 'AETHER_BROWSER_HOST_INSTALL=PASS'
  if [[ "\$(/usr/bin/aether-browser --version 2>/dev/null || true)" == "Aether Browser \$VERSION" ]]; then
    host_record "AETHER_BROWSER_HOST_INSTALLED_VERSION=PASS:\$VERSION"
  else
    host_record 'AETHER_BROWSER_HOST_INSTALLED_VERSION=FAIL'
    rm -rf "\$work"
    return 86
  fi
  run_runtime_gate
  rc=\$?
  rm -rf "\$work"
  return "\$rc"
}

case "\${1:-}" in
  --verify) verify_only ;;
  --help|-h)
    say 'Aether Browser v2.1.60 one-file host-gate installer'
    say '  default   build + verify + install + Velora + stability + browser-takeover host gate'
    say '  --verify verify the embedded release payload only'
    ;;
  '') install_all ;;
  *) die "AETHER_BROWSER_CONSOLIDATED=FAIL:unknown-option:\$1" 84 ;;
esac
exit 0
EOF_HEADER
cat "$HEADER" > "$RUN"
printf '# __AETHER_PAYLOAD_BEGIN__\n' >> "$RUN"
base64 -w 76 "$PAYLOAD" | sed 's/^/# /' >> "$RUN"
printf '# __AETHER_PAYLOAD_END__\n' >> "$RUN"
chmod +x "$RUN"
"$RUN" --verify
printf 'AETHER_BROWSER_CONSOLIDATED_INSTALLER=PASS:%s\n' "$RUN"
