#!/usr/bin/env bash
set -uo pipefail

VERSION="1.0.11"
NAME="ForgeClean-v${VERSION}"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
OUT_DIR="${HOME}/Downloads"
VERIFY="${OUT_DIR}/${NAME}-VERIFY.txt"
SUMS="${OUT_DIR}/${NAME}-SHA256SUMS.txt"
BIN_OUT="${OUT_DIR}/${NAME}-forgeclean"
GUI_OUT="${OUT_DIR}/${NAME}-forgeclean-gui"
mkdir -p "$OUT_DIR"
: > "$VERIFY"

log() { printf '%s\n' "$*" | tee -a "$VERIFY"; }
run_gate() {
  local label="$1"; shift
  if "$@" >>"$VERIFY" 2>&1; then
    log "${label}=PASS"
    return 0
  else
    local rc=$?
    log "${label}=FAIL:${rc}"
    return "$rc"
  fi
}

log "FORGECLEAN_VERSION=${VERSION}"
log "FORGECLEAN_SOURCE=${SCRIPT_DIR}"
log "FORGECLEAN_DELETE_MODE=DIRECT_UNLINK_NO_TRASH"
log "FORGECLEAN_DEFAULT_KEEP_VERSIONS=2"
log "FORGECLEAN_AUTO_POLICY=NO_EXTERNAL_AUTO_CLEAN__EXTERNAL_AUTO_OFFLOAD"
log "FORGECLEAN_OFFLOAD_COMMIT=SHA256_FSYNC_REVERIFY_THEN_UNLINK"
log "FORGECLEAN_ORGANIZER_POLICY=PERSISTENT_SYSTEMD_USER_SERVICE"
log "FORGECLEAN_COLDSTORE_FORMAT=FCOLDPACK_CDC_DEDUP_ZSTD_SHA256"
log "FORGECLEAN_COLDPACK_GC_POLICY=MARK_SWEEP_7_DAY_QUARANTINE"
log "FORGECLEAN_COLDPACK_LOCK=STD_FILE_SHARED_EXCLUSIVE"
log "FORGECLEAN_GUI=EFRAme_EGUI_NATIVE_DRAGONGLASS"
log "FORGECLEAN_GUI_TRANSPARENCY=90_PERCENT_TRANSPARENT_10_PERCENT_SMOKY_GLASSY"

for tool in cargo pacman vercmp; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    log "FORGECLEAN_${tool^^}=FAIL:NOT_FOUND"
    log "FORGECLEAN_VERIFY=FAIL"
    log "VERIFY_FILE=${VERIFY}"
    exit 127
  fi
done
if [[ ! -x /usr/bin/lsblk ]]; then
  log "FORGECLEAN_LSBLK=FAIL:NOT_FOUND:/usr/bin/lsblk"
  log "FORGECLEAN_VERIFY=FAIL"
  log "VERIFY_FILE=${VERIFY}"
  exit 127
fi
log "FORGECLEAN_PACMAN=PASS"
log "FORGECLEAN_CARGO=PASS"
log "FORGECLEAN_VERCMP=PASS"
log "FORGECLEAN_LSBLK=PASS"

