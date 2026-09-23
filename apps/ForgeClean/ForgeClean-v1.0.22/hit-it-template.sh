#!/usr/bin/env bash
set -uo pipefail

VERSION="1.0.22"
DOWNLOADS="${HOME}/Downloads"
ZIP="${DOWNLOADS}/ForgeClean-v${VERSION}-SOURCE.zip"
BUILD_DIR="${DOWNLOADS}/ForgeClean-v${VERSION}"
VERIFY="${DOWNLOADS}/ForgeClean-v${VERSION}-VERIFY.txt"
EXPECTED_SHA="__SOURCE_SHA256__"

mkdir -p "$DOWNLOADS" || exit 1
: > "$VERIFY"

emit() { printf '%s\n' "$*" | tee -a "$VERIFY"; }
finalize() {
  local rc=$?
  trap - EXIT
  if [[ $rc -eq 0 ]]; then
    emit "FORGECLEAN_HIT_IT=PASS"
  else
    emit "FORGECLEAN_HIT_IT=FAIL:${rc}"
  fi
  emit "VERIFY_FILE=${VERIFY}"
  exit "$rc"
}
trap finalize EXIT

fail_now() {
  local rc="$1"; shift
  emit "$*"
  exit "$rc"
}

run_stage() {
  local label="$1"; shift
  emit "HIT_STAGE=${label}"
  if "$@" >>"$VERIFY" 2>&1; then
    emit "${label}=PASS"
    return 0
  else
    local rc=$?
    emit "${label}=FAIL:${rc}"
    exit "$rc"
  fi
}

extract_source() {
  rm -rf -- "$BUILD_DIR"
  if command -v bsdtar >/dev/null 2>&1; then
    bsdtar -xf "$ZIP" -C "$DOWNLOADS"
  elif command -v unzip >/dev/null 2>&1; then
    unzip -q "$ZIP" -d "$DOWNLOADS"
  else
    echo "FORGECLEAN_EXTRACT=FAIL:NEED_BSDTAR_OR_UNZIP"
    return 127
  fi
}

normalize_source_permissions() {
  [[ -d "$BUILD_DIR" ]] || return 1
  chmod 0755 "$BUILD_DIR"
  for dir in src tests docs systemd; do
    if [[ -d "$BUILD_DIR/$dir" ]]; then
      chmod 0755 "$BUILD_DIR/$dir"
    fi
  done
  chmod -R u+rwX,go+rX "$BUILD_DIR"
}

emit "FORGECLEAN_VERSION=${VERSION}"
emit "FORGECLEAN_VERIFY_ARTIFACT=PRECREATED"
cd "$DOWNLOADS" || fail_now 1 "FORGECLEAN_DOWNLOADS=FAIL:CD:${DOWNLOADS}"
[[ -f "$ZIP" ]] || fail_now 1 "FORGECLEAN_SOURCE=FAIL:NOT_FOUND:${ZIP}"
ACTUAL_SHA="$(sha256sum "$ZIP" | awk '{print $1}')" || fail_now 1 "FORGECLEAN_SOURCE_SHA256=FAIL:UNREADABLE"
[[ "$ACTUAL_SHA" == "$EXPECTED_SHA" ]] || fail_now 1 "FORGECLEAN_SOURCE_SHA256=FAIL:EXPECTED=${EXPECTED_SHA}:ACTUAL=${ACTUAL_SHA}"
emit "FORGECLEAN_SOURCE_SHA256=PASS"

