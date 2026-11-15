#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Same-host authored fixture, not a physical-host qualification claim.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ "${RENDER_SKIP_BUILD:-0}" == 1 ]] || bash scripts/render-build.sh
TARGET_DIR="${CARGO_TARGET_DIR:-$(pwd)/target}"
BIN="$TARGET_DIR/release/molten-orchestrator"
RENDER="$TARGET_DIR/release/molten-render-fixture"
COMPONENT="$TARGET_DIR/guests/render.component.wasm"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/molten-render.XXXXXX")"
chmod 700 "$WORK"
REPO="$(git rev-parse --show-toplevel)"
{
  printf 'product=Aspen\nscope=same-host-separate-process; no cross-host or machine-loss qualification\n'
  rustc --version --verbose
  cargo --version --verbose
  uname -srm
  wasm-tools --version
  printf 'cargo=%s\nrustc=%s\nwasm-tools=%s\n' "$(command -v cargo)" "$(command -v rustc)" "$(command -v wasm-tools)"
  env | LC_ALL=C sort | grep -E '^(CARGO_(TARGET_DIR|BUILD_JOBS|INCREMENTAL|NET_OFFLINE|PROFILE_DEV_DEBUG|PROFILE_TEST_DEBUG)|TMPDIR|IN_NIX_SHELL)='
  stat -f -c 'artifact-filesystem=%T' "$WORK"
  b3sum "$BIN" "$RENDER" "$COMPONENT"
} > "$WORK/environment.txt"
{
  git -C "$REPO" ls-files -z --cached --others --exclude-standard -- .cargo rust-toolchain.toml flake.lock pilots/orchestration crates/molten-core/src/committed_stream
  if [[ -f "$REPO/pilots/orchestration/examples/render-guest/Cargo.lock" ]]; then printf '%s\0' pilots/orchestration/examples/render-guest/Cargo.lock; fi
} | sort -zu > "$WORK/source-files.nul"
tar -C "$REPO" --null -T "$WORK/source-files.nul" -cf "$WORK/source.tar"
b3sum "$WORK/source.tar" > "$WORK/source.blake3"
json_field() { python3 -c 'import json,sys; print(json.load(sys.stdin)[sys.argv[1]])' "$1"; }
CPORT="${CONTROLLER_PORT:-45200}"
WPORT="${WORKER_PORT:-45201}"
W2PORT="${WORKER2_PORT:-45202}"
CPID=''; W1PID=''; W2PID=''
cleanup() {
  for p in "$CPID" "$W1PID" "$W2PID"; do [[ -z "$p" ]] || kill "$p" 2>/dev/null || true; done
  for p in "$CPID" "$W1PID" "$W2PID"; do [[ -z "$p" ]] || wait "$p" 2>/dev/null || true; done
  printf '\nPrivate state, inputs, logs and artifacts retained at %s\n' "$WORK"
}
trap cleanup EXIT
trap 'exit 130' INT TERM
"$BIN" new-policy --out "$WORK/policy.json" --allow-pure-components > "$WORK/policy-info.json"
C=$("$BIN" init --root "$WORK/controller" --role controller --policy "$WORK/policy.json" --bind "127.0.0.1:$CPORT" | json_field id)
U=$("$BIN" init --root "$WORK/client" --role client --policy "$WORK/policy.json" | json_field id)
W1=$("$BIN" init --root "$WORK/worker-a" --role worker --policy "$WORK/policy.json" --bind "127.0.0.1:$WPORT" --controller "$C" | json_field id)
W2=$("$BIN" init --root "$WORK/worker-b" --role worker --policy "$WORK/policy.json" --bind "127.0.0.1:$W2PORT" --controller "$C" | json_field id)
"$BIN" allow-operator --root "$WORK/controller" --peer "$U"
"$BIN" add-worker --root "$WORK/controller" --peer "$W1" --address "127.0.0.1:$WPORT"
"$BIN" add-worker --root "$WORK/controller" --peer "$W2" --address "127.0.0.1:$W2PORT"
"$BIN" serve --root "$WORK/worker-a" > "$WORK/worker-a.log" 2>&1 & W1PID=$!
"$BIN" serve --root "$WORK/worker-b" > "$WORK/worker-b.log" 2>&1 & W2PID=$!
"$BIN" serve --root "$WORK/controller" > "$WORK/controller.log" 2>&1 & CPID=$!
TARGET="$C@127.0.0.1:$CPORT"
ready() {
  for _ in $(seq 1 100); do
    if "$BIN" status --root "$WORK/client" --target "$TARGET" > /dev/null 2>&1; then return; fi
    sleep 0.2
  done
  echo 'Controller not ready; inspect retained logs.' >&2; return 1
}
ready
ART=$("$BIN" put --root "$WORK/client" --target "$TARGET" --kind component --file "$COMPONENT" | json_field id)
PREP_START=$SECONDS
"$RENDER" prepare "$COMPONENT" "$WORK/prepared" > "$WORK/prepared-plan.json"
[[ "$ART" == "$(json_field component < "$WORK/prepared-plan.json")" ]]
PREP_SECONDS=$((SECONDS - PREP_START))
for file in "$WORK/prepared/"*.input; do
  ID=$("$BIN" put --root "$WORK/client" --target "$TARGET" --kind data --file "$file" | json_field id)
  [[ "${file##*/}" == "$ID.input" ]]
