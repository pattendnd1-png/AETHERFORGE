#!/usr/bin/env bash
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace --release

file target/release/opensanctuary-launcher target/release/opensanctuary-engine
for binary in target/release/opensanctuary-launcher target/release/opensanctuary-engine; do
  file "$binary" | grep -q 'ELF'
done

# Compatibility-runner references are permitted only in the isolated Battle.net bridge.
# Native engine/render/assets and the rest of the workspace must remain independent from
# Wine/Proton/DXVK/VKD3D implementation details.
if grep -Eir \
  --exclude-dir='sanctuary-battlenet' \
  --include='Cargo.toml' --include='*.rs' \
  '(^|[^[:alpha:]])(wine|proton|dxvk|vkd3d|winetricks|bottles|lutris)([^[:alpha:]]|$)' \
  crates apps Cargo.toml; then
  echo 'Compatibility-layer reference escaped the isolated Battle.net bridge.' >&2
  exit 1
fi

if ! grep -q 'name = "sanctuary-battlenet"' crates/sanctuary-battlenet/Cargo.toml; then
  echo 'Battle.net bridge crate is missing.' >&2
  exit 1
fi

if ! grep -q 'https://download.battle.net/en-us/desktop' crates/sanctuary-battlenet/src/lib.rs; then
  echo 'Official Battle.net download handoff is missing.' >&2
  exit 1
fi

for symbol in \
  'BridgeRecord' \
  'default_bridge_record_path' \
  'save_bridge_record' \
  'clear_bridge_record' \
  'RetryPolicy'; do
  if ! grep -q "$symbol" crates/sanctuary-battlenet/src/lib.rs; then
    echo "Battle.net bridge persistence/retry symbol is missing: $symbol" >&2
    exit 1
  fi
done

for symbol in \
  'BridgeHealthState' \
  'evaluate_bridge_health' \
  'recover_bridge' \
  'quarantine_bridge_record' \
  'FailureTracker'; do
  if ! grep -Rq "$symbol" crates/sanctuary-battlenet/src; then
    echo "Battle.net bridge health/self-healing symbol is missing: $symbol" >&2
    exit 1
  fi
done

for symbol in \
  'SessionSupervisor' \
  'snapshot_processes' \
  'build_fingerprint' \
  'SupervisorState' \
  'update_settled'; do
  if ! grep -q "$symbol" crates/sanctuary-battlenet/src/supervisor.rs; then
    echo "Battle.net session-supervisor symbol is missing: $symbol" >&2
    exit 1
  fi
done

for symbol in 'SessionObserved' 'UpdateSettled'; do
  if ! grep -q "$symbol" crates/sanctuary-launcher/src/lib.rs; then
    echo "Launcher supervisor event is missing: $symbol" >&2
    exit 1
  fi
done

if ! grep -q 'Path::new("/proc")' apps/launcher/src/main.rs; then
  echo 'Launcher /proc session polling is missing.' >&2
  exit 1
fi

if ! grep -Rq 'REPAIR SOFTWARE BRIDGE' apps/launcher/src; then
  echo 'Repair Software Bridge UI action is missing.' >&2
  exit 1
fi

# Explanatory security text is allowed; actual secret-shaped fields/assignments are not.
if grep -Eir \
  --include='*.rs' \
  '((pub[[:space:]]+)?(password|passwd|client_secret|access_token|refresh_token|authenticator_secret|recovery_code)[[:space:]]*:|[[:space:]](password|passwd|client_secret|access_token|refresh_token)[[:space:]]*=)' \
  crates/sanctuary-battlenet/src apps/launcher/src; then
  echo 'Secret-shaped Battle.net credential storage entered production source.' >&2
  exit 1
fi

if grep -Rq '.password(true)' apps/launcher/src; then
  echo 'Password-capture widget entered the launcher UI.' >&2
  exit 1
fi


for source in apps/launcher/src/theme.rs apps/launcher/src/widgets.rs apps/launcher/src/ui_motion.rs apps/launcher/src/account_page.rs apps/launcher/src/battlenet_page.rs apps/launcher/src/chrome.rs apps/launcher/src/game_page.rs apps/launcher/src/downloads.rs apps/launcher/src/notifications.rs apps/launcher/src/diagnostics.rs; do
  if [[ ! -f "$source" ]]; then
    echo "Launcher presentation module is missing: $source" >&2
    exit 1
  fi
done

for module_decl in 'mod account_page;' 'mod battlenet_page;' 'mod chrome;' 'mod diagnostics;' 'mod downloads;' 'mod game_page;' 'mod notifications;' 'mod theme;' 'mod widgets;' 'mod ui_motion;'; do
  if ! grep -q "$module_decl" apps/launcher/src/main.rs; then
    echo "Launcher module declaration is missing: $module_decl" >&2
    exit 1
  fi