run_stage FORGECLEAN_EXTRACT extract_source
run_stage FORGECLEAN_PERMISSION_NORMALIZE normalize_source_permissions
cd "$BUILD_DIR" || fail_now 1 "FORGECLEAN_BUILD_DIR=FAIL:CD:${BUILD_DIR}"
run_stage FORGECLEAN_PERMISSIONS chmod +x build-and-verify.sh install-local.sh tests/*.sh
run_stage FORGECLEAN_V0_3_0_REGRESSION ./tests/regression_v0_3_0.sh
run_stage FORGECLEAN_V0_3_0_PERSISTENCE ./tests/regression_v0_3_0_persistence.sh
run_stage FORGECLEAN_V0_3_1_PARTIAL_MOVE ./tests/regression_v0_3_1_partial_move.sh
run_stage FORGECLEAN_V0_3_2_COLDSTORE_STACK ./tests/regression_v0_3_2_coldstore_stack.sh
run_stage FORGECLEAN_V0_3_2_REGRESSION ./tests/regression_v0_3_2.sh
run_stage FORGECLEAN_V0_4_0_REGRESSION ./tests/regression_v0_4_0.sh
run_stage FORGECLEAN_V0_4_1_CLIPPY ./tests/regression_v0_4_1_clippy.sh
run_stage FORGECLEAN_V0_4_2_VERIFY_ARTIFACT ./tests/regression_v0_4_2_verify_artifact.sh
run_stage FORGECLEAN_V0_5_0_REGRESSION ./tests/regression_v0_5_0.sh
run_stage FORGECLEAN_V0_5_1_HOST_FAILURES ./tests/regression_v0_5_1_host_failures.sh
run_stage FORGECLEAN_V0_5_2_CLIPPY_TEST ./tests/regression_v0_5_2_clippy_test.sh
run_stage FORGECLEAN_V0_5_2_BUILD_AWARE ./tests/regression_v0_5_2_build_aware.sh
run_stage FORGECLEAN_V0_6_0_GUI ./tests/regression_v0_6_0_gui.sh
run_stage FORGECLEAN_V0_6_1_PERMISSIONS ./tests/regression_v0_6_1_permissions.sh
run_stage FORGECLEAN_V0_6_2_EGUI_API ./tests/regression_v0_6_2_egui_api.sh
run_stage FORGECLEAN_V0_6_3_TITLEBAR_BORROW ./tests/regression_v0_6_3_titlebar_borrow.sh
run_stage FORGECLEAN_V0_6_4_GUI_CLIPPY ./tests/regression_v0_6_4_gui_clippy.sh
run_stage FORGECLEAN_V1_0_1_PROMOTION ./tests/regression_v1_0_1_promotion.sh
run_stage FORGECLEAN_V1_0_5_ORBITAL_UI ./tests/regression_v1_0_5_orbital_ui.sh
run_stage FORGECLEAN_V1_0_7_VERIFIED_AUTHORITY ./tests/regression_v1_0_7_verified_authority.sh
run_stage FORGECLEAN_V1_0_10_ATOMIC_ORBIT ./tests/regression_v1_0_10_atomic_orbit.sh
run_stage FORGECLEAN_V1_0_11_CLIPPY_GATE ./tests/regression_v1_0_11_clippy_gate.sh
run_stage FORGECLEAN_V1_0_16_PRE_REBASE ./tests/regression_v1_0_16_pre_rebase.sh
run_stage FORGECLEAN_V1_0_16_RUST_FIRST ./tests/regression_v1_0_16_rust_first.sh
run_stage FORGECLEAN_V1_0_19_PROJECT_STORAGE ./tests/regression_v1_0_19_project_storage.sh
run_stage FORGECLEAN_V1_0_21_DOWNLOADS_INVENTORY ./tests/regression_v1_0_21_downloads_inventory.sh
run_stage FORGECLEAN_BUILD_VERIFY ./build-and-verify.sh
run_stage FORGECLEAN_INSTALL ./install-local.sh

emit "FORGECLEAN_V1_0_21_HOSTILE_TAKEOVER=PASS"
emit "FORGECLEAN_GUI=PASS"
emit "FORGECLEAN_DRAGONGLASS=PASS"
emit "FORGECLEAN_COLDPACK_GC=PASS"
emit "FORGECLEAN_COLDPACK=PASS"
emit "FORGECLEAN_PERSISTENT=PASS"
emit "FORGECLEAN_PRE_REBASE=PASS"
emit "FORGECLEAN_PRE_REBASE_TIMER=PASS"
emit "FORGECLEAN_RUST_NATIVE=PASS"
emit "FORGECLEAN_SERVICE=forgeclean-organizer.service"
emit "FORGECLEAN_ROOT=${HOME}/Downloads/ForgeClean"
exit 0
