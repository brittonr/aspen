#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail
cd "$(dirname "$0")/.."
BIN="${CARGO_TARGET_DIR:-$(pwd)/target}/release/molten-orchestrator"
[[ -x "$BIN" ]] || { echo 'Run scripts/verify.sh or cargo build --release -p molten-orchestrator-cli first.' >&2; exit 1; }
WORK="$(mktemp -d "${TMPDIR:-/tmp}/aspen-batch-demo.XXXXXX")"
chmod 700 "$WORK"
json_field() { python3 -c 'import json,sys; print(json.load(sys.stdin)[sys.argv[1]])' "$1"; }
# The default ports may be overridden. Each is a directly reachable UDP listener.
CPORT="${CONTROLLER_PORT:-45100}"
WPORT="${WORKER_PORT:-45101}"
"$BIN" new-policy --out "$WORK/policy.json" --allow-pure-components > "$WORK/policy-info.json"
C=$("$BIN" init --root "$WORK/controller" --role controller --policy "$WORK/policy.json" --bind "127.0.0.1:$CPORT" | json_field id)
U=$("$BIN" init --root "$WORK/client" --role client --policy "$WORK/policy.json" | json_field id)
W=$("$BIN" init --root "$WORK/worker" --role worker --policy "$WORK/policy.json" --bind "127.0.0.1:$WPORT" --controller "$C" | json_field id)
"$BIN" allow-operator --root "$WORK/controller" --peer "$U"
"$BIN" add-worker --root "$WORK/controller" --peer "$W" --address "127.0.0.1:$WPORT"
WPID=''; CPID=''
cleanup() {
  [[ -z "$CPID" ]] || kill "$CPID" 2>/dev/null || true
  [[ -z "$WPID" ]] || kill "$WPID" 2>/dev/null || true
  [[ -z "$CPID" ]] || wait "$CPID" 2>/dev/null || true
  [[ -z "$WPID" ]] || wait "$WPID" 2>/dev/null || true
  printf '\nState and logs retained at %s\n' "$WORK"
}
trap cleanup EXIT INT TERM
"$BIN" serve --root "$WORK/worker" >"$WORK/worker.log" 2>&1 & WPID=$!
"$BIN" serve --root "$WORK/controller" >"$WORK/controller.log" 2>&1 & CPID=$!
TARGET="$C@127.0.0.1:$CPORT"
ready=0
for _ in $(seq 1 30); do
  if "$BIN" status --root "$WORK/client" --target "$TARGET" > /dev/null 2>&1; then ready=1; break; fi
  sleep 0.2
done
[[ "$ready" == 1 ]] || { cat "$WORK/controller.log" >&2; exit 1; }
ART=$("$BIN" put --root "$WORK/client" --target "$TARGET" --kind component --file examples/constant.wat | json_field id)
python3 - "$ART" "$WORK/job.json" <<'PY'
import json,sys
artifact,path=sys.argv[1:]
def task(name,deps):
 return dict(id=name,component=artifact,inputs=[],dependencies=deps,resources=dict(cpu_millis=1000,memory_bytes=16777216,slots=1),required_labels={},fuel=1000000,timeout_ms=5000,max_output_bytes=1024,max_attempts=2,retry_delay_ticks=1)
with open(path,'x') as f: json.dump(dict(name='demo',tasks=[task('a',[]),task('b',['a'])]),f,indent=2)
PY
RUN=$("$BIN" submit --root "$WORK/client" --target "$TARGET" --key demo --file "$WORK/job.json" | json_field run)
finished=0
for _ in $(seq 1 120); do
  "$BIN" status --root "$WORK/client" --target "$TARGET" --run "$RUN" > "$WORK/status.json"
  PHASE=$(json_field phase < "$WORK/status.json")
  case "$PHASE" in Succeeded) finished=1; break;; Failed|Cancelled) cat "$WORK/status.json"; exit 1;; esac
  sleep 0.2
done
[[ "$finished" == 1 ]] || { cat "$WORK/status.json"; exit 1; }
OUTPUT=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["run"]["tasks"]["b"]["phase"]["Succeeded"]["output"])' "$WORK/status.json")
"$BIN" get --root "$WORK/client" --target "$TARGET" --id "$OUTPUT" --out "$WORK/output.bin"
python3 - "$WORK/output.bin" <<'PY'
import pathlib,sys
assert pathlib.Path(sys.argv[1]).read_bytes() == b'ok'
print('DAG completed through separate controller and worker processes; output is exactly b"ok".')
PY