done

for source in \
  crates/sanctuary-battlenet/src/account.rs \
  crates/sanctuary-battlenet/src/window_host/mod.rs \
  crates/sanctuary-battlenet/src/window_host/x11.rs \
  crates/sanctuary-battlenet/src/window_host/wayland.rs; do
  if [[ ! -f "$source" ]]; then
    echo "Integrated Battle.net bridge module is missing: $source" >&2
    exit 1
  fi
done

for symbol in \
  'AccountProfile' \
  'DesktopSessionState' \
  'BattleNetWindowHost' \
  'WindowHostMode' \
  'X11Embedded' \
  'WaylandCompanion' \
  'window_matches_battlenet'; do
  if ! grep -Rq "$symbol" crates/sanctuary-battlenet/src apps/launcher/src; then
    echo "Integrated Battle.net symbol is missing: $symbol" >&2
    exit 1
  fi
done

if ! grep -q 'x11rb' crates/sanctuary-battlenet/Cargo.toml; then
  echo 'x11rb dependency for X11 Battle.net hosting is missing.' >&2
  exit 1
fi

if ! grep -q 'Page::BattleNet' apps/launcher/src/main.rs || ! grep -q 'Page::Account' apps/launcher/src/main.rs; then
  echo 'Integrated Battle.net/Account launcher routes are missing.' >&2
  exit 1
fi

if (( $(wc -l < apps/launcher/src/main.rs) >= 2800 )); then
  echo 'Launcher main.rs regressed toward the pre-v0.3.7 presentation monolith.' >&2
  exit 1
fi

# Player-facing game/Battle.net modules must keep low-level bridge implementation details behind diagnostics.
if sed '/#\[cfg(test)\]/,$d' apps/launcher/src/game_page.rs | grep -Eiq '\b(RUNNER|PREFIX|WINE PREFIX)\b'; then
  echo 'Advanced bridge implementation detail leaked into the primary game page.' >&2
  exit 1
fi

# Retired pre-v0.3.12 presentation helpers must stay deleted. They were superseded by
# the focused chrome/game/downloads/diagnostics modules and trigger -D dead-code.
for stale_symbol in \
  'fn build_strip(' \
  'fn storage_cards('; do
  if grep -Fq "$stale_symbol" apps/launcher/src/main.rs; then
    echo "Retired launcher method returned: $stale_symbol" >&2
    exit 1
  fi
done

for stale_symbol in \
  'fn nav_button(' \
  'fn launcher_menu_button(' \
  'fn game_meta_chip(' \
  'fn game_status_line(' \
  'fn game_link(' \
  'fn paint_diablo_mark(' \
  'fn build_field(' \
  'fn metric_card(' \
  'Muted,' \
  'fn bridge_health_tone(' \
  'fn inventory_state_tone(' \
  'fn bridge_status_row(' \
  'fn lifecycle_banner('; do
  if grep -Fq "$stale_symbol" apps/launcher/src/widgets.rs; then
    echo "Retired launcher widget returned: $stale_symbol" >&2
    exit 1
  fi
done

if grep -Fq 'fn label(&self)' apps/launcher/src/chrome.rs; then
  echo 'Retired TopBarAction::label method returned.' >&2
  exit 1
fi

for required_symbol in \
  'render_top_bar' \
  'render_game_hero' \
  'render_downloads_tray' \
  'render_notifications' \
  'render_advanced_diagnostics'; do
  if ! grep -Rq "$required_symbol" apps/launcher/src; then
    echo "v0.3.12 UI symbol is missing: $required_symbol" >&2
    exit 1
  fi
done


# v0.3.12 software-bridge repair must work before Diablo III exists.
for repair_test in \
  'client_only_bridge_record_round_trips_and_validates' \
  'repair_succeeds_for_usable_client_without_diablo_install' \
  'recovery_completion_preserves_degraded_bridge_health'; do
  if ! grep -Rq "$repair_test" crates; then
    echo "Software-bridge repair regression test is missing: $repair_test" >&2
    exit 1
  fi
done

if ! grep -q 'pub diablo_install: Option<PathBuf>' crates/sanctuary-battlenet/src/lib.rs \
  || ! grep -q 'pub diablo_exe: Option<PathBuf>' crates/sanctuary-battlenet/src/lib.rs; then
  echo 'BridgeRecord regressed to requiring Diablo III before the Battle.net client bridge can persist.' >&2
  exit 1
fi

