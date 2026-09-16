#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
DL="$HOME/Downloads"
STUDIO="$ROOT/apps/opendeck-studio"
NEW_BIN="$ROOT/target/release/opendeck-studio"
VERIFY="$DL/OpenDeck-v2.0.12-RENDER-PERFORMANCE-VERIFY.txt"
ROLLBACK="$DL/OpenDeck-v2.0.12-RENDER-PERFORMANCE-ROLLBACK.txt"
LOGDIR="$DL/OpenDeck-v2.0.12-RENDER-PERFORMANCE-logs"
QDIR="$DL/OpenDeck-v2.0.12-qualification"
VISUAL_JSON="$QDIR/OpenDeck-v2.0.12-VISUAL-METRICS.json"
PERF_JSON="$QDIR/OpenDeck-v2.0.12-PERFORMANCE-METRICS.json"
SCREENSHOT="$DL/OpenDeck-v2.0.12-QUALIFICATION.png"
STAMP="$DL/OpenDeck-v2.0.12-QUALIFIED-STAMP.txt"
STAGED_QUALIFIED="$HOME/.local/lib/.opendeck-v2.0.12-qualified"
EXPECTED_ACTIVE_SHA="78b5ceb386773d0e58aa322f67766780432c276888df9e3f91537bf687727937"
ARCHIVE_SHA="${OPENDECK_V212_ARCHIVE_SHA:?OPENDECK_V212_ARCHIVE_SHA is required}"

mkdir -p "$LOGDIR" "$QDIR" "$HOME/.local/bin" "$HOME/.local/lib"
: > "$VERIFY"
say(){ printf '%s\n' "$*" | tee -a "$VERIFY"; }
fail(){ local stage="$1" rc="${2:-1}"; say "OPENDECK_V2_0_12_RENDER_PERFORMANCE=FAIL:$rc"; say "OPENDECK_FAILURE_STAGE=$stage"; say "VERIFY_FILE=$VERIFY"; exit "$rc"; }
gate(){ local name="$1" log="$2"; shift 2; say "OPENDECK_V212_STAGE=${name}:START"; if "$@" >"$log" 2>&1; then say "OPENDECK_V212_STAGE=${name}:PASS"; else local rc=$?; say "OPENDECK_V212_STAGE=${name}:FAIL:$rc"; tail -260 "$log" | tee -a "$VERIFY"; fail "$name" "$rc"; fi; }

say "OPENDECK_V2_0_12_RENDER_PERFORMANCE=START"
say "OPENDECK_RELEASE=CANONICAL_RENDER_FINAL_TEST_QUALIFIER_CLOSURE"
say "OPENDECK_BASELINE_VERSION=2.0.8"
say "OPENDECK_POLICY=QUALIFY_VISUAL_PERFORMANCE_BEFORE_ACTIVE_BINARY_SWITCH"
say "OPENDECK_CANONICAL_RENDER=1536x1024"
say "OPENDECK_DIAL_STACKS=DEFERRED_TO_2.0.13"
say "OPENDECK_BACKGROUND_SERVICES=NONE"
say "OPENDECK_AUTOSTART=DISABLED"
say "OPENDECK_PHYSICAL_INPUT_SYNTHESIS=DISABLED"
say "OPENDECK_SOURCE_ARCHIVE_SHA256=$ARCHIVE_SHA"
say "OPENDECK_PREVIOUS_FAILED_VERSION=2.0.11"
say "OPENDECK_PREVIOUS_FAILURE_STAGE=FRONTEND_TESTS_DIAL_ACTION_TEST_CONTRACT"
say "OPENDECK_REPAIR=RESTORE_PREVIOUS_PAGE_ACTION_TEST_AND_V212_QUALIFIER_LABELS"

for cmd in node npm cargo rustc cargo-clippy rustfmt python3 sha256sum spectacle ps; do command -v "$cmd" >/dev/null 2>&1 || fail "REQUIRED_COMMAND_MISSING:$cmd" 2; done
RUSTC_VERSION="$(rustc --version)"; CARGO_VERSION="$(cargo --version)"; CLIPPY_VERSION="$(cargo clippy --version)"
say "OPENDECK_RUSTC=$RUSTC_VERSION"; say "OPENDECK_CARGO=$CARGO_VERSION"; say "OPENDECK_CLIPPY=$CLIPPY_VERSION"
[[ "$RUSTC_VERSION" == rustc\ 1.98.1* ]] || fail "RUSTC_1_98_1_REQUIRED" 2
[[ "$CARGO_VERSION" == cargo\ 1.98.1* ]] || fail "CARGO_1_98_1_REQUIRED" 2