done
await_run() {
  local run=$1 out=$2
  for _ in $(seq 1 "${RENDER_POLL_LIMIT:-3000}"); do
    "$BIN" status --root "$WORK/client" --target "$TARGET" --run "$run" > "$out"
    case "$(json_field phase < "$out")" in
      Succeeded) return;; Failed|Cancelled) cat "$out" >&2; return 1;;
    esac
    sleep 0.2
  done
  echo 'Render did not reach success within polling bound.' >&2; return 1
}
EXEC_START=$SECONDS
"$BIN" submit --root "$WORK/client" --target "$TARGET" --key render-a --file "$WORK/prepared/job.json" > "$WORK/submission-a.json"
RUN=$(json_field run < "$WORK/submission-a.json")
"$BIN" submit --root "$WORK/client" --target "$TARGET" --key render-a --file "$WORK/prepared/job.json" > "$WORK/submission-duplicate.json"
DUP=$(json_field run < "$WORK/submission-duplicate.json")
[[ "$RUN" == "$DUP" ]]
# A one-shot observer disconnects here; execution is not cancelled.
"$BIN" progress --root "$WORK/client" --target "$TARGET" --run "$RUN" > "$WORK/progress-initial.json"
python3 -c 'import json,sys; json.dump(json.load(open(sys.argv[1]))["watermark"],open(sys.argv[2],"w"))' "$WORK/progress-initial.json" "$WORK/progress-cursor.json"
await_run "$RUN" "$WORK/status-a.json"
EXEC_SECONDS=$((SECONDS - EXEC_START))
for page in $(seq 1 8); do
  "$BIN" progress --root "$WORK/client" --target "$TARGET" --cursor "$WORK/progress-cursor.json" --limit 64 > "$WORK/progress-page-$page.json"
  COUNT=$(python3 - "$WORK/progress-page-$page.json" "$WORK/progress-cursor.json" <<'PY'
import json,sys
p=json.load(open(sys.argv[1]))
if "Events" not in p: raise SystemExit("Explicit progress resnapshot/epoch/service outcome: "+repr(p))
json.dump(p["Events"]["next"],open(sys.argv[2],"w"))
print(len(p["Events"]["events"]))
PY
)
  [[ "$COUNT" != 0 ]] || break
  [[ "$page" != 8 ]] || { echo 'Progress page bound exceeded.' >&2; exit 1; }
done
"$BIN" progress --root "$WORK/client" --target "$TARGET" --run "$RUN" > "$WORK/progress-complete.json"
python3 - "$WORK" "$RUN" <<'PY'
import glob,json,sys
root,run=sys.argv[1:]
initial=json.load(open(root+"/progress-initial.json")); complete=json.load(open(root+"/progress-complete.json"))
projected={k:v["Accepted"]["output"] for k,v in initial["tasks"].items() if "Accepted" in v}
seen={}
for path in sorted(glob.glob(root+"/progress-page-*.json")):
    events=json.load(open(path))["Events"]["events"]
    for event in events+events: # Replay the same page: apply stable identities once.
        identity=event["event_ref"]
        if identity in seen:
            assert seen[identity]==event
            continue
        seen[identity]=event
        payload=event["payload"]
        if "AcceptedOutput" not in payload: continue
        accepted=payload["AcceptedOutput"]; fence=accepted["fence"]
        assert (fence["run"],fence["spec_ref"],fence["cluster"],fence["generation"]) == (run,initial["spec_ref"],initial["cluster"],initial["generation"])
        name=fence["task"]
        assert name not in projected or projected[name]==accepted["output"]
        assert complete["tasks"][name]["Accepted"]["fence"]==fence
        projected[name]=accepted["output"]
        print("Accepted batch",name,accepted["output"])
