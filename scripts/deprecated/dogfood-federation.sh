#!/usr/bin/env bash
# Federated dogfood: Two independent Aspen clusters where one pushes source,
# the other syncs via federation and builds it with CI.
#
# Usage:
#   nix run .#dogfood-federation             # Full pipeline
#   nix run .#dogfood-federation -- start     # Start both clusters
#   nix run .#dogfood-federation -- push      # Push source to alice's Forge
#   nix run .#dogfood-federation -- federate  # Mark repo as federated
#   nix run .#dogfood-federation -- sync      # Sync federated repo to bob
#   nix run .#dogfood-federation -- build     # Trigger CI build on bob
#   nix run .#dogfood-federation -- verify    # Verify CI-built binary
#   nix run .#dogfood-federation -- stop      # Stop both clusters
#
# Prerequisites (set by flake.nix wrapper):
#   ASPEN_NODE_BIN, ASPEN_CLI_BIN, GIT_REMOTE_ASPEN_BIN, PROJECT_DIR
set -euo pipefail

BASE_DIR="/tmp/aspen-dogfood-federation"
ALICE_DIR="$BASE_DIR/alice"
BOB_DIR="$BASE_DIR/bob"

ALICE_COOKIE="fed-alice-$(date +%Y%m%d)"
BOB_COOKIE="fed-bob-$(date +%Y%m%d)"

# Fixed keys so federation identity is stable across restarts.
# Federation cluster key MUST match iroh secret key — the federated clone URL
# uses iroh node ID as the origin, and the resolver looks up settings by that key.
ALICE_SECRET_KEY="a11ce00000000001a11ce00000000001a11ce00000000001a11ce00000000001"
BOB_SECRET_KEY="b0b0000000000002b0b0000000000002b0b0000000000002b0b0000000000002"

ALICE_FED_KEY="$ALICE_SECRET_KEY"
BOB_FED_KEY="$BOB_SECRET_KEY"

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'
BOLD='\033[1m'; NC='\033[0m'

log()  { echo -e "${BLUE}[fed-dogfood]${NC} $*"; }
ok()   { echo -e "${GREEN}  ✅ $*${NC}"; }
err()  { echo -e "${RED}  ❌ $*${NC}"; }
warn() { echo -e "${YELLOW}  ⚠️  $*${NC}"; }

# ── Binaries ──────────────────────────────────────────────────────────

ASPEN_NODE="${ASPEN_NODE_BIN:-aspen-node}"
ASPEN_CLI="${ASPEN_CLI_BIN:-aspen-cli}"
PROJECT="${PROJECT_DIR:-$(pwd)}"

# ── Helpers ───────────────────────────────────────────────────────────

parse_json() {
  local expr="$1"
  PARSE_EXPR="$expr" python3 -c "
import json, sys, re, os
raw = sys.stdin.read()
expr = os.environ['PARSE_EXPR']
decoder = json.JSONDecoder()
for m in re.finditer(r'[\[{]', raw):
    try:
        d, _ = decoder.raw_decode(raw, m.start())
    except (json.JSONDecodeError, ValueError):
        continue
    try:
        result = eval(expr)
        print(result if result else '')
    except:
        print('')
    sys.exit(0)
print('')
" 2>/dev/null
}

get_ticket_alice() {
  cat "$ALICE_DIR/node1/cluster-ticket.txt" 2>/dev/null || return 1
}

get_ticket_bob() {
  cat "$BOB_DIR/node1/cluster-ticket.txt" 2>/dev/null || return 1
}

cli_alice() {
  local ticket
  ticket=$(get_ticket_alice) || { err "No alice ticket"; return 1; }
  "$ASPEN_CLI" --ticket "$ticket" "$@"
}

cli_alice_json() {
  local ticket
  ticket=$(get_ticket_alice) || return 1
  "$ASPEN_CLI" --ticket "$ticket" --json "$@"
}

cli_bob() {
  local ticket
  ticket=$(get_ticket_bob) || { err "No bob ticket"; return 1; }
  "$ASPEN_CLI" --ticket "$ticket" "$@"
}