cd "$SCRIPT_DIR" || exit 1
status=0
run_gate FORGECLEAN_WORKSPACE_ISOLATION_STATIC ./tests/workspace_isolation.sh || status=1
run_gate FORGECLEAN_V0_1_2_REGRESSION ./tests/regression_v0_1_2.sh || status=1
run_gate FORGECLEAN_V0_1_3_REGRESSION ./tests/regression_v0_1_3.sh || status=1
run_gate FORGECLEAN_V0_1_4_REGRESSION ./tests/regression_v0_1_4.sh || status=1
run_gate FORGECLEAN_V0_2_0_REGRESSION ./tests/regression_v0_2_0.sh || status=1
run_gate FORGECLEAN_CLIPPY_QUESTION_MARK_REGRESSION ./tests/regression_clippy_question_mark.sh || status=1
run_gate FORGECLEAN_V0_2_1_REGRESSION ./tests/regression_v0_2_1.sh || status=1
run_gate FORGECLEAN_V0_2_2_REGRESSION ./tests/regression_v0_2_2.sh || status=1
run_gate FORGECLEAN_V0_3_0_REGRESSION ./tests/regression_v0_3_0.sh || status=1
run_gate FORGECLEAN_V0_3_0_PERSISTENCE ./tests/regression_v0_3_0_persistence.sh || status=1
run_gate FORGECLEAN_V0_3_1_PARTIAL_MOVE ./tests/regression_v0_3_1_partial_move.sh || status=1
run_gate FORGECLEAN_V0_3_2_COLDSTORE_STACK ./tests/regression_v0_3_2_coldstore_stack.sh || status=1
run_gate FORGECLEAN_V0_3_2_REGRESSION ./tests/regression_v0_3_2.sh || status=1
run_gate FORGECLEAN_V0_4_0_REGRESSION ./tests/regression_v0_4_0.sh || status=1
run_gate FORGECLEAN_V0_4_1_CLIPPY ./tests/regression_v0_4_1_clippy.sh || status=1
run_gate FORGECLEAN_V0_4_2_VERIFY_ARTIFACT ./tests/regression_v0_4_2_verify_artifact.sh || status=1
run_gate FORGECLEAN_V0_5_0_REGRESSION ./tests/regression_v0_5_0.sh || status=1
run_gate FORGECLEAN_V0_5_1_HOST_FAILURES ./tests/regression_v0_5_1_host_failures.sh || status=1
run_gate FORGECLEAN_V0_5_2_CLIPPY_TEST ./tests/regression_v0_5_2_clippy_test.sh || status=1
run_gate FORGECLEAN_V0_5_2_BUILD_AWARE ./tests/regression_v0_5_2_build_aware.sh || status=1
run_gate FORGECLEAN_V0_6_0_GUI ./tests/regression_v0_6_0_gui.sh || status=1
run_gate FORGECLEAN_V0_6_1_PERMISSIONS ./tests/regression_v0_6_1_permissions.sh || status=1
run_gate FORGECLEAN_V0_6_2_EGUI_API ./tests/regression_v0_6_2_egui_api.sh || status=1
run_gate FORGECLEAN_V0_6_3_TITLEBAR_BORROW ./tests/regression_v0_6_3_titlebar_borrow.sh || status=1
run_gate FORGECLEAN_V0_6_4_GUI_CLIPPY ./tests/regression_v0_6_4_gui_clippy.sh || status=1
run_gate FORGECLEAN_V1_0_1_PROMOTION ./tests/regression_v1_0_1_promotion.sh || status=1
run_gate FORGECLEAN_V1_0_5_ORBITAL_UI ./tests/regression_v1_0_5_orbital_ui.sh || status=1
run_gate FORGECLEAN_V1_0_10_ATOMIC_ORBIT ./tests/regression_v1_0_10_atomic_orbit.sh || status=1
run_gate FORGECLEAN_V1_0_7_VERIFIED_AUTHORITY ./tests/regression_v1_0_7_verified_authority.sh || status=1
run_gate FORGECLEAN_ORBITAL_MONITOR_TEST cargo test --test orbital_monitor || status=1
run_gate FORGECLEAN_WORKSPACE_ISOLATION cargo metadata --manifest-path Cargo.toml --no-deps --format-version 1 || status=1
run_gate FORGECLEAN_FMT_APPLY cargo fmt --all || status=1
run_gate FORGECLEAN_FMT cargo fmt --all -- --check || status=1
run_gate FORGECLEAN_PACKAGE_TEST cargo test --test package || status=1
run_gate FORGECLEAN_SYSTEM_SCAN_UNIT_TEST cargo test --test system_scan || status=1
run_gate FORGECLEAN_ORGANIZER_TEST cargo test --test organizer || status=1
run_gate FORGECLEAN_BUILD_BUNDLE_UNIT_TEST cargo test --test organizer organizer_groups_build_bundle_and_leaves_legacy_aliases || status=1
run_gate FORGECLEAN_EXISTING_BUILD_RECONCILE_UNIT_TEST cargo test --test organizer organizer_reconciles_existing_legacy_buckets_into_project_builds || status=1
run_gate FORGECLEAN_CONTINUOUS_BUILD_RECONCILE_UNIT_TEST cargo test --test organizer organizer_continuously_reconciles_new_artifacts_in_legacy_project_areas || status=1
run_gate FORGECLEAN_GUI_STATE_TEST cargo test --test gui_state || status=1
run_gate FORGECLEAN_REGISTRY_TEST cargo test --test registry || status=1
run_gate FORGECLEAN_COLDSTORE_DEEP_TREE_TEST cargo test --test coldstore deeply_nested_tree_archives_and_restores_without_stack_exhaustion || status=1
run_gate FORGECLEAN_COLDPACK_DEDUP_TEST cargo test --test coldstore coldpack_shared_store_reuses_identical_chunks_across_archives || status=1
run_gate FORGECLEAN_COLDPACK_RESTORE_TEST cargo test --test coldstore coldpack_restore_reconstructs_exact_bytes_from_shared_store || status=1
run_gate FORGECLEAN_COLDPACK_CORRUPTION_GUARD cargo test --test coldstore coldpack_corrupt_reused_object_preserves_new_source || status=1
run_gate FORGECLEAN_LEGACY_COLD_RESTORE_TEST cargo test --test coldstore legacy_tar_zst_archive_remains_restorable || status=1
run_gate FORGECLEAN_COLDSTORE_TEST cargo test --test coldstore || status=1
run_gate FORGECLEAN_COLDPACK_LOCK_TEST cargo test coldpack_store_exclusive_lock_blocks_second_writer || status=1
run_gate FORGECLEAN_COLDPACK_GC_STATUS_TEST cargo test --test coldpack_gc status_reports_live_and_orphan_objects_without_mutation || status=1
run_gate FORGECLEAN_COLDPACK_GC_CORRUPTION_TEST cargo test --test coldpack_gc audit_blocks_on_corrupt_referenced_object || status=1
run_gate FORGECLEAN_COLDPACK_GC_QUARANTINE_TEST cargo test --test coldpack_gc preview_is_non_mutating_and_apply_quarantines_only_orphans || status=1
run_gate FORGECLEAN_COLDPACK_GC_PURGE_TEST cargo test --test coldpack_gc apply_purges_only_expired_still_unreferenced_quarantine || status=1
run_gate FORGECLEAN_COLDPACK_GC_TEST cargo test --test coldpack_gc || status=1
run_gate FORGECLEAN_AUTO_CLEAN_MODE_TEST cargo test --test storage empty_inventory_is_clean_only || status=1
run_gate FORGECLEAN_AUTO_OFFLOAD_MODE_TEST cargo test --test storage usb_partition_inherits_external_parent || status=1
run_gate FORGECLEAN_STORAGE_TEST cargo test --test storage || status=1
run_gate FORGECLEAN_OFFLOAD_TEST cargo test --test offload || status=1
run_gate FORGECLEAN_OFFLOAD_E2E cargo test --test offload verified_offload_copies_then_removes_source || status=1
run_gate FORGECLEAN_OFFLOAD_SYMLINK_GUARD cargo test --test offload destination_component_symlink_is_rejected || status=1
run_gate FORGECLEAN_SIGNATURE_SIDECAR_TEST cargo test --test scan_manifest superseded_package_signature_is_batched_with_archive || status=1
run_gate FORGECLEAN_INSTALLED_SIGNATURE_PIN_TEST cargo test --test scan_manifest installed_version_pin_keeps_archive_and_signature_pair || status=1
run_gate FORGECLEAN_CLIPPY cargo clippy --all-targets --all-features -- -D warnings || status=1
run_gate FORGECLEAN_TEST cargo test --all-targets --all-features || status=1
run_gate FORGECLEAN_BUILD cargo build --release || status=1