[[ -x "$HOME/.local/bin/opendeck-studio" ]] || fail "ACTIVE_V208_BASELINE_MISSING" 2
BASELINE_TARGET="$(readlink -f "$HOME/.local/bin/opendeck-studio" 2>/dev/null || true)"
BASELINE_SHA="$(sha256sum "$HOME/.local/bin/opendeck-studio" | awk '{print $1}')"
say "OPENDECK_ACTIVE_BASELINE_TARGET=$BASELINE_TARGET"; say "OPENDECK_ACTIVE_BASELINE_BINARY_SHA256=$BASELINE_SHA"
[[ "$BASELINE_SHA" == "$EXPECTED_ACTIVE_SHA" ]] || fail "ACTIVE_V208_BASELINE_SHA_MISMATCH" 2

# Static/source regression gates.
gate "SOURCE_CONTRACT" "$LOGDIR/source-contract.log" bash -lc "cd '$ROOT' && python3 scripts/check-v212-clean-baseline.py"
gate "VERSION_DEMO_CONTRACT" "$LOGDIR/version-demo.log" bash -lc "cd '$ROOT' && python3 scripts/check-v212-version-and-demo.py"
gate "SHELL_CONTRACT" "$LOGDIR/shell.log" bash -lc "cd '$ROOT' && python3 scripts/check-v212-shell-contract.py"
gate "RENDER_CONTRACT" "$LOGDIR/render-contract.log" bash -lc "cd '$ROOT' && python3 scripts/check-v212-render-contract.py"
gate "PERFORMANCE_CONTRACT" "$LOGDIR/perf-contract.log" bash -lc "cd '$ROOT' && python3 scripts/check-v212-performance-contract.py"
gate "TEST_CONTRACT" "$LOGDIR/test-contract.log" bash -lc "cd '$ROOT' && python3 scripts/check-v212-test-contract.py"
gate "ACCESSIBILITY_NAME_CONTRACT" "$LOGDIR/accessibility-name.log" bash -lc "cd '$ROOT' && python3 scripts/check-v212-accessibility-name-contract.py"
gate "DIALS_FILTER_CONTRACT" "$LOGDIR/dials-filter.log" bash -lc "cd '$ROOT' && python3 scripts/check-v207-dials-filter.py"
gate "ENCODER_PRESS_CONTRACT" "$LOGDIR/encoder.log" bash -lc "cd '$ROOT' && python3 scripts/check-v207-encoder-press.py"
gate "INTERACTION_ASSIGNMENT_CONTRACT" "$LOGDIR/assignment.log" bash -lc "cd '$ROOT' && python3 scripts/check-v207-interaction-assignment.py"
gate "TWITCH_AUTH_POLLING_CONTRACT" "$LOGDIR/twitch.log" bash -lc "cd '$ROOT' && python3 scripts/check-twitch-poll-ui.py"
gate "RUSTFMT_REGRESSION_CONTRACT" "$LOGDIR/rustfmt-regression.log" bash -lc "cd '$ROOT' && python3 scripts/check-v208-rustfmt-regression.py"
gate "SOURCE_DIFF_WHITESPACE" "$LOGDIR/source-whitespace.log" bash -lc "cd '$ROOT' && ! grep -RInE '[[:blank:]]+$' --exclude-dir=.git --exclude-dir=node_modules --exclude-dir=target --exclude='*.png' ."

NODE_MODULES_REUSE="${OPENDECK_V212_NODE_MODULES_REUSE:-}"
if [[ ! -d "$STUDIO/node_modules" && -n "$NODE_MODULES_REUSE" && -d "$NODE_MODULES_REUSE" ]]; then
  mv "$NODE_MODULES_REUSE" "$STUDIO/node_modules"
  say "OPENDECK_V212_DEPENDENCY_REUSE=PASS"