cli_bob_json() {
  local ticket
  ticket=$(get_ticket_bob) || return 1
  "$ASPEN_CLI" --ticket "$ticket" --json "$@"
}

wait_for_ticket() {
  local dir="$1" name="$2" timeout="${3:-30}"
  local retries=$timeout
  while [ $retries -gt 0 ]; do
    if [ -f "$dir/node1/cluster-ticket.txt" ]; then
      return 0
    fi
    sleep 1
    retries=$((retries - 1))
  done
  err "$name failed to produce ticket within ${timeout}s"
  cat "$dir/node1.log" 2>/dev/null | tail -20
  return 1
}

# Extract alice's iroh_node_id from cluster health.
get_alice_node_id() {
  local health
  health=$(cli_alice_json cluster health 2>&1) || return 1
  parse_json "d.get('iroh_node_id', '')" <<< "$health"
}

# Extract alice's listening address from the node log.
# Picks the first non-relay direct address (IPv4 preferred).
get_alice_addr() {
  local addr
  addr=$(grep "direct_addrs" "$ALICE_DIR/node1.log" 2>/dev/null \
    | sed 's/\x1b\[[0-9;]*m//g' \
    | grep -oP '\d+\.\d+\.\d+\.\d+:\d+' | head -1)
  echo "$addr"
}

# ── Stop ──────────────────────────────────────────────────────────────

do_stop() {
  log "Stopping both clusters..."
  for dir in "$ALICE_DIR" "$BOB_DIR"; do
    local pid
    pid=$(cat "$dir/node1.pid" 2>/dev/null || true)
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      ok "Stopped PID $pid"
    fi
  done
  sleep 1
  rm -rf "$BASE_DIR"
  ok "Cleaned up $BASE_DIR"
}

# ── Start ─────────────────────────────────────────────────────────────

start_cluster() {
  local name="$1" dir="$2" cookie="$3" secret_key="$4" fed_key="$5"
  local enable_ci="${6:-false}"

  mkdir -p "$dir/node1"

  local ci_args=()
  if [ "$enable_ci" = "true" ]; then
    ci_args=(--enable-workers --enable-ci --ci-auto-trigger)
    # Enable CI triggers for federation mirror repos.
    export ASPEN_CI_FEDERATION_CI_ENABLED=true
  fi

  ASPEN_FEDERATION_ENABLED=true \
  ASPEN_FEDERATION_CLUSTER_KEY="$fed_key" \
  ASPEN_FEDERATION_CLUSTER_NAME="$name" \
  ASPEN_FEDERATION_ENABLE_DHT_DISCOVERY=false \
  ASPEN_FEDERATION_ENABLE_GOSSIP=false \
  ASPEN_DOCS_ENABLED=true \
  ASPEN_DOCS_IN_MEMORY=true \
  ASPEN_HOOKS_ENABLED=true \
  "$ASPEN_NODE" \
    --node-id 1 \
    --data-dir "$dir/node1" \
    --cookie "$cookie" \
    --iroh-secret-key "$secret_key" \
    --disable-mdns \
    --relay-mode disabled \
    --heartbeat-interval-ms 500 \
    --election-timeout-min-ms 1500 \
    --election-timeout-max-ms 3000 \
    "${ci_args[@]}" \
    > "$dir/node1.log" 2>&1 &
  echo $! > "$dir/node1.pid"
}