if ! grep -q 'self.battle_net_window_host = BattleNetWindowHost::detect()' apps/launcher/src/main.rs \
  || ! grep -q 'self.session_supervisor = SessionSupervisor::default()' apps/launcher/src/main.rs \
  || ! grep -q 'self.bridge_failures.record_success()' apps/launcher/src/main.rs; then
  echo 'In-app bridge repair no longer resets all bridge supervisors/cooldown state.' >&2
  exit 1
fi

# Keep the X11 host behind indirection so strict Clippy does not flag HostBackend.
if grep -Fq 'X11(x11::X11Host)' crates/sanctuary-battlenet/src/window_host/mod.rs; then
  echo "ERROR: HostBackend::X11 must remain boxed (clippy::large-enum-variant)." >&2
  exit 1
fi


# v0.3.12: successful recovery must not map a stale/unvalidated Broken health snapshot to Error.
if ! grep -Fq 'else if self.install_state == InstallState::Ready {' crates/sanctuary-launcher/src/lib.rs; then
  echo 'v0.3.12 recovery-completion fallback is missing' >&2
  exit 1
fi


# v0.3.12 network bridge must independently validate Battle.net connectivity.
if [[ ! -f crates/sanctuary-battlenet/src/network.rs ]]; then
  echo 'Battle.net network bridge module is missing.' >&2
  exit 1
fi
for symbol in \
  'NetworkBridgeState' \
  'NetworkBridgeReport' \
  'probe_network_bridge' \
  'NetworkCheckKind'; do
  if ! grep -Rq "$symbol" crates/sanctuary-battlenet/src crates/sanctuary-launcher/src apps/launcher/src; then
    echo "Network bridge symbol is missing: $symbol" >&2
    exit 1
  fi
done
for host in 'account.battle.net' 'download.battle.net'; do
  if ! grep -Fq "$host" crates/sanctuary-battlenet/src/network.rs; then
    echo "Official Battle.net network probe host is missing: $host" >&2
    exit 1
  fi
done
if ! grep -Fq 'REPAIR NETWORK BRIDGE' apps/launcher/src/battlenet_page.rs; then
  echo 'Repair Network Bridge UI action is missing.' >&2
  exit 1
fi
if ! grep -Fq 'thread::spawn' apps/launcher/src/main.rs \
  || ! grep -Fq 'NetworkBridgeChecked' apps/launcher/src/main.rs; then
  echo 'Network bridge probe is not asynchronous through the launcher event bus.' >&2
  exit 1
fi
if ! grep -Fq 'rustls.workspace = true' crates/sanctuary-battlenet/Cargo.toml \
  || ! grep -Fq 'webpki-roots.workspace = true' crates/sanctuary-battlenet/Cargo.toml; then
  echo 'Certificate-validated TLS dependencies are missing from the network bridge.' >&2
  exit 1
fi
if [[ ! -x INSTALL-ACTIVATE-ON-ARCH.sh ]]; then
  echo 'Arch install/activation script is missing or not executable.' >&2
  exit 1
fi



# v0.3.12 seamless Battle.net flow must keep one preparation policy path.
for symbol in \
  'SessionIntent' \
  'PreparationContext' \
  'PreparationDecision' \
  'prepare_session'; do
  if ! grep -Fq "$symbol" crates/sanctuary-launcher/src/lib.rs; then
    echo "Seamless session-preparation symbol is missing: $symbol" >&2
    exit 1
  fi
done
for symbol in \
  'fn preparation_context(' \
  'fn prepare_for(' \
  'fn execute_preparation(' \
  'fn launch_ready_game(' \
  'fn open_battlenet_client(' \
  'fn show_battlenet_sign_in('; do
  if ! grep -Fq "$symbol" apps/launcher/src/main.rs; then
    echo "Seamless launcher orchestration symbol is missing: $symbol" >&2
    exit 1
  fi
done
if grep -Fq 'PrimaryAction::Install => self.start_official_install()' apps/launcher/src/main.rs; then
  echo 'INSTALL bypassed the session-preparation pipeline.' >&2
  exit 1
fi
if ! grep -Fq 'Starting Battle.net in the background and launching Diablo III' apps/launcher/src/main.rs; then
  echo 'Background Battle.net launch status is missing.' >&2
  exit 1
fi
if ! grep -Fq 'session_status' apps/launcher/src/game_page.rs \
  || ! grep -Fq 'session_status' apps/launcher/src/main.rs; then
  echo 'Player-facing seamless session status is missing.' >&2
  exit 1
fi
if sed -n '/fn start_official_game_launch/,/fn primary_action_label/p' apps/launcher/src/main.rs | grep -Fq 'self.page = Page::BattleNet'; then
  echo 'Direct official PLAY still forces the Battle.net page.' >&2
  exit 1