fi
if [[ ! -d "$STUDIO/node_modules" ]]; then gate "NPM_INSTALL" "$LOGDIR/npm-install.log" bash -lc "cd '$STUDIO' && npm install --prefer-offline --no-audit --no-fund"; else say "OPENDECK_V212_NPM_INSTALL=REUSED_PREVIOUS_PASS"; fi
gate "FRONTEND_TESTS" "$LOGDIR/frontend-tests.log" bash -lc "cd '$STUDIO' && npm test"
gate "FRONTEND_LINT" "$LOGDIR/frontend-lint.log" bash -lc "cd '$STUDIO' && npm run lint"
gate "FRONTEND_BUILD" "$LOGDIR/frontend-build.log" bash -lc "cd '$STUDIO' && npm run build"
gate "CARGO_LOCK" "$LOGDIR/cargo-lock.log" bash -lc "cd '$ROOT' && cargo generate-lockfile"
gate "CARGO_FETCH" "$LOGDIR/cargo-fetch.log" bash -lc "cd '$ROOT' && cargo fetch --locked"
gate "CARGO_FMT" "$LOGDIR/cargo-fmt.log" bash -lc "cd '$ROOT' && cargo fmt --all -- --check"
gate "CARGO_CHECK" "$LOGDIR/cargo-check.log" bash -lc "cd '$ROOT' && cargo check --workspace --all-targets --all-features --locked"
gate "CARGO_CLIPPY_STRICT" "$LOGDIR/cargo-clippy.log" bash -lc "cd '$ROOT' && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings"
gate "CARGO_TEST" "$LOGDIR/cargo-test.log" bash -lc "cd '$ROOT' && cargo test --workspace --all-targets --all-features --locked"
gate "CARGO_RELEASE" "$LOGDIR/cargo-release.log" bash -lc "cd '$ROOT' && cargo build --workspace --all-features --release --locked"
gate "TAURI_BUILD" "$LOGDIR/tauri-build.log" bash -lc "cd '$STUDIO' && CI=true NO_COLOR=1 npm run tauri -- build --no-bundle"
gate "STREAMDECK_PLUS_OS_PROBE" "$LOGDIR/streamdeck-plus-probe.log" bash -lc "cd '$ROOT' && ./scripts/check-streamdeck-plus.sh"
[[ -x "$NEW_BIN" ]] || fail "NEW_BINARY_MISSING" 6
NEW_SHA="$(sha256sum "$NEW_BIN" | awk '{print $1}')"; say "OPENDECK_V212_NEW_BINARY_SHA256=$NEW_SHA"

stop_candidate(){ local pid="${1:-}"; [[ -n "$pid" ]] || return 0; kill "$pid" >/dev/null 2>&1 || true; for _ in {1..30}; do kill -0 "$pid" >/dev/null 2>&1 || break; sleep .05; done; kill -9 "$pid" >/dev/null 2>&1 || true; wait "$pid" 2>/dev/null || true; }
wait_file(){ local file="$1" pid="$2" limit="${3:-2000}"; for ((i=0;i<limit;i++)); do [[ -s "$file" ]] && return 0; kill -0 "$pid" >/dev/null 2>&1 || return 2; sleep .01; done; return 1; }
launch_candidate(){ local phase="$1"; OPENDECK_V212_QUALIFICATION=1 OPENDECK_V212_QUALIFICATION_PHASE="$phase" OPENDECK_QUALIFICATION_DIR="$QDIR" "$NEW_BIN" >>"$LOGDIR/candidate-$phase.log" 2>&1 & echo $!; }