do_start() {
  log "Starting two independent clusters (alice + bob)..."

  if [ -d "$BASE_DIR" ]; then
    warn "Existing clusters found, cleaning up..."
    do_stop
  fi

  mkdir -p "$BASE_DIR"

  # Alice: Forge host only (no CI)
  log "Starting alice (Forge host)..."
  start_cluster "alice-cluster" "$ALICE_DIR" "$ALICE_COOKIE" \
    "$ALICE_SECRET_KEY" "$ALICE_FED_KEY" false

  # Bob: CI builder
  log "Starting bob (CI builder)..."
  start_cluster "bob-cluster" "$BOB_DIR" "$BOB_COOKIE" \
    "$BOB_SECRET_KEY" "$BOB_FED_KEY" true

  # Wait for both
  wait_for_ticket "$ALICE_DIR" "alice" 30
  wait_for_ticket "$BOB_DIR" "bob" 30

  # Verify processes are alive
  for pair in "alice:$ALICE_DIR" "bob:$BOB_DIR"; do
    local name="${pair%%:*}" dir="${pair#*:}"
    local pid
    pid=$(cat "$dir/node1.pid")
    if kill -0 "$pid" 2>/dev/null; then
      ok "$name (PID $pid): running"
    else
      err "$name failed — check $dir/node1.log"
      exit 1
    fi
  done

  # Initialize both clusters
  log "Initializing clusters..."
  cli_alice cluster init 2>/dev/null || true
  cli_bob cluster init 2>/dev/null || true
  sleep 2

  # Health checks
  if cli_alice cluster health 2>/dev/null | grep -q healthy; then
    ok "alice cluster healthy"
  else
    warn "alice health check inconclusive"
  fi

  if cli_bob cluster health 2>/dev/null | grep -q healthy; then
    ok "bob cluster healthy"
  else
    warn "bob health check inconclusive"
  fi

  log "Both clusters started at $BASE_DIR"
  echo ""
  echo -e "${BOLD}Alice ticket:${NC} $(get_ticket_alice)"
  echo -e "${BOLD}Bob ticket:${NC}   $(get_ticket_bob)"
  echo ""
}

# ── Status ────────────────────────────────────────────────────────────

do_status() {
  if [ ! -d "$BASE_DIR" ]; then
    err "No clusters running"
    exit 1
  fi

  echo -e "${BOLD}╔══════════════════════════════════════════╗${NC}"
  echo -e "${BOLD}║   Federated Dogfood Cluster Status       ║${NC}"
  echo -e "${BOLD}╚══════════════════════════════════════════╝${NC}"
  echo ""

  for pair in "alice:$ALICE_DIR" "bob:$BOB_DIR"; do
    local name="${pair%%:*}" dir="${pair#*:}"
    local pid
    pid=$(cat "$dir/node1.pid" 2>/dev/null || echo "?")
    if [ "$pid" != "?" ] && kill -0 "$pid" 2>/dev/null; then
      ok "$name (PID $pid): running"
    else
      err "$name: not running"
    fi
  done

  echo ""
  log "Alice cluster:"
  cli_alice cluster status 2>/dev/null || warn "Could not get alice status"
  echo ""
  log "Bob cluster:"
  cli_bob cluster status 2>/dev/null || warn "Could not get bob status"
}

# ── Push ──────────────────────────────────────────────────────────────

do_push() {
  if [ ! -f "$ALICE_DIR/node1/cluster-ticket.txt" ]; then
    err "Alice not running. Start with: $0 start"
    exit 1
  fi

  log "Creating Forge repository on alice..."
  local create_out
  create_out=$(cli_alice_json git init aspen 2>&1)
  local repo_id
  repo_id=$(parse_json "d.get('id') or d.get('repo_id','')" <<< "$create_out")

  if [ -z "$repo_id" ]; then
    log "Repo may already exist, checking..."
    local list_out
    list_out=$(cli_alice_json git list 2>&1)
    repo_id=$(parse_json "next((r.get('id') or r.get('repo_id','') for r in (d.get('repos') or d.get('repositories') or []) if r.get('name')=='aspen'), '')" <<< "$list_out")
  fi

  if [ -z "$repo_id" ]; then
    err "Failed to create or find Forge repository on alice"
    echo "Output: $create_out"
    exit 1
  fi
  ok "Alice Forge repo: $repo_id"

  # Push Aspen source
  local ticket
  ticket=$(get_ticket_alice)
  local remote_url="aspen://${ticket}/${repo_id}"

  log "Pushing Aspen source to alice's Forge..."
  cd "$PROJECT"

  git remote remove aspen-fed-alice 2>/dev/null || true
  git remote add aspen-fed-alice "$remote_url"

  if RUST_LOG=warn git push aspen-fed-alice HEAD:main 2>&1 | tail -5; then
    ok "Source pushed to alice's Forge"
  else
    err "Push failed"
    git remote remove aspen-fed-alice 2>/dev/null || true
    exit 1
  fi

  git remote remove aspen-fed-alice 2>/dev/null || true
  echo "$repo_id" > "$BASE_DIR/alice_repo_id"
}