if [[ $status -eq 0 && -x target/release/forgeclean && -x target/release/forgeclean-gui ]]; then
  cp -f target/release/forgeclean "$BIN_OUT"
  cp -f target/release/forgeclean-gui "$GUI_OUT"
  chmod +x "$BIN_OUT" "$GUI_OUT"
  if "$BIN_OUT" --version >>"$VERIFY" 2>&1; then
    log "FORGECLEAN_CLI_SMOKE=PASS"
  else
    log "FORGECLEAN_CLI_SMOKE=FAIL"
    status=1
  fi
  if "$GUI_OUT" --self-test >>"$VERIFY" 2>&1; then
    log "FORGECLEAN_GUI_SELF_TEST=PASS"
  else
    log "FORGECLEAN_GUI_SELF_TEST=FAIL"
    status=1
  fi

  STORAGE_OUT="$(mktemp "${OUT_DIR}/forgeclean-storage-status.XXXXXX")"
  if "$BIN_OUT" storage-status >"$STORAGE_OUT" 2>&1 \
      && grep -Fxq 'FORGECLEAN_STORAGE_SCAN=PASS' "$STORAGE_OUT" \
      && grep -Eq '^FORGECLEAN_MODE=AUTO_(CLEAN|OFFLOAD)$' "$STORAGE_OUT"; then
    grep -E '^(FORGECLEAN_STORAGE_SCAN|EXTERNAL_DRIVES|FORGECLEAN_MODE|EXTERNAL_STORAGE|EXTERNAL_MOUNT|EXTERNAL_DEVICE|EXTERNAL_UUID|EXTERNAL_TRANSPORT)=' "$STORAGE_OUT" >>"$VERIFY"
    log "FORGECLEAN_STORAGE_STATUS_E2E=PASS"
  else
    cat "$STORAGE_OUT" >>"$VERIFY" 2>/dev/null || true
    log "FORGECLEAN_STORAGE_STATUS_E2E=FAIL"
    status=1
  fi
  rm -f -- "$STORAGE_OUT"

  AUTO_GUARD_OUT="$(mktemp "${OUT_DIR}/forgeclean-auto-guard.XXXXXX")"
  if "$BIN_OUT" auto >"$AUTO_GUARD_OUT" 2>&1; then
    cat "$AUTO_GUARD_OUT" >>"$VERIFY"
    log "FORGECLEAN_AUTO_CONFIRMATION_GUARD=FAIL:ACCEPTED_WITHOUT_YES"
    status=1
  elif grep -q 'auto requires explicit --yes' "$AUTO_GUARD_OUT"; then
    cat "$AUTO_GUARD_OUT" >>"$VERIFY"
    log "FORGECLEAN_AUTO_CONFIRMATION_GUARD=PASS"
  else
    cat "$AUTO_GUARD_OUT" >>"$VERIFY"
    log "FORGECLEAN_AUTO_CONFIRMATION_GUARD=FAIL:WRONG_ERROR"
    status=1
  fi
  rm -f -- "$AUTO_GUARD_OUT"

  SYSTEM_HOME="$(mktemp -d "${OUT_DIR}/forgeclean-system-home.XXXXXX")"
  SYSTEM_OUT="${SYSTEM_HOME}/scan-system.out"
  SYSTEM_MANIFEST="${SYSTEM_HOME}/Downloads/ForgeClean-v1.0.1-PACMAN-BATCH.txt"
  mkdir -p "${SYSTEM_HOME}/Downloads"
  if HOME="$SYSTEM_HOME" "$BIN_OUT" scan-system --keep 2 --partial-age-days 7 >"$SYSTEM_OUT" 2>&1 \
      && [[ -f "$SYSTEM_MANIFEST" ]] \
      && grep -Fxq 'SYSTEM_SCAN=1' "$SYSTEM_OUT" \
      && grep -Fxq 'PURGE_NOT_EXECUTED=1' "$SYSTEM_OUT"; then
    grep -E '^(FORGECLEAN_SCAN|SYSTEM_SCAN|CACHE_ROOT|KEEP_VERSIONS|PARTIAL_AGE_DAYS|INSTALLED_VERSION_PINS|CANDIDATE_FILES|RECOVERABLE_BYTES|MANIFEST|PURGE_NOT_EXECUTED)=' "$SYSTEM_OUT" >>"$VERIFY"
    log "FORGECLEAN_SYSTEM_SCAN_E2E=PASS"
  else
    cat "$SYSTEM_OUT" >>"$VERIFY" 2>/dev/null || true
    log "FORGECLEAN_SYSTEM_SCAN_E2E=FAIL"
    status=1
  fi

  SYSTEM_GUARD_OUT="${SYSTEM_HOME}/scan-system-guard.out"
  if HOME="$SYSTEM_HOME" "$BIN_OUT" scan-system --cache-dir /tmp >"$SYSTEM_GUARD_OUT" 2>&1; then
    cat "$SYSTEM_GUARD_OUT" >>"$VERIFY"
    log "FORGECLEAN_SYSTEM_SCAN_OVERRIDE_GUARD=FAIL:ACCEPTED_CACHE_OVERRIDE"
    status=1
  elif grep -q 'unknown scan-system option: --cache-dir' "$SYSTEM_GUARD_OUT"; then
    cat "$SYSTEM_GUARD_OUT" >>"$VERIFY"
    log "FORGECLEAN_SYSTEM_SCAN_OVERRIDE_GUARD=PASS"
  else
    cat "$SYSTEM_GUARD_OUT" >>"$VERIFY"
    log "FORGECLEAN_SYSTEM_SCAN_OVERRIDE_GUARD=FAIL:WRONG_ERROR"
    status=1
  fi
  rm -rf -- "$SYSTEM_HOME"

  ORGANIZER_E2E="$(mktemp -d "${OUT_DIR}/forgeclean-organizer-e2e.XXXXXX")"
  ORG_DOWNLOADS="${ORGANIZER_E2E}/Downloads"
  mkdir -p "${ORG_DOWNLOADS}/Demo-v1.0.0/src"
  printf '[package]\nname = "demo"\nversion = "1.0.0"\nedition = "2024"\n' > "${ORG_DOWNLOADS}/Demo-v1.0.0/Cargo.toml"
  printf 'fn main() {}\n' > "${ORG_DOWNLOADS}/Demo-v1.0.0/src/main.rs"
  printf 'persistent organizer note\n' > "${ORG_DOWNLOADS}/notes.txt"
  ORG_OUT="${ORGANIZER_E2E}/organize.out"
  if "$BIN_OUT" organize-once --downloads "$ORG_DOWNLOADS" --stable-seconds 0 >"$ORG_OUT" 2>&1 \
      && grep -Fxq 'FORGECLEAN_ORGANIZER=PASS' "$ORG_OUT" \
      && [[ -d "${ORG_DOWNLOADS}/ForgeClean/Projects/Demo/Active" ]] \
      && [[ -L "${ORG_DOWNLOADS}/Demo-v1.0.0" ]] \
      && [[ -f "${ORG_DOWNLOADS}/ForgeClean/ColdStorage/Documents/notes.txt.fcoldpack" ]] \
      && [[ -f "${ORG_DOWNLOADS}/ForgeClean/ColdStorage/Documents/notes.txt.fcoldpack.sha256" ]] \
      && find "${ORG_DOWNLOADS}/ForgeClean/ColdStorage/.coldpack-store/objects" -type f -name "*.zst" -print -quit | grep -q .; then
    log "FORGECLEAN_ORGANIZER_E2E=PASS"
  else
    cat "$ORG_OUT" >>"$VERIFY" 2>/dev/null || true
    log "FORGECLEAN_ORGANIZER_E2E=FAIL"
    status=1
  fi

  # v0.5.2 build-aware migration: old misplaced artifacts are reconciled, and
  # later-arriving artifacts are reconciled on the next organizer cycle.
  mkdir -p "${ORG_DOWNLOADS}/ForgeClean/Archives" \
           "${ORG_DOWNLOADS}/ForgeClean/Documents" \
           "${ORG_DOWNLOADS}/ForgeClean/Installers" \
           "${ORG_DOWNLOADS}/ForgeClean/Projects/ForgeHX/Releases"
  printf 'source\n' > "${ORG_DOWNLOADS}/ForgeClean/Archives/ForgeHX-10.0.30-SOURCE.zip"
  printf 'verify\n' > "${ORG_DOWNLOADS}/ForgeClean/Documents/ForgeHX-10.0.30-VERIFY.txt"
  printf '#!/bin/sh\n' > "${ORG_DOWNLOADS}/ForgeClean/Installers/ForgeHX-10.0.30-HIT-IT.sh"
  RECONCILE_OLD_OUT="${ORGANIZER_E2E}/reconcile-old.out"
  if "$BIN_OUT" organize-once --downloads "$ORG_DOWNLOADS" --stable-seconds 0 >"$RECONCILE_OLD_OUT" 2>&1 \
      && [[ -f "${ORG_DOWNLOADS}/ForgeClean/Projects/ForgeHX/Builds/10.0.30/ForgeHX-10.0.30-SOURCE.zip" ]] \
      && [[ -f "${ORG_DOWNLOADS}/ForgeClean/Projects/ForgeHX/Builds/10.0.30/ForgeHX-10.0.30-VERIFY.txt" ]] \
      && [[ -f "${ORG_DOWNLOADS}/ForgeClean/Projects/ForgeHX/Builds/10.0.30/ForgeHX-10.0.30-HIT-IT.sh" ]] \
      && [[ -L "${ORG_DOWNLOADS}/ForgeClean/Archives/ForgeHX-10.0.30-SOURCE.zip" ]] \
      && [[ -L "${ORG_DOWNLOADS}/ForgeClean/Documents/ForgeHX-10.0.30-VERIFY.txt" ]] \
      && [[ -L "${ORG_DOWNLOADS}/ForgeClean/Installers/ForgeHX-10.0.30-HIT-IT.sh" ]]; then
    log "FORGECLEAN_EXISTING_BUILD_RECONCILE_E2E=PASS"
  else
    cat "$RECONCILE_OLD_OUT" >>"$VERIFY" 2>/dev/null || true
    log "FORGECLEAN_EXISTING_BUILD_RECONCILE_E2E=FAIL"
    status=1
  fi

  printf 'late-checksum\n' > "${ORG_DOWNLOADS}/ForgeClean/Projects/ForgeHX/Releases/ForgeHX-10.0.30-SHA256SUMS.txt"
  RECONCILE_LATE_OUT="${ORGANIZER_E2E}/reconcile-late.out"
  if "$BIN_OUT" organize-once --downloads "$ORG_DOWNLOADS" --stable-seconds 0 >"$RECONCILE_LATE_OUT" 2>&1 \
      && [[ -f "${ORG_DOWNLOADS}/ForgeClean/Projects/ForgeHX/Builds/10.0.30/ForgeHX-10.0.30-SHA256SUMS.txt" ]] \
      && [[ -L "${ORG_DOWNLOADS}/ForgeClean/Projects/ForgeHX/Releases/ForgeHX-10.0.30-SHA256SUMS.txt" ]]; then
    log "FORGECLEAN_CONTINUOUS_BUILD_RECONCILE_E2E=PASS"
  else
    cat "$RECONCILE_LATE_OUT" >>"$VERIFY" 2>/dev/null || true
    log "FORGECLEAN_CONTINUOUS_BUILD_RECONCILE_E2E=FAIL"
    status=1
  fi

  RESOLVE_OUT="${ORGANIZER_E2E}/resolve.out"
  if "$BIN_OUT" resolve-project Demo --downloads "$ORG_DOWNLOADS" >"$RESOLVE_OUT" 2>&1 \
      && grep -Fxq 'FORGECLEAN_RESOLVE_PROJECT=PASS' "$RESOLVE_OUT" \
      && grep -Fq "ACTIVE_PATH=${ORG_DOWNLOADS}/ForgeClean/Projects/Demo/Active" "$RESOLVE_OUT"; then
    log "FORGECLEAN_BUILD_REGISTRY_E2E=PASS"
  else
    cat "$RESOLVE_OUT" >>"$VERIFY" 2>/dev/null || true
    log "FORGECLEAN_BUILD_REGISTRY_E2E=FAIL"
    status=1
  fi

  BUILD_OUT="${ORGANIZER_E2E}/build.out"
  if "$BIN_OUT" build Demo --downloads "$ORG_DOWNLOADS" -- /usr/bin/pwd >"$BUILD_OUT" 2>&1 \
      && grep -Fxq 'FORGECLEAN_BUILD_REDIRECT=PASS' "$BUILD_OUT" \
      && grep -Fxq "${ORG_DOWNLOADS}/ForgeClean/Projects/Demo/Active" "$BUILD_OUT"; then
    log "FORGECLEAN_BUILD_REDIRECT_E2E=PASS"
  else
    cat "$BUILD_OUT" >>"$VERIFY" 2>/dev/null || true
    log "FORGECLEAN_BUILD_REDIRECT_E2E=FAIL"
    status=1
  fi

  RESTORE_OUT="${ORGANIZER_E2E}/restore.out"
  RESTORE_ROOT="${ORG_DOWNLOADS}/ForgeClean/Restored/Documents"
  if "$BIN_OUT" restore \
      --archive "${ORG_DOWNLOADS}/ForgeClean/ColdStorage/Documents/notes.txt.fcoldpack" \
      --to "$RESTORE_ROOT" \
      --downloads "$ORG_DOWNLOADS" >"$RESTORE_OUT" 2>&1 \
      && grep -Fxq 'FORGECLEAN_RESTORE=PASS' "$RESTORE_OUT" \
      && grep -Fxq 'persistent organizer note' "$RESTORE_ROOT/notes.txt"; then
    log "FORGECLEAN_COLDSTORE_RESTORE_E2E=PASS"
    log "FORGECLEAN_V0_4_0_COLDPACK_DEDUP=PASS"
  else
    cat "$RESTORE_OUT" >>"$VERIFY" 2>/dev/null || true
    log "FORGECLEAN_COLDSTORE_RESTORE_E2E=FAIL"
    status=1
  fi
  GC_SOURCE="${ORG_DOWNLOADS}/gc-orphan.bin"
  printf 'forgeclean-v1.0.1-gc-orphan-%s\n' "$$" > "$GC_SOURCE"
  GC_ARCHIVE_OUT="${ORGANIZER_E2E}/gc-archive.out"
  if "$BIN_OUT" archive --path "$GC_SOURCE" --downloads "$ORG_DOWNLOADS" >"$GC_ARCHIVE_OUT" 2>&1; then
    GC_MANIFEST="$(grep '^ARCHIVE=' "$GC_ARCHIVE_OUT" | head -n1 | cut -d= -f2-)"
  else
    GC_MANIFEST=""
  fi
  if [[ -n "$GC_MANIFEST" && -f "$GC_MANIFEST" && -f "${GC_MANIFEST}.sha256" ]]; then
    rm -f -- "$GC_MANIFEST" "${GC_MANIFEST}.sha256"
    OBJECT_FINGERPRINT_BEFORE="$(find "${ORG_DOWNLOADS}/ForgeClean/ColdStorage/.coldpack-store/objects" -type f -name '*.zst' -print0 | sort -z | xargs -0 sha256sum | sha256sum | awk '{print $1}')"
    GC_STATUS_OUT="${ORGANIZER_E2E}/gc-status.out"
    if "$BIN_OUT" coldpack-status --downloads "$ORG_DOWNLOADS" >"$GC_STATUS_OUT" 2>&1 \
        && grep -Fxq 'FORGECLEAN_COLDPACK_STATUS=PASS' "$GC_STATUS_OUT" \
        && grep -Eq '^ORPHAN_OBJECTS=[1-9][0-9]*$' "$GC_STATUS_OUT"; then
      log "FORGECLEAN_COLDPACK_STATUS_E2E=PASS"
    else
      cat "$GC_STATUS_OUT" >>"$VERIFY" 2>/dev/null || true
      log "FORGECLEAN_COLDPACK_STATUS_E2E=FAIL"
      status=1
    fi

    GC_PREVIEW_OUT="${ORGANIZER_E2E}/gc-preview.out"
    if "$BIN_OUT" coldpack-gc --preview --downloads "$ORG_DOWNLOADS" >"$GC_PREVIEW_OUT" 2>&1 \
        && grep -Fxq 'FORGECLEAN_COLDPACK_GC_PREVIEW=PASS' "$GC_PREVIEW_OUT" \
        && grep -Eq '^ORPHAN_OBJECTS=[1-9][0-9]*$' "$GC_PREVIEW_OUT"; then
      OBJECT_FINGERPRINT_AFTER="$(find "${ORG_DOWNLOADS}/ForgeClean/ColdStorage/.coldpack-store/objects" -type f -name '*.zst' -print0 | sort -z | xargs -0 sha256sum | sha256sum | awk '{print $1}')"
      if [[ "$OBJECT_FINGERPRINT_BEFORE" == "$OBJECT_FINGERPRINT_AFTER" ]]; then
        log "FORGECLEAN_COLDPACK_GC_PREVIEW_E2E=PASS"
      else
        log "FORGECLEAN_COLDPACK_GC_PREVIEW_E2E=FAIL:MUTATED"
        status=1
      fi
    else
      cat "$GC_PREVIEW_OUT" >>"$VERIFY" 2>/dev/null || true
      log "FORGECLEAN_COLDPACK_GC_PREVIEW_E2E=FAIL"
      status=1
    fi

    GC_APPLY_OUT="${ORGANIZER_E2E}/gc-apply.out"
    if "$BIN_OUT" coldpack-gc --apply --downloads "$ORG_DOWNLOADS" >"$GC_APPLY_OUT" 2>&1 \
        && grep -Fxq 'FORGECLEAN_COLDPACK_GC=PASS' "$GC_APPLY_OUT" \
        && grep -Eq '^QUARANTINED_OBJECTS=[1-9][0-9]*$' "$GC_APPLY_OUT" \
        && find "${ORG_DOWNLOADS}/ForgeClean/ColdStorage/.coldpack-store/quarantine" -type f -name '*.zst' -print -quit | grep -q .; then
      log "FORGECLEAN_COLDPACK_GC_APPLY_E2E=PASS"
    else
      cat "$GC_APPLY_OUT" >>"$VERIFY" 2>/dev/null || true
      log "FORGECLEAN_COLDPACK_GC_APPLY_E2E=FAIL"
      status=1
    fi

    GC_AUDIT_OUT="${ORGANIZER_E2E}/gc-audit.out"
    if "$BIN_OUT" coldpack-audit --downloads "$ORG_DOWNLOADS" >"$GC_AUDIT_OUT" 2>&1 \
        && grep -Fxq 'FORGECLEAN_COLDPACK_AUDIT=PASS' "$GC_AUDIT_OUT"; then
      log "FORGECLEAN_COLDPACK_GC_AUDIT_E2E=PASS"
    else
      cat "$GC_AUDIT_OUT" >>"$VERIFY" 2>/dev/null || true
      log "FORGECLEAN_COLDPACK_GC_AUDIT_E2E=FAIL"
      status=1
    fi
  else
    cat "$GC_ARCHIVE_OUT" >>"$VERIFY" 2>/dev/null || true
    log "FORGECLEAN_COLDPACK_GC_SETUP_E2E=FAIL"
    status=1
  fi
  rm -rf -- "$ORGANIZER_E2E"

  E2E_DIR="$(mktemp -d "${OUT_DIR}/forgeclean-e2e.XXXXXX")"
  E2E_MANIFEST="${E2E_DIR}/batch.txt"
  cleanup_e2e() { rm -rf -- "$E2E_DIR"; }
  trap cleanup_e2e EXIT
  printf 'old\n' > "${E2E_DIR}/forgeclean-testpkg-1-1-x86_64.pkg.tar.zst"
  printf 'old-sig\n' > "${E2E_DIR}/forgeclean-testpkg-1-1-x86_64.pkg.tar.zst.sig"
  printf 'new\n' > "${E2E_DIR}/forgeclean-testpkg-2-1-x86_64.pkg.tar.zst"
  printf 'new-sig\n' > "${E2E_DIR}/forgeclean-testpkg-2-1-x86_64.pkg.tar.zst.sig"
  if "$BIN_OUT" scan --cache-dir "$E2E_DIR" --keep 1 --partial-age-days 7 --manifest "$E2E_MANIFEST" >>"$VERIFY" 2>&1 \
      && "$BIN_OUT" verify --manifest "$E2E_MANIFEST" >>"$VERIFY" 2>&1 \
      && "$BIN_OUT" purge --manifest "$E2E_MANIFEST" --yes >>"$VERIFY" 2>&1 \
      && [[ ! -e "${E2E_DIR}/forgeclean-testpkg-1-1-x86_64.pkg.tar.zst" ]] \
      && [[ ! -e "${E2E_DIR}/forgeclean-testpkg-1-1-x86_64.pkg.tar.zst.sig" ]] \
      && [[ -e "${E2E_DIR}/forgeclean-testpkg-2-1-x86_64.pkg.tar.zst" ]] \
      && [[ -e "${E2E_DIR}/forgeclean-testpkg-2-1-x86_64.pkg.tar.zst.sig" ]]; then
    log "FORGECLEAN_AUTO_CLEAN_E2E=PASS"
    log "FORGECLEAN_E2E_PURGE=PASS"
  else
    log "FORGECLEAN_AUTO_CLEAN_E2E=FAIL"
    log "FORGECLEAN_E2E_PURGE=FAIL"
    status=1
  fi
  cleanup_e2e
  trap - EXIT
else
  log "FORGECLEAN_BINARY=FAIL:NOT_BUILT"
  status=1
fi

if [[ -f "$BIN_OUT" && -f "$GUI_OUT" ]]; then
  sha256sum "$BIN_OUT" "$GUI_OUT" > "$SUMS"
  log "FORGECLEAN_SHA256=PASS"
fi

if [[ $status -eq 0 ]]; then
  log "FORGECLEAN_VERIFY=PASS"
else
  log "FORGECLEAN_VERIFY=FAIL"
fi
log "VERIFY_FILE=${VERIFY}"
log "SHA256_FILE=${SUMS}"
log "BINARY_FILE=${BIN_OUT}"
log "GUI_BINARY_FILE=${GUI_OUT}"
exit "$status"