# Startup: first launch is the cold-process sample; next ten are warm-process samples.
rm -f "$VISUAL_JSON" "$QDIR/OpenDeck-v2.0.12-QUALIFICATION-ERROR.json"
start_ns="$(date +%s%N)"; pid="$(launch_candidate visual)"; wait_file "$VISUAL_JSON" "$pid" 2000 || { stop_candidate "$pid"; fail "COLD_START_READY" 11; }; end_ns="$(date +%s%N)"; cold_ms="$(python3 - <<PY
print((${end_ns}-${start_ns})/1_000_000)
PY
)"; stop_candidate "$pid"; say "OPENDECK_V212_COLD_START_MS=$cold_ms"
WARM_FILE="$QDIR/warm-start-ms.txt"; : > "$WARM_FILE"
for run in {1..10}; do rm -f "$VISUAL_JSON"; start_ns="$(date +%s%N)"; pid="$(launch_candidate visual)"; wait_file "$VISUAL_JSON" "$pid" 1500 || { stop_candidate "$pid"; fail "WARM_START_READY_$run" 11; }; end_ns="$(date +%s%N)"; python3 - <<PY >> "$WARM_FILE"
print((${end_ns}-${start_ns})/1_000_000)
PY
stop_candidate "$pid"; done
warm_p95="$(python3 - "$WARM_FILE" <<'PY'
import math,sys
v=sorted(float(x) for x in open(sys.argv[1]) if x.strip()); print(v[max(0,math.ceil(.95*len(v))-1)])
PY
)"; say "OPENDECK_V212_WARM_START_P95_MS=$warm_p95"
python3 - "$cold_ms" "$warm_p95" <<'PY' || fail "STARTUP_PERFORMANCE" 12
import sys
cold,warm=map(float,sys.argv[1:]); assert cold<=1200,(cold,1200); assert warm<=650,(warm,650)
PY
say "OPENDECK_V212_STARTUP_PERFORMANCE=PASS"

# Visual run: candidate remains separate from active 2.0.8.
rm -f "$VISUAL_JSON" "$SCREENSHOT" "$QDIR/OpenDeck-v2.0.12-QUALIFICATION-ERROR.json"
pid="$(launch_candidate visual)"; wait_file "$VISUAL_JSON" "$pid" 2000 || { stop_candidate "$pid"; fail "VISUAL_METRICS_GENERATION" 13; }; sleep .8
if ! "$ROOT/scripts/capture-v212-window.sh" "$SCREENSHOT" >>"$LOGDIR/capture.log" 2>&1; then stop_candidate "$pid"; fail "SCREENSHOT_CAPTURE" 13; fi
stop_candidate "$pid"
gate "VISUAL_GEOMETRY" "$LOGDIR/visual-geometry.log" python3 "$ROOT/scripts/check-v212-visual-metrics.py" "$VISUAL_JSON"
gate "SCREENSHOT_SIMILARITY" "$LOGDIR/screenshot-similarity.log" python3 "$ROOT/scripts/v212-png-metrics.py" "$ROOT/qualification/OpenDeck-v2.0.12-CANONICAL-UI-TARGET.png" "$SCREENSHOT"

# Performance run. The on-screen banner gives a 10-second real-hardware exercise window.
say "OPENDECK_V212_HARDWARE_EXERCISE=WHEN_BANNER_APPEARS_ROTATE_OR_PRESS_ANY_DIAL_CONTINUOUSLY_FOR_10_SECONDS"
rm -f "$PERF_JSON" "$QDIR/OpenDeck-v2.0.12-QUALIFICATION-ERROR.json"
pid="$(launch_candidate performance)"; wait_file "$PERF_JSON" "$pid" 9000 || { stop_candidate "$pid"; fail "PERFORMANCE_METRICS_GENERATION_OR_HARDWARE_SAMPLES" 14; }; stop_candidate "$pid"

# 60-second idle process-tree efficiency sample.
rm -f "$VISUAL_JSON"
pid="$(launch_candidate visual)"; wait_file "$VISUAL_JSON" "$pid" 2000 || { stop_candidate "$pid"; fail "IDLE_READY" 15; }
IDLE_FILE="$QDIR/idle-samples.txt"; : > "$IDLE_FILE"
for sample in {1..12}; do python3 - "$pid" <<'PY' >> "$IDLE_FILE"
import subprocess,sys
root=int(sys.argv[1]); rows=[]
for line in subprocess.check_output(['ps','-eo','pid=,ppid=,%cpu=,rss='],text=True).splitlines():
    parts=line.split()
    if len(parts)!=4: continue
    try: rows.append((int(parts[0]),int(parts[1]),float(parts[2]),int(parts[3])))
    except ValueError: pass