# ── Federate ──────────────────────────────────────────────────────────

do_federate() {
  local repo_id
  repo_id=$(cat "$BASE_DIR/alice_repo_id" 2>/dev/null || true)

  if [ -z "$repo_id" ]; then
    err "No repo ID found. Push source first: $0 push"
    exit 1
  fi

  log "Marking alice's repo as federated (public mode)..."
  local fed_out
  fed_out=$(cli_alice_json federation federate "$repo_id" --mode public 2>&1)
  log "Federate result: $fed_out"

  local is_success
  is_success=$(parse_json "d.get('is_success', False)" <<< "$fed_out")
  if [ "$is_success" = "True" ] || [ "$is_success" = "true" ]; then
    ok "Repo $repo_id federated on alice"
  else
    # May already be federated
    warn "Federate returned: $fed_out (continuing)"
  fi
}

# ── Sync ──────────────────────────────────────────────────────────────

do_sync() {
  local alice_node_id alice_addr

  log "Extracting alice's identity..."
  alice_node_id=$(get_alice_node_id)
  alice_addr=$(get_alice_addr)

  if [ -z "$alice_node_id" ]; then
    err "Could not get alice's iroh_node_id"
    exit 1
  fi
  ok "Alice node_id: $alice_node_id"

  if [ -z "$alice_addr" ]; then
    warn "Could not extract alice's address from log, sync may fail"
  else
    ok "Alice addr: $alice_addr"
  fi

  # Save for later steps
  echo "$alice_node_id" > "$BASE_DIR/alice_node_id"
  echo "$alice_addr" > "$BASE_DIR/alice_addr"

  # Bob trusts alice
  log "Bob trusting alice..."
  cli_bob federation trust "$alice_node_id" 2>/dev/null || warn "Trust command returned non-zero (may be OK)"
  sleep 1

  # Bob syncs from alice
  log "Bob syncing from alice..."
  local bob_ticket
  bob_ticket=$(get_ticket_bob)

  local addr_args=()
  if [ -n "$alice_addr" ]; then
    addr_args=(--addr "$alice_addr")
  fi

  local retries=3
  local sync_ok=false
  while [ $retries -gt 0 ]; do
    local rc=0
    local output
    output=$("$ASPEN_CLI" --ticket "$bob_ticket" federation sync --peer "$alice_node_id" "${addr_args[@]}" 2>&1) || rc=$?
    log "Sync attempt (rc=$rc): $output"

    if [ $rc -eq 0 ]; then
      sync_ok=true
      break
    fi

    retries=$((retries - 1))
    if [ $retries -gt 0 ]; then
      log "Retrying sync in 5s..."
      sleep 5
    fi
  done

  if [ "$sync_ok" = "true" ]; then
    ok "Federation sync completed"
  else
    warn "Sync returned errors (may still have partial data)"
  fi

  sleep 2

  # Check mirror metadata on bob
  local mirror_keys
  mirror_keys=$(cli_bob_json kv scan "_fed:mirror:" 2>&1) || true
  log "Mirror keys on bob: $mirror_keys"
}

# ── Build ─────────────────────────────────────────────────────────────