fi
for test_name in \
  'preparation_repairs_offline_network_first' \
  'preparation_surfaces_blizzard_interaction_for_sign_in' \
  'preparation_waits_for_active_update' \
  'preparation_launches_ready_game' \
  'direct_play_does_not_require_battlenet_surface'; do
  if ! grep -Rq "$test_name" crates/sanctuary-launcher/src apps/launcher/src; then
    echo "Seamless-flow regression test is missing: $test_name" >&2
    exit 1
  fi
done


# v0.4.2 native CASC read foundation.
for module in build.rs key.rs index.rs archive.rs blte.rs storage.rs; do
  if [[ ! -f "crates/sanctuary-casc/src/${module}" ]]; then
    echo "v0.4.2 CASC module is missing: ${module}" >&2
    exit 1
  fi
done

for symbol in \
  'EncodingKey' \
  'EncodingKeyPrefix' \
  'BuildConfig' \
  'LocalIndex' \
  'ArchiveReader' \
  'BlteDecodeOptions' \
  'decode_blte' \
  'CascStorage'; do
  if ! grep -Rq "$symbol" crates/sanctuary-casc/src; then
    echo "v0.4.2 CASC symbol is missing: $symbol" >&2
    exit 1
  fi
done

for dep in 'flate2.workspace = true' 'md-5.workspace = true'; do
  if ! grep -Fq "$dep" crates/sanctuary-casc/Cargo.toml; then
    echo "v0.4.2 CASC dependency is missing: $dep" >&2
    exit 1
  fi
done

# Transport modules are read-only. Ignore #[cfg(test)] fixtures, which create synthetic files.
for module in build.rs key.rs index.rs archive.rs blte.rs storage.rs; do
  production="$(sed '/^#\[cfg(test)\]/,$d' "crates/sanctuary-casc/src/${module}")"
  if grep -Eq '(fs::write|File::create|OpenOptions|remove_file|remove_dir|rename\()' <<<"$production"; then
    echo "v0.4.2 CASC transport writes to storage in ${module}" >&2
    exit 1
  fi
done

for mode in "b'N'" "b'Z'" "b'F'" "b'E'"; do
  if ! grep -Fq "$mode" crates/sanctuary-casc/src/blte.rs; then
    echo "BLTE mode handling is missing: $mode" >&2
    exit 1
  fi
done
if ! grep -Fq 'verify_chunk_md5' crates/sanctuary-casc/src/blte.rs \
  || ! grep -Fq 'max_output_bytes' crates/sanctuary-casc/src/blte.rs \
  || ! grep -Fq 'max_recursion_depth' crates/sanctuary-casc/src/blte.rs; then
  echo 'BLTE integrity/bounds controls are missing.' >&2
  exit 1
fi
if ! grep -Fq 'encrypted BLTE chunks require a legitimate TACT key source' crates/sanctuary-casc/src/blte.rs; then
  echo 'Encrypted BLTE must remain explicitly unsupported without a legitimate key source.' >&2
  exit 1
fi

for test_name in \
  'encoding_key_round_trips_hex_and_prefix' \
  'newest_file_per_bucket_is_selected_and_duplicate_first_wins' \
  'reads_bounded_archive_record_and_strips_envelope' \
  'decodes_zlib_and_recursive_chunks' \
  'rejects_digest_mismatch_and_encrypted_chunks' \
  'enforces_output_and_recursion_limits' \
  'reads_build_blob_end_to_end_without_modifying_installation'; do
  if ! grep -Rq "$test_name" crates/sanctuary-casc/src; then
    echo "v0.4.2 CASC regression test is missing: $test_name" >&2
    exit 1
  fi
done


# Rust 1.98 Clippy regression: constant-size chunk parsing must use as_chunks.
if grep -RFn --include='*.rs' '.chunks_exact(INDEX_ENTRY_BYTES)' crates/sanctuary-casc/src >/dev/null; then
  echo "ERROR: sanctuary-casc still uses chunks_exact with constant INDEX_ENTRY_BYTES" >&2
  exit 1
fi
echo "CLIPPY_AS_CHUNKS_REGRESSION=PASS"

# Rust 1.98 Clippy regression: optional BLTE size validation must use a let-chain.
if python - <<'PY'
from pathlib import Path
import re, sys
text = Path('crates/sanctuary-casc/src/blte.rs').read_text()
pattern = re.compile(r'if let Some\(expected_size\) = expected_size \{\s*if decoded\.len\(\) != expected_size \{', re.S)
if pattern.search(text):
    sys.exit(1)
PY
then
  :
else
  echo 'ERROR: BLTE expected-size validation regressed to clippy::collapsible_if shape' >&2
  exit 1
fi
echo "CLIPPY_COLLAPSIBLE_IF_REGRESSION=PASS"

echo 'OpenSanctuary v0.4.2 verification passed.'