expected={k:v["Accepted"]["output"] for k,v in complete["tasks"].items()}
assert complete["phase"]=="Succeeded" and projected==expected
with open(root+"/progress-projection.json","x") as out: json.dump(dict(run=run,outputs=projected,event_refs=sorted(seen)),out,indent=2)
PY
output_id() { python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["run"]["tasks"]["assemble"]["phase"]["Succeeded"]["output"])' "$1"; }
OUTPUT=$(output_id "$WORK/status-a.json")
TRANSFER_START=$SECONDS
"$BIN" get --root "$WORK/client" --target "$TARGET" --id "$OUTPUT" --out "$WORK/output.rgb-envelope"
TRANSFER_SECONDS=$((SECONDS - TRANSFER_START))
VERIFY_START=$SECONDS
"$RENDER" verify "$WORK/prepared/manifest.frame" "$WORK/output.rgb-envelope" "$OUTPUT" "$WORK/image.ppm" > "$WORK/verification.json"
VERIFY_SECONDS=$((SECONDS - VERIFY_START))
# Completed controller restart must preserve the same accepted result.
kill "$CPID"; wait "$CPID" || true; CPID=''
"$BIN" serve --root "$WORK/controller" > "$WORK/controller-restarted.log" 2>&1 & CPID=$!
ready
"$BIN" status --root "$WORK/client" --target "$TARGET" --run "$RUN" > "$WORK/status-restarted.json"
[[ "$(output_id "$WORK/status-restarted.json")" == "$OUTPUT" ]]
"$BIN" progress --root "$WORK/client" --target "$TARGET" --run "$RUN" > "$WORK/progress-restarted.json"
python3 - "$WORK/progress-complete.json" "$WORK/progress-restarted.json" <<'PY'
import json,sys
a,b=[json.load(open(p)) for p in sys.argv[1:]]
assert a["tasks"]==b["tasks"] and a["watermark"]==b["watermark"]
PY
# Repeat the exact manifest under a fresh run: compare every accepted tile and final identity.
"$BIN" submit --root "$WORK/client" --target "$TARGET" --key render-b --file "$WORK/prepared/job.json" > "$WORK/submission-b.json"
REPEAT=$(json_field run < "$WORK/submission-b.json")
await_run "$REPEAT" "$WORK/status-b.json"
python3 - "$WORK/status-a.json" "$WORK/status-b.json" "$WORK/metrics.json" "$PREP_SECONDS" "$EXEC_SECONDS" "$TRANSFER_SECONDS" "$VERIFY_SECONDS" <<'PY'
import json,sys
first,second,metrics,*durations=sys.argv[1:]
a=json.load(open(first))["run"]["tasks"]
b=json.load(open(second))["run"]["tasks"]
assert set(a)==set(b)
for key in a:
    assert a[key]["phase"]["Succeeded"]["output"] == b[key]["phase"]["Succeeded"]["output"], key
record=dict(schema="molten.render.fixture-observation.v1", qualification="same-host separate-process only",
    scene_preparation_seconds=int(durations[0]), execution_plus_scheduling_transfer_and_assembly_seconds=int(durations[1]),
    final_download_seconds=int(durations[2]), local_verification_seconds=int(durations[3]),
    precision="shell whole seconds", scheduling_seconds=None, tile_transfer_seconds=None,
    assembly_seconds=None, retry_cost=None, peak_memory_bytes=None,
    unavailable_reason="phase instrumentation not exposed by existing CLI; no invented measurements")
with open(metrics,'x') as f: json.dump(record,f,indent=2)
print("Same-host separate-process render, duplicate submission, completed restart and fixed-profile output identity verified.")
PY
python3 - "$WORK" <<'PY'
import json,pathlib,sys
root=pathlib.Path(sys.argv[1])
def load(name): return json.loads((root/name).read_text())
a=load("submission-a.json")["run"]; duplicate=load("submission-duplicate.json")["run"]; b=load("submission-b.json")["run"]
assert a==duplicate and a!=b
first=load("status-a.json")["run"]["tasks"]
repeat=load("status-b.json")["run"]["tasks"]
restarted=load("status-restarted.json")["run"]["tasks"]
outputs=lambda tasks: {k:v["phase"]["Succeeded"]["output"] for k,v in tasks.items()}
assert outputs(first)==outputs(repeat)==outputs(restarted)
record=dict(schema="molten.render.acceptance-observation.v1",run=a,duplicate_run=duplicate,repeat_run=b,
    duplicate_same_run=True,repeat_fresh_run=True,accepted_task_count=len(first),
    repeated_task_outputs_equal=True,restarted_task_outputs_equal=True,
    restarted_progress_tasks_and_watermark_equal=True,progress_replay_apply_once=True,
    output=outputs(first)["assemble"])
with (root/"observations.json").open('x') as out: json.dump(record,out,indent=2)
PY
printf 'Image: %s/image.ppm\n' "$WORK"