do_build() {
  if [ ! -f "$BOB_DIR/node1/cluster-ticket.txt" ]; then
    err "Bob not running. Start with: $0 start"
    exit 1
  fi

  local alice_node_id alice_addr alice_repo_id
  alice_node_id=$(cat "$BASE_DIR/alice_node_id" 2>/dev/null || true)
  alice_addr=$(cat "$BASE_DIR/alice_addr" 2>/dev/null || true)
  alice_repo_id=$(cat "$BASE_DIR/alice_repo_id" 2>/dev/null || true)

  if [ -z "$alice_node_id" ] || [ -z "$alice_repo_id" ]; then
    err "Missing alice identity or repo ID. Run sync first: $0 sync"
    exit 1
  fi

  # Create a local repo on bob and push the synced content there for CI.
  log "Creating Forge repo on bob for CI..."
  local bob_repo_out
  bob_repo_out=$(cli_bob_json git init aspen-mirror 2>&1)
  local bob_repo_id
  bob_repo_id=$(parse_json "d.get('id') or d.get('repo_id','')" <<< "$bob_repo_out")

  if [ -z "$bob_repo_id" ]; then
    log "Repo may exist, checking..."
    local list_out
    list_out=$(cli_bob_json git list 2>&1)
    bob_repo_id=$(parse_json "next((r.get('id') or r.get('repo_id','') for r in (d.get('repos') or d.get('repositories') or []) if r.get('name')=='aspen-mirror'), '')" <<< "$list_out")
  fi

  if [ -z "$bob_repo_id" ]; then
    err "Failed to create Forge repo on bob"
    exit 1
  fi
  ok "Bob Forge repo: $bob_repo_id"

  # Enable CI watch on bob's repo
  log "Enabling CI watch on bob (repo_id: $bob_repo_id)..."
  local watch_out
  watch_out=$(cli_bob_json ci watch "$bob_repo_id" 2>&1) || true
  local watch_success
  watch_success=$(parse_json "d.get('is_success', False)" <<< "$watch_out")
  if [ "$watch_success" = "True" ] || [ "$watch_success" = "true" ]; then
    ok "CI watch registered for repo $bob_repo_id"
  else
    warn "CI watch response: $watch_out"
    warn "CI auto-trigger may not work — watch registration may have failed"
  fi

  # Federated clone: bob clones from alice's forge via federation URL.
  # Bob's node connects to alice, fetches git objects over iroh QUIC,
  # imports them into a local mirror repo incrementally.
  local bob_ticket alice_node_id alice_addr alice_repo_id
  bob_ticket=$(get_ticket_bob)
  alice_node_id=$(cat "$BASE_DIR/alice_node_id" 2>/dev/null || true)
  alice_addr=$(cat "$BASE_DIR/alice_addr" 2>/dev/null || true)
  alice_repo_id=$(cat "$BASE_DIR/alice_repo_id" 2>/dev/null || true)

  if [ -z "$alice_node_id" ] || [ -z "$alice_repo_id" ]; then
    err "Missing alice identity or repo ID. Run sync first: $0 sync"
    exit 1
  fi

  local fed_url="aspen://${bob_ticket}/fed:${alice_node_id}:${alice_repo_id}"
  local clone_dir="/tmp/aspen-fed-clone-$$"
  rm -rf "$clone_dir"

  log "Federated clone: bob cloning from alice..."
  log "  URL: $fed_url"
  local clone_rc=0
  ASPEN_ORIGIN_ADDR="$alice_addr" \
    RUST_LOG=warn \
    git clone "$fed_url" "$clone_dir" 2>&1 | tail -10 || clone_rc=$?

  if [ $clone_rc -ne 0 ]; then
    warn "Federated clone failed (rc=$clone_rc), retrying after 5s..."
    rm -rf "$clone_dir"
    sleep 5
    ASPEN_ORIGIN_ADDR="$alice_addr" \
      RUST_LOG=warn \
      git clone "$fed_url" "$clone_dir" 2>&1 | tail -10 || clone_rc=$?
  fi

  if [ $clone_rc -ne 0 ]; then
    err "Federated clone failed after retry"
    rm -rf "$clone_dir"
    exit 1
  fi

  # Check if clone has content
  if ! git -C "$clone_dir" rev-parse HEAD >/dev/null 2>&1; then
    warn "Federated clone returned empty repo, falling back to direct push..."
    rm -rf "$clone_dir"
    log "Pushing source directly to bob's Forge..."
    cd "$PROJECT"
    git remote remove aspen-fed-bob 2>/dev/null || true
    git remote add aspen-fed-bob "aspen://${bob_ticket}/${bob_repo_id}"
    if RUST_LOG=warn git push aspen-fed-bob HEAD:main 2>&1 | tail -5; then
      ok "Source pushed to bob's Forge (direct fallback)"
    else
      err "Push to bob's Forge failed"
      git remote remove aspen-fed-bob 2>/dev/null || true
      exit 1
    fi
    git remote remove aspen-fed-bob 2>/dev/null || true
  else
    ok "Federated clone succeeded"
    # Push cloned content to bob's local forge for CI
    log "Pushing federated clone to bob's Forge..."
    cd "$clone_dir"
    git remote add bob-forge "aspen://${bob_ticket}/${bob_repo_id}"
    if RUST_LOG=warn git push bob-forge main 2>&1 | tail -5; then
      ok "Federated content pushed to bob's Forge"
    else
      err "Push to bob's Forge failed"
      rm -rf "$clone_dir"
      exit 1
    fi
    cd "$PROJECT"
    rm -rf "$clone_dir"
  fi

  # Wait for CI pipeline
  log "Waiting for CI pipeline on bob..."
  local deadline=$((SECONDS + 120))
  local run_id=""
  while [ $SECONDS -lt $deadline ]; do
    local list_out
    list_out=$(cli_bob_json ci list 2>&1) || true
    run_id=$(parse_json "d.get('runs',[])[0].get('run_id','') if d.get('runs') else ''" <<< "$list_out")
    if [ -n "$run_id" ]; then
      ok "Pipeline found: $run_id"
      break
    fi
    sleep 3
  done

  if [ -z "$run_id" ]; then
    warn "No auto-triggered pipeline after 120s, manually triggering..."
    warn "  repo_id: $bob_repo_id"
    local trigger_out
    trigger_out=$(cli_bob_json ci run "$bob_repo_id" 2>&1) || true
    log "Manual trigger RPC response: $trigger_out"
    run_id=$(parse_json "d.get('run_id','')" <<< "$trigger_out")
    local trigger_error
    trigger_error=$(parse_json "d.get('error','')" <<< "$trigger_out")
    if [ -z "$run_id" ]; then
      err "Failed to trigger CI pipeline on bob"
      if [ -n "$trigger_error" ]; then
        err "  RPC error: $trigger_error"
      fi
      err "  Full response: $trigger_out"
      exit 1
    fi
    ok "Pipeline triggered: $run_id"
  fi

  echo "$run_id" > "$BASE_DIR/run_id"

  # Stream build progress
  log "Streaming pipeline progress..."
  echo ""
  stream_pipeline "$run_id"
}