children={}
for pid,ppid,cpu,rss in rows: children.setdefault(ppid,[]).append(pid)
keep={root}; stack=[root]
while stack:
    parent=stack.pop()
    for child in children.get(parent,[]):
        if child not in keep: keep.add(child); stack.append(child)
cpu=sum(row[2] for row in rows if row[0] in keep); rss=sum(row[3] for row in rows if row[0] in keep)
print(cpu,rss)
PY
sleep 5; done
stop_candidate "$pid"
read -r idle_cpu rss_mib < <(python3 - "$IDLE_FILE" <<'PY'
import sys
v=[tuple(map(float,line.split())) for line in open(sys.argv[1]) if line.strip()]
print(sum(x[0] for x in v)/len(v), max(x[1] for x in v)/1024)
PY
)
say "OPENDECK_V212_IDLE_CPU_PERCENT=$idle_cpu"; say "OPENDECK_V212_RSS_MAX_MIB=$rss_mib"

# Merge host measurements into the browser/Rust performance payload.
python3 - "$PERF_JSON" "$cold_ms" "$warm_p95" "$WARM_FILE" "$idle_cpu" "$rss_mib" <<'PY'
import json,sys
p,cold,warm,warmfile,cpu,rss=sys.argv[1:]
d=json.load(open(p)); d['startup']={'coldMs':float(cold),'warmP95Ms':float(warm),'warmCount':sum(1 for x in open(warmfile) if x.strip())}; d['idle']={'cpuAveragePercent':float(cpu),'rssMaxMiB':float(rss)}
open(p,'w').write(json.dumps(d,indent=2,sort_keys=True)+'\n')
PY
gate "PERFORMANCE_METRICS" "$LOGDIR/performance-metrics.log" python3 "$ROOT/scripts/check-v212-performance-metrics.py" "$PERF_JSON"
say "OPENDECK_V212_WINDOWS_REFERENCE=UNAVAILABLE_ABSOLUTE_GATES_APPLIED"

# Preserve fully-qualified candidate + evidence before human review.
rm -rf "$STAGED_QUALIFIED"; mkdir -p "$STAGED_QUALIFIED/bin"; install -m 0755 "$NEW_BIN" "$STAGED_QUALIFIED/bin/opendeck-studio"
STAGED_SHA="$(sha256sum "$STAGED_QUALIFIED/bin/opendeck-studio" | awk '{print $1}')"; [[ "$STAGED_SHA" == "$NEW_SHA" ]] || fail "QUALIFIED_STAGE_SHA_MISMATCH" 16
SCREEN_SHA="$(sha256sum "$SCREENSHOT" | awk '{print $1}')"; VISUAL_SHA="$(sha256sum "$VISUAL_JSON" | awk '{print $1}')"; PERF_SHA="$(sha256sum "$PERF_JSON" | awk '{print $1}')"
{
  printf 'SOURCE_ARCHIVE_SHA256=%s\n' "$ARCHIVE_SHA"
  printf 'QUALIFIED_BINARY_SHA256=%s\n' "$NEW_SHA"
  printf 'QUALIFICATION_SCREENSHOT_SHA256=%s\n' "$SCREEN_SHA"
  printf 'VISUAL_METRICS_SHA256=%s\n' "$VISUAL_SHA"
  printf 'PERFORMANCE_METRICS_SHA256=%s\n' "$PERF_SHA"
  printf 'RSS_MAX_MIB=%s\n' "$rss_mib"
} > "$STAMP"
chmod 600 "$STAMP"
say "OPENDECK_V212_AUTOMATED_QUALIFY=PASS"
say "OPENDECK_V212_SCREENSHOT=$SCREENSHOT"
say "OPENDECK_V212_VISUAL_METRICS=$VISUAL_JSON"
say "OPENDECK_V212_PERFORMANCE_METRICS=$PERF_JSON"
say "OPENDECK_V212_QUALIFIED_STAMP=$STAMP"
say "OPENDECK_V212_VISUAL_HUMAN_APPROVAL=PENDING"
say "OPENDECK_V212_ACTIVATE=BLOCKED_PENDING_HUMAN_VISUAL_APPROVAL"
say "VERIFY_FILE=$VERIFY"
exit 0