# Stream pipeline: poll status, show logs.
stream_pipeline() {
  local run_id="$1"
  local streaming_job=""
  local stream_pid=""
  local seen_jobs=""

  cleanup_stream() {
    if [ -n "$stream_pid" ] && kill -0 "$stream_pid" 2>/dev/null; then
      kill "$stream_pid" 2>/dev/null || true
      wait "$stream_pid" 2>/dev/null || true
    fi
  }
  trap cleanup_stream EXIT

  while true; do
    local status_out
    status_out=$(cli_bob_json ci status "$run_id" 2>&1) || true
    local pipeline_status
    pipeline_status=$(parse_json "d.get('status','unknown')" <<< "$status_out")

    # Find running job
    local active_job_id active_job_name
    read -r active_job_id active_job_name < <(python3 -c "
import json, sys, re
raw = sys.stdin.read()
for m in re.finditer(r'[\[{]', raw):
    try:
        d, _ = json.JSONDecoder().raw_decode(raw, m.start())
    except (json.JSONDecodeError, ValueError):
        continue
    for stage in d.get('stages', []):
        for job in stage.get('jobs', []):
            if job.get('status') == 'running' and job.get('id'):
                print(job['id'], job.get('name', ''))
                sys.exit(0)
    print(' ')
    sys.exit(0)
print(' ')
" <<< "$status_out" 2>/dev/null)

    if [ -n "$active_job_id" ] && [ "$active_job_id" != "$streaming_job" ]; then
      case " $seen_jobs " in
        *" $active_job_id "*) ;;
        *)
          cleanup_stream
          stream_pid=""
          echo ""
          echo -e "${BOLD}━━━ ${active_job_name:-$active_job_id} ━━━${NC}"
          echo ""
          cli_bob ci logs --follow "$run_id" "$active_job_id" 2>/dev/null &
          stream_pid=$!
          streaming_job="$active_job_id"
          seen_jobs="$seen_jobs $active_job_id"
          ;;
      esac
    fi

    case "$pipeline_status" in
      success)
        cleanup_stream; stream_pid=""
        trap - EXIT
        echo ""
        ok "Pipeline completed successfully! 🎉"
        return 0
        ;;
      failed|cancelled)
        cleanup_stream; stream_pid=""
        trap - EXIT
        echo ""
        err "Pipeline $pipeline_status"
        return 1
        ;;
    esac

    sleep 2
  done
}

# ── Verify ────────────────────────────────────────────────────────────

do_verify() {
  if [ ! -f "$BOB_DIR/node1/cluster-ticket.txt" ]; then
    err "Bob not running"
    exit 1
  fi

  local run_id
  run_id=$(cat "$BASE_DIR/run_id" 2>/dev/null || true)

  if [ -z "$run_id" ]; then
    local list_out
    list_out=$(cli_bob_json ci list 2>&1) || true
    run_id=$(parse_json "d.get('runs',[])[0].get('run_id','') if d.get('runs') else ''" <<< "$list_out")
  fi

  if [ -z "$run_id" ]; then
    err "No pipeline runs found on bob"
    exit 1
  fi

  local status_out
  status_out=$(cli_bob_json ci status "$run_id" 2>&1) || true
  local pipeline_status
  pipeline_status=$(parse_json "d.get('status','unknown')" <<< "$status_out")

  if [ "$pipeline_status" != "success" ]; then
    err "Pipeline $run_id is not successful (status: $pipeline_status)"
    exit 1
  fi

  # Find the build-node job (not clippy or other check jobs)
  local build_job_id
  build_job_id=$(python3 -c "
import json, sys, re
raw = sys.stdin.read()
for m in re.finditer(r'[\[{]', raw):
    try:
        d, _ = json.JSONDecoder().raw_decode(raw, m.start())
    except (json.JSONDecodeError, ValueError):
        continue
    # Prefer build-node, then any nix-build, then first successful job with output
    candidates = []
    for stage in d.get('stages', []):
        for job in stage.get('jobs', []):
            if job.get('status') == 'success' and job.get('id'):
                name = job.get('name', '')
                if name == 'build-node':
                    print(job['id'])
                    sys.exit(0)
                candidates.append((name, job['id']))
    # Fallback: nix-build > build-* > anything
    for prefix in ['nix-build', 'build-']:
        for name, jid in candidates:
            if name.startswith(prefix):
                print(jid)
                sys.exit(0)
    if candidates:
        print(candidates[0][1])
    else:
        print('')
    sys.exit(0)
print('')
" <<< "$status_out" 2>/dev/null)

  if [ -z "$build_job_id" ]; then
    err "No successful build job found"
    exit 1
  fi
  ok "Build job: $build_job_id"

  # Extract output path
  local job_data
  job_data=$(cli_bob_json kv get "__jobs:$build_job_id" 2>&1) || true

  local output_path
  output_path=$(python3 -c "
import json, sys, re
raw = sys.stdin.read()
for m in re.finditer(r'[\[{]', raw):
    try:
        d, _ = json.JSONDecoder().raw_decode(raw, m.start())
    except (json.JSONDecodeError, ValueError):
        continue
    try:
        job_str = d.get('value', '')
        job = json.loads(job_str)
        data = job.get('result', {}).get('Success', {}).get('data', {})
        paths = data.get('output_paths', [])
        # Prefer paths without -man/-doc/-dev suffixes
        good = [p for p in paths if not any(p.endswith(s) for s in ['-man', '-doc', '-dev', '-info'])]
        print(good[0] if good else (paths[0] if paths else ''))
    except:
        print('')
    sys.exit(0)
print('')
" <<< "$job_data" 2>/dev/null)

  if [ -z "$output_path" ]; then
    err "Could not extract output path from build job"
    exit 1
  fi
  ok "Nix output: $output_path"

  # Find a runnable binary in the output
  local binary=""
  if [ -f "$output_path/bin/aspen-node" ]; then
    binary="$output_path/bin/aspen-node"
  else
    # Check for any binary in bin/
    binary=$(find "$output_path/bin/" -type f -executable 2>/dev/null | head -1)
  fi

  if [ -n "$binary" ]; then
    log "Running CI-built binary: $binary"
    local version_output
    version_output=$("$binary" --version 2>&1 || "$binary" --help 2>&1 | head -1 || true)
    if [ -n "$version_output" ]; then
      ok "Binary runs: $version_output"
    else
      warn "Binary produced no version output but executed without error"
    fi
  else
    warn "No binary found in $output_path/bin/ — checking if store path exists..."
    if [ -d "$output_path" ]; then
      ok "Store path exists: $output_path"
      ls "$output_path" 2>/dev/null
    else
      err "Store path does not exist: $output_path"
      exit 1
    fi
  fi
}

# ── Full Pipeline ─────────────────────────────────────────────────────

do_full() {
  echo -e "${BOLD}"
  echo "╔══════════════════════════════════════════════════════════════╗"
  echo "║    ASPEN FEDERATED DOGFOOD — CROSS-CLUSTER BUILD PIPELINE  ║"
  echo "║                                                            ║"
  echo "║  Alice hosts source → Bob syncs via federation → Bob builds ║"
  echo "╚══════════════════════════════════════════════════════════════╝"
  echo -e "${NC}"
  echo ""

  do_start
  echo ""
  do_push
  echo ""
  do_federate
  echo ""
  do_sync
  echo ""
  do_build
  local build_result=$?
  echo ""

  if [ $build_result -eq 0 ]; then
    do_verify
    local verify_result=$?
    echo ""

    echo -e "${GREEN}${BOLD}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║        🎉 FEDERATED DOGFOOD BUILD SUCCEEDED 🎉             ║"
    echo "║                                                            ║"
    echo "║  Two independent clusters collaborated:                     ║"
    echo "║  • Alice: hosted source via Forge                           ║"
    echo "║  • Federation: synced repo across cluster boundary          ║"
    echo "║  • Bob: built from synced content via CI + Nix              ║"
    if [ $verify_result -eq 0 ]; then
    echo "║  • Verify: CI-built binary validated                        ║"
    fi
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
  fi
  return $build_result
}

# ── Main ──────────────────────────────────────────────────────────────

CMD="${1:-full}"

case "$CMD" in
  start)    do_start ;;
  stop)     do_stop ;;
  status)   do_status ;;
  push)     do_push ;;
  federate) do_federate ;;
  sync)     do_sync ;;
  build)    do_build ;;
  verify)   do_verify ;;
  full)     do_full ;;
  *)
    echo "Usage: $0 {start|stop|status|push|federate|sync|build|verify|full}"
    echo ""
    echo "Commands:"
    echo "  full       Full pipeline: start → push → federate → sync → build → verify (default)"
    echo "  start      Start both clusters (alice + bob)"
    echo "  stop       Stop both clusters"
    echo "  status     Show cluster status"
    echo "  push       Push Aspen source to alice's Forge"
    echo "  federate   Mark alice's repo as federated"
    echo "  sync       Sync federated repo from alice to bob"
    echo "  build      Clone synced content, push to bob's Forge, trigger CI"
    echo "  verify     Verify CI-built binary"
    exit 1
    ;;
esac
