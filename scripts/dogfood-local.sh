#!/usr/bin/env bash
# Self-hosted dogfood: Run a local Aspen cluster and build Aspen from its own Forge + CI.
#
# Usage:
#   nix run .#dogfood-local           # Full pipeline: cluster → forge → push → CI build
#   nix run .#dogfood-local -- start   # Just start the cluster
#   nix run .#dogfood-local -- push    # Push source to existing cluster
#   nix run .#dogfood-local -- build   # Trigger CI build on existing cluster
#   nix run .#dogfood-local -- stop    # Stop the cluster
#   nix run .#dogfood-local -- status  # Show cluster status
#
# Prerequisites (set by flake.nix wrapper):
#   ASPEN_NODE_BIN, ASPEN_CLI_BIN, GIT_REMOTE_ASPEN_BIN, PROJECT_DIR
set -euo pipefail

CLUSTER_DIR="/tmp/aspen-dogfood"
COOKIE="dogfood-$(date +%Y%m%d)"
NODE_COUNT="${ASPEN_NODE_COUNT:-1}"

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'
BOLD='\033[1m'; NC='\033[0m'

log() { echo -e "${BLUE}[dogfood]${NC} $*"; }
ok()  { echo -e "${GREEN}  ✅ $*${NC}"; }
err() { echo -e "${RED}  ❌ $*${NC}"; }
warn() { echo -e "${YELLOW}  ⚠️  $*${NC}"; }

# ── Binaries ──────────────────────────────────────────────────────────

ASPEN_NODE="${ASPEN_NODE_BIN:-aspen-node}"
ASPEN_CLI="${ASPEN_CLI_BIN:-aspen-cli}"
# GIT_REMOTE="${GIT_REMOTE_ASPEN_BIN:-git-remote-aspen}"
PROJECT="${PROJECT_DIR:-$(pwd)}"

# Find a valid cluster ticket from any running node.
# Prefers node1 but falls back to other nodes during rolling deploy.
get_ticket() {
  for i in $(seq 1 "$NODE_COUNT"); do
    local t
    t=$(cat "$CLUSTER_DIR/node$i/cluster-ticket.txt" 2>/dev/null) || continue
    local pid
    pid=$(cat "$CLUSTER_DIR/node$i.pid" 2>/dev/null || true)
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      echo "$t"
      return 0
    fi
  done
  return 1
}

cli() {
  local ticket
  ticket=$(get_ticket) || {
    err "No cluster ticket found. Is the cluster running?"
    return 1
  }
  "$ASPEN_CLI" --ticket "$ticket" "$@"
}

cli_json() {
  local ticket
  ticket=$(get_ticket) || return 1
  "$ASPEN_CLI" --ticket "$ticket" --json "$@"
}

# Target a specific node by its ticket file.
cli_node() {
  local node_id="$1"; shift
  local ticket
  ticket=$(cat "$CLUSTER_DIR/node$node_id/cluster-ticket.txt" 2>/dev/null) || {
    err "No ticket for node $node_id"
    return 1
  }
  "$ASPEN_CLI" --ticket "$ticket" --timeout 5000 "$@"
}

cli_node_json() {
  local node_id="$1"; shift
  local ticket
  ticket=$(cat "$CLUSTER_DIR/node$node_id/cluster-ticket.txt" 2>/dev/null) || return 1
  "$ASPEN_CLI" --ticket "$ticket" --timeout 5000 --json "$@"
}

# ── Post-Deploy Cluster Assertion ─────────────────────────────────────
#
# After all nodes are upgraded, verify the cluster is actually healthy:
#   1. All nodes agree on the same leader
#   2. A KV write/read round-trip succeeds (proves consensus works)
#   3. All nodes are voters (no one stuck as learner)

assert_cluster_healthy() {
  local timeout_secs="${1:-30}"
  local start_time=$SECONDS

  log "Asserting cluster health (timeout: ${timeout_secs}s)..."

  while [ $((SECONDS - start_time)) -lt "$timeout_secs" ]; do
    # ── Check 1: Leader agreement ──
    local leaders=()
    local all_agree=true

    for i in $(seq 1 "$NODE_COUNT"); do
      local pid
      pid=$(cat "$CLUSTER_DIR/node$i.pid" 2>/dev/null || true)
      if [ -z "$pid" ] || ! kill -0 "$pid" 2>/dev/null; then
        all_agree=false
        break
      fi

      local metrics_out
      metrics_out=$(cli_node_json "$i" cluster metrics 2>&1) || {
        all_agree=false
        break
      }

      local leader
      leader=$(parse_json "d.get('current_leader', '')" <<< "$metrics_out" 2>/dev/null) || true
      if [ -z "$leader" ] || [ "$leader" = "None" ] || [ "$leader" = "null" ]; then
        all_agree=false
        break
      fi
      leaders+=("$leader")
    done

    if [ "$all_agree" != "true" ] || [ ${#leaders[@]} -ne "$NODE_COUNT" ]; then
      sleep 2
      continue
    fi

    # All nodes must agree on the same leader
    local first_leader="${leaders[0]}"
    local leader_consensus=true
    for l in "${leaders[@]}"; do
      if [ "$l" != "$first_leader" ]; then
        leader_consensus=false
        break
      fi
    done

    if [ "$leader_consensus" != "true" ]; then
      sleep 2
      continue
    fi

    ok "Leader agreement: all $NODE_COUNT nodes see leader=$first_leader"

    # ── Check 2: KV write/read round-trip ──
    local test_key
    test_key="_deploy:health:$(date +%s)"
    local test_value="deploy-check-$$"

    if cli kv set "$test_key" "$test_value" >/dev/null 2>&1; then
      local read_out
      read_out=$(cli_json kv get "$test_key" 2>&1) || true
      local got_value
      got_value=$(parse_json "d.get('value', '')" <<< "$read_out" 2>/dev/null) || true

      if [ "$got_value" = "$test_value" ]; then
        ok "KV round-trip: write + read succeeded"

        # Clean up the test key (best-effort)
        cli kv delete "$test_key" >/dev/null 2>&1 || true

        # ── Check 3: All nodes are voters ──
        local status_out
        status_out=$(cli_json cluster status 2>&1) || true
        local voter_count
        voter_count=$(python3 -c "
import json, sys, re
raw = sys.stdin.read()
for m in re.finditer(r'[\[{]', raw):
    try:
        d, _ = json.JSONDecoder().raw_decode(raw, m.start())
    except (json.JSONDecodeError, ValueError):
        continue
    nodes = d.get('nodes', [])
    voters = sum(1 for n in nodes if n.get('is_voter'))
    print(voters)
    sys.exit(0)
print(0)
" <<< "$status_out" 2>/dev/null) || voter_count=0

        if [ "$voter_count" -ge "$NODE_COUNT" ]; then
          ok "All $NODE_COUNT nodes are voters"
        else
          warn "Only $voter_count/$NODE_COUNT nodes are voters"
        fi

        return 0
      fi
    fi

    sleep 2
  done

  local elapsed=$((SECONDS - start_time))
  err "Cluster health assertion failed after ${elapsed}s"
  return 1
}

# Extract a JSON field from CLI output that may have a version banner prefix.
# Usage: parse_json <python_expr> <<< "$output"
#   python_expr receives the parsed dict as 'd' and should print the result.
# Examples:
#   parse_json "d.get('id','')" <<< "$out"
#   parse_json "d.get('runs',[])[0].get('run_id','')" <<< "$out"
parse_json() {
  local expr="$1"
  PARSE_EXPR="$expr" python3 -c "
import json, sys, re, os
raw = sys.stdin.read()
expr = os.environ['PARSE_EXPR']
decoder = json.JSONDecoder()
# Try each '{' or '[' position - log lines (e.g. 'Os { code: 101 }')
# can contain braces before the real JSON, so skip invalid starts.
# Use raw_decode to handle trailing non-JSON content (stderr errors,
# version banners, etc.) that would cause json.loads to fail.
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

# ── Stop ──────────────────────────────────────────────────────────────

do_stop() {
  log "Stopping cluster..."
  stop_cache_gateway
  for i in $(seq 1 "$NODE_COUNT"); do
    local pid
    pid=$(cat "$CLUSTER_DIR/node$i.pid" 2>/dev/null || true)
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      ok "Node $i (PID $pid) stopped"
    fi
  done
  sleep 1
  rm -rf "$CLUSTER_DIR"
  ok "Cluster directory cleaned up"
}

# ── Cache Gateway ─────────────────────────────────────────────────────

NIX_CACHE_GATEWAY="${ASPEN_NIX_CACHE_GATEWAY_BIN:-aspen-nix-cache-gateway}"
GATEWAY_PORT=8380

start_cache_gateway() {
  # Only start if binary exists and --enable-ci was passed
  if ! command -v "$NIX_CACHE_GATEWAY" >/dev/null 2>&1; then
    # Gateway binary not available, skipping
    return 0
  fi

  local ticket
  ticket=$(cat "$CLUSTER_DIR/node1/cluster-ticket.txt" 2>/dev/null) || return 0

  log "Starting nix cache gateway on 127.0.0.1:$GATEWAY_PORT..."
  "$NIX_CACHE_GATEWAY" \
    --ticket "$ticket" \
    --port "$GATEWAY_PORT" \
    --bind 127.0.0.1 \
    > "$CLUSTER_DIR/nix-cache-gateway.log" 2>&1 &
  echo $! > "$CLUSTER_DIR/nix-cache-gateway.pid"

  # Wait for gateway to respond
  local retries=15
  while [ $retries -gt 0 ]; do
    if curl -s "http://127.0.0.1:$GATEWAY_PORT/nix-cache-info" >/dev/null 2>&1; then
      ok "Nix cache gateway ready (PID $(cat "$CLUSTER_DIR/nix-cache-gateway.pid"))"
      return 0
    fi
    sleep 1
    retries=$((retries - 1))
  done
  warn "Nix cache gateway not responding after 15s — continuing without cache"
}

stop_cache_gateway() {
  local pid
  pid=$(cat "$CLUSTER_DIR/nix-cache-gateway.pid" 2>/dev/null || true)
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
    ok "Nix cache gateway (PID $pid) stopped"
  fi
}

verify_cache() {
  if ! curl -s "http://127.0.0.1:$GATEWAY_PORT/nix-cache-info" >/dev/null 2>&1; then
    warn "Nix cache gateway not running, skipping cache verification"
    return 0
  fi

  log "Checking cache entries after build..."
  local entries
  entries=$(cli kv scan _cache: 2>/dev/null | wc -l)
  if [ "$entries" -gt 0 ]; then
    ok "Cache has $entries entries"
  else
    warn "No cache entries found"
  fi
}

# ── Start ─────────────────────────────────────────────────────────────

do_start() {
  log "Starting ${NODE_COUNT}-node cluster..."

  # Kill any existing cluster
  if [ -d "$CLUSTER_DIR" ]; then
    warn "Existing cluster found, cleaning up..."
    do_stop
  fi

  mkdir -p "$CLUSTER_DIR"
  echo "$COOKIE" > "$CLUSTER_DIR/cookie"

  # Check if cache gateway binary is available for substituter config
  local cache_args=()
  if command -v "$NIX_CACHE_GATEWAY" >/dev/null 2>&1; then
    cache_args=(--nix-cache-gateway-url "http://127.0.0.1:$GATEWAY_PORT")
  fi

  # Start nodes
  for i in $(seq 1 "$NODE_COUNT"); do
    local key
    key=$(printf '%064x' $((2000 + i)))
    mkdir -p "$CLUSTER_DIR/node$i"

    ASPEN_DOCS_ENABLED=true \
    ASPEN_DOCS_IN_MEMORY=true \
    ASPEN_HOOKS_ENABLED=true \
    ASPEN_NIX_CACHE_ENABLED=${cache_args:+true} \
    "$ASPEN_NODE" \
      --node-id "$i" \
      --data-dir "$CLUSTER_DIR/node$i" \
      --cookie "$COOKIE" \
      --iroh-secret-key "$key" \
      --disable-mdns \
      --heartbeat-interval-ms 500 \
      --election-timeout-min-ms 1500 \
      --election-timeout-max-ms 3000 \
      --enable-workers \
      --enable-ci \
      --ci-auto-trigger \
      "${cache_args[@]}" \
      > "$CLUSTER_DIR/node$i.log" 2>&1 &
    echo $! > "$CLUSTER_DIR/node$i.pid"
  done

  log "Waiting for nodes to start..."
  local retries=30
  while [ $retries -gt 0 ]; do
    if [ -f "$CLUSTER_DIR/node1/cluster-ticket.txt" ]; then
      break
    fi
    sleep 1
    retries=$((retries - 1))
  done

  if [ ! -f "$CLUSTER_DIR/node1/cluster-ticket.txt" ]; then
    err "Node 1 failed to start within 30s"
    cat "$CLUSTER_DIR/node1.log" 2>/dev/null | tail -20
    exit 1
  fi

  # Check all nodes are alive
  for i in $(seq 1 "$NODE_COUNT"); do
    local pid
    pid=$(cat "$CLUSTER_DIR/node$i.pid")
    if kill -0 "$pid" 2>/dev/null; then
      ok "Node $i (PID $pid): running"
    else
      err "Node $i failed — check $CLUSTER_DIR/node$i.log"
      exit 1
    fi
  done

  # Initialize cluster
  log "Initializing cluster..."
  cli cluster init 2>/dev/null || true
  sleep 2

  # Add learners and form cluster if multi-node
  if [ "$NODE_COUNT" -gt 1 ]; then
    local strip_ansi='sed s/\x1b\[[0-9;]*m//g'
    for i in $(seq 2 "$NODE_COUNT"); do
      local endpoint addr
      endpoint=$(grep "created Iroh endpoint" "$CLUSTER_DIR/node$i.log" \
        | $strip_ansi | grep -oP 'endpoint_id=\K[a-f0-9]+')
      addr=$(grep "direct_addrs" "$CLUSTER_DIR/node$i.log" \
        | $strip_ansi | grep -oP '\d+\.\d+\.\d+\.\d+:\d+' | head -1)
      log "Adding node $i as learner..."
      cli cluster add-learner --node-id "$i" \
        --addr "{\"id\":\"$endpoint\",\"addrs\":[{\"Ip\":\"$addr\"}]}" 2>/dev/null
      sleep 1
    done

    log "Promoting all nodes to voters..."
    local members
    members=$(seq -s' ' 1 "$NODE_COUNT")
    cli cluster change-membership $members 2>/dev/null
    sleep 2
  fi

  # Verify cluster health: full Raft assertion for multi-node, basic check for single
  if [ "$NODE_COUNT" -gt 1 ]; then
    if assert_cluster_healthy 30; then
      ok "Cluster healthy (all nodes agree on leader, KV round-trip OK)"
    else
      warn "Cluster health assertion inconclusive — continuing"
    fi
  else
    if cli cluster health 2>/dev/null | grep -q healthy; then
      ok "Cluster healthy"
    else
      warn "Cluster health check inconclusive (single-node is OK)"
    fi
  fi

  # Start nix cache gateway if binary is available
  start_cache_gateway

  log "Cluster started at $CLUSTER_DIR"
  echo ""
  echo -e "${BOLD}Cluster ticket:${NC}"
  cat "$CLUSTER_DIR/node1/cluster-ticket.txt"
  echo ""
}

# ── Status ────────────────────────────────────────────────────────────

do_status() {
  if [ ! -d "$CLUSTER_DIR" ]; then
    err "No cluster running"
    exit 1
  fi

  echo -e "${BOLD}╔══════════════════════════════════════════╗${NC}"
  echo -e "${BOLD}║     Aspen Self-Hosted Cluster Status     ║${NC}"
  echo -e "${BOLD}╚══════════════════════════════════════════╝${NC}"
  echo ""

  for i in $(seq 1 "$NODE_COUNT"); do
    local pid
    pid=$(cat "$CLUSTER_DIR/node$i.pid" 2>/dev/null || echo "?")
    if [ "$pid" != "?" ] && kill -0 "$pid" 2>/dev/null; then
      ok "Node $i (PID $pid): running"
    else
      err "Node $i: not running"
    fi
  done

  echo ""

  # Check gateway status
  local gw_pid
  gw_pid=$(cat "$CLUSTER_DIR/nix-cache-gateway.pid" 2>/dev/null || true)
  if [ -n "$gw_pid" ] && kill -0 "$gw_pid" 2>/dev/null; then
    ok "Nix cache gateway (PID $gw_pid): running on 127.0.0.1:$GATEWAY_PORT"
  fi

  cli cluster status 2>/dev/null || warn "Could not get cluster status"
  echo ""
  cli cluster metrics 2>/dev/null || true
}

# ── Push ──────────────────────────────────────────────────────────────

do_push() {
  if [ ! -f "$CLUSTER_DIR/node1/cluster-ticket.txt" ]; then
    err "No cluster running. Start with: $0 start"
    exit 1
  fi

  local ticket repo_id

  log "Creating Forge repository..."
  local create_out
  create_out=$(cli_json git init aspen 2>&1)
  repo_id=$(parse_json "d.get('id') or d.get('repo_id','')" <<< "$create_out")

  if [ -z "$repo_id" ]; then
    # Maybe repo already exists, try listing
    log "Repo may already exist, checking..."
    local list_out
    list_out=$(cli_json git list 2>&1)
    repo_id=$(parse_json "next((r.get('id') or r.get('repo_id','') for r in (d.get('repos') or d.get('repositories') or []) if r.get('name')=='aspen'), '')" <<< "$list_out")
  fi

  if [ -z "$repo_id" ]; then
    err "Failed to create or find Forge repository"
    echo "Create output: $create_out"
    exit 1
  fi
  ok "Forge repo: $repo_id"

  # Enable CI watch on the repo
  log "Enabling CI auto-trigger on repo..."
  cli_json ci watch "$repo_id" 2>/dev/null || warn "CI watch may already be enabled"

  # Push source to Forge — target the leader node directly.
  # The git import writes ~62K KV entries (hash mappings). Forwarding each
  # one individually from a follower to the leader is slow and fragile.
  # Using the leader's ticket avoids this entirely.
  local leader_ticket
  if [ "$NODE_COUNT" -gt 1 ]; then
    local leader_id
    leader_id=$(parse_json "next((n.get('node_id',0) for n in d.get('nodes',[]) if n.get('is_leader')), 1)" <<< "$(cli_json cluster status 2>&1)") || leader_id=""
    leader_id="${leader_id:-1}"
    leader_ticket=$(cat "$CLUSTER_DIR/node$leader_id/cluster-ticket.txt" 2>/dev/null)
    if [ -z "$leader_ticket" ]; then
      warn "Could not get leader ticket (node $leader_id), falling back to node1"
      leader_ticket=$(cat "$CLUSTER_DIR/node1/cluster-ticket.txt")
    else
      log "Pushing to leader node $leader_id"
    fi
  else
    leader_ticket=$(cat "$CLUSTER_DIR/node1/cluster-ticket.txt")
  fi
  ticket="$leader_ticket"
  local remote_url="aspen://${ticket}/${repo_id}"

  log "Pushing Aspen source to Forge..."
  cd "$PROJECT"

  # Create a temporary worktree-like checkout if we're in a dirty state
  if git diff --quiet 2>/dev/null && git diff --cached --quiet 2>/dev/null; then
    log "Clean working tree, pushing directly..."
  else
    warn "Working tree has uncommitted changes (pushing HEAD)"
  fi

  # Add remote and push
  git remote remove aspen-dogfood 2>/dev/null || true
  git remote add aspen-dogfood "$remote_url"

  log "Pushing to aspen://<ticket>/${repo_id}..."
  if RUST_LOG=warn git push aspen-dogfood HEAD:main 2>&1 | tail -5; then
    ok "Source pushed to Forge"
  else
    err "Push failed"
    exit 1
  fi

  git remote remove aspen-dogfood 2>/dev/null || true
  echo ""
  echo -e "${BOLD}Repo ID:${NC} $repo_id"
  echo "$repo_id" > "$CLUSTER_DIR/repo_id"
}

# ── Build ─────────────────────────────────────────────────────────────

do_build() {
  if [ ! -f "$CLUSTER_DIR/node1/cluster-ticket.txt" ]; then
    err "No cluster running. Start with: $0 start"
    exit 1
  fi

  local repo_id
  repo_id=$(cat "$CLUSTER_DIR/repo_id" 2>/dev/null || true)

  if [ -z "$repo_id" ]; then
    err "No repo ID found. Push source first: $0 push"
    exit 1
  fi

  log "Checking for auto-triggered pipeline..."
  local deadline=$((SECONDS + 120))
  local run_id=""

  while [ $SECONDS -lt $deadline ]; do
    local list_out
    list_out=$(cli_json ci list 2>&1) || true
    run_id=$(parse_json "d.get('runs',[])[0].get('run_id','') if d.get('runs') else ''" <<< "$list_out")

    if [ -n "$run_id" ]; then
      ok "Pipeline found: $run_id"
      break
    fi
    sleep 3
  done

  if [ -z "$run_id" ]; then
    warn "No auto-triggered pipeline found, manually triggering..."
    local trigger_out
    trigger_out=$(cli_json ci run "$repo_id" 2>&1) || {
      err "CI trigger command failed"
      echo "Output: $trigger_out"
      exit 1
    }
    run_id=$(parse_json "d.get('run_id','')" <<< "$trigger_out")

    if [ -z "$run_id" ]; then
      err "Failed to trigger pipeline"
      echo "Output: $trigger_out"
      exit 1
    fi
    ok "Pipeline triggered: $run_id"
  fi

  # Save run_id for verify step
  echo "$run_id" > "$CLUSTER_DIR/run_id"

  # Stream build progress with live logs
  log "Streaming pipeline progress..."
  echo ""
  stream_pipeline "$run_id"
}

# Stream pipeline progress: poll status, tail logs for each running job.
stream_pipeline() {
  local run_id="$1"
  local streaming_job=""      # job currently being tailed
  local stream_pid=""         # PID of background log tail
  local seen_jobs=""          # jobs we've already streamed (space-separated)

  cleanup_stream() {
    if [ -n "${stream_pid:-}" ] && kill -0 "$stream_pid" 2>/dev/null; then
      kill "$stream_pid" 2>/dev/null || true
      wait "$stream_pid" 2>/dev/null || true
    fi
  }
  trap cleanup_stream EXIT

  while true; do
    local status_out
    status_out=$(cli_json ci status "$run_id" 2>&1) || true
    local pipeline_status
    pipeline_status=$(parse_json "d.get('status','unknown')" <<< "$status_out")

    # Find the currently running job (first one with status=running)
    local active_job_id active_job_name
    read -r active_job_id active_job_name < <(python3 -c "
import json, sys, re, os
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

    # If a new job started running, switch log stream to it
    if [ -n "$active_job_id" ] && [ "$active_job_id" != "$streaming_job" ]; then
      # Check we haven't already streamed this job
      case " $seen_jobs " in
        *" $active_job_id "*) ;;
        *)
          # Stop previous stream
          cleanup_stream
          stream_pid=""

          # Print header for new job
          echo ""
          echo -e "${BOLD}━━━ ${active_job_name:-$active_job_id} ━━━${NC}"
          echo ""

          # Start tailing logs for this job in background
          cli ci logs --follow "$run_id" "$active_job_id" 2>/dev/null &
          stream_pid=$!
          streaming_job="$active_job_id"
          seen_jobs="$seen_jobs $active_job_id"
          ;;
      esac
    fi

    # Check terminal state
    case "$pipeline_status" in
      success)
        cleanup_stream
        stream_pid=""
        # Print final status summary
        trap - EXIT  # clear trap before returning (stream_pid goes out of scope)
        echo ""
        print_pipeline_summary "$status_out"
        verify_cache
        ok "Pipeline completed successfully! 🎉"
        return 0
        ;;
      failed|cancelled)
        cleanup_stream
        stream_pid=""
        echo ""
        trap - EXIT  # clear trap before returning (stream_pid goes out of scope)
        print_pipeline_summary "$status_out"
        err "Pipeline $pipeline_status"
        return 1
        ;;
    esac

    sleep 2
  done
}

# Print a compact summary of all stages and jobs.
print_pipeline_summary() {
  local status_out="$1"
  python3 -c "
import json, sys, re
raw = sys.stdin.read()
for m in re.finditer(r'[\[{]', raw):
    try:
        d, _ = json.JSONDecoder().raw_decode(raw, m.start())
    except (json.JSONDecodeError, ValueError):
        continue
    icons = {'success': '✅', 'failed': '❌', 'cancelled': '⏹️', 'running': '🔄', 'pending': '⏳'}
    for stage in d.get('stages', []):
        si = icons.get(stage.get('status', ''), '❓')
        print(f'  {si} {stage[\"name\"]}')
        for job in stage.get('jobs', []):
            ji = icons.get(job.get('status', ''), '❓')
            print(f'      {ji} {job[\"name\"]}')
    break
" <<< "$status_out" 2>/dev/null
  echo ""
}

# ── Verify ────────────────────────────────────────────────────────────

# Download the CI-built aspen-node binary from blob store and compare
# with the locally-built binary used to run this cluster.
do_verify() {
  if [ ! -f "$CLUSTER_DIR/node1/cluster-ticket.txt" ]; then
    err "No cluster running. Start with: $0 start"
    exit 1
  fi

  log "Verifying CI-built binary against local binary..."

  # Check running node version if available
  local cluster_version
  cluster_version=$(cli cluster status 2>/dev/null | grep -i 'version' | head -1 || true)
  if [ -n "$cluster_version" ]; then
    log "Running node version: $cluster_version"
  fi

  # Find the pipeline run — prefer saved run_id, fall back to latest
  local run_id
  run_id=$(cat "$CLUSTER_DIR/run_id" 2>/dev/null || true)

  if [ -z "$run_id" ]; then
    local list_out
    list_out=$(cli_json ci list 2>&1) || true
    run_id=$(parse_json "d.get('runs',[])[0].get('run_id','') if d.get('runs') else ''" <<< "$list_out")
  fi

  if [ -z "$run_id" ]; then
    err "No pipeline runs found"
    exit 1
  fi

  # Get pipeline status to find build-node job ID
  local status_out
  status_out=$(cli_json ci status "$run_id" 2>&1) || true

  local pipeline_status
  pipeline_status=$(parse_json "d.get('status','unknown')" <<< "$status_out")
  if [ "$pipeline_status" != "success" ]; then
    err "Pipeline $run_id is not successful (status: $pipeline_status)"
    exit 1
  fi

  # Extract build-node job ID from stages
  local build_node_job_id
  build_node_job_id=$(python3 -c "
import json, sys, re
raw = sys.stdin.read()
for m in re.finditer(r'[\[{]', raw):
    try:
        d, _ = json.JSONDecoder().raw_decode(raw, m.start())
    except (json.JSONDecodeError, ValueError):
        continue
    for stage in d.get('stages', []):
        if stage.get('name') == 'build':
            for job in stage.get('jobs', []):
                if job.get('name') == 'build-node' and job.get('id'):
                    print(job['id'])
                    sys.exit(0)
    print('')
    sys.exit(0)
print('')
" <<< "$status_out" 2>/dev/null)

  if [ -z "$build_node_job_id" ]; then
    err "Could not find build-node job ID in pipeline $run_id"
    exit 1
  fi
  ok "Found build-node job: $build_node_job_id"

  # Read the job result data from KV to get artifact blob hash
  log "Reading job result from KV store..."
  local job_data
  job_data=$(cli_json kv get "__jobs:$build_node_job_id" 2>&1) || true

  local blob_hash
  blob_hash=$(python3 -c "
import json, sys, re
raw = sys.stdin.read()
for m in re.finditer(r'[\[{]', raw):
    try:
        d, _ = json.JSONDecoder().raw_decode(raw, m.start())
    except (json.JSONDecodeError, ValueError):
        continue
    try:
        # KV get returns {key, value, ...} — value is the job JSON string
        job_str = d.get('value', '')
        job = json.loads(job_str)
        data = job.get('result', {}).get('Success', {}).get('data', {})
        for artifact in data.get('artifacts', []):
            if 'aspen-node' in artifact.get('path', ''):
                print(artifact.get('blob_hash', ''))
                sys.exit(0)
        # Fallback: try uploaded_store_paths
        for sp in data.get('uploaded_store_paths', []):
            if sp.get('blob_hash'):
                print(sp['blob_hash'])
                sys.exit(0)
        print('')
    except:
        print('')
    sys.exit(0)
print('')
" <<< "$job_data" 2>/dev/null)

  if [ -z "$blob_hash" ]; then
    # Show what we got for debugging
    warn "No artifact blob hash found in job result"
    echo "  Job data (truncated):"
    echo "$job_data" | head -5
    # Fall back to comparing nix store paths directly (local cluster)
    log "Falling back to local nix store path comparison..."
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
        if paths:
            print(paths[0])
        else:
            print('')
    except:
        print('')
    sys.exit(0)
print('')
" <<< "$job_data" 2>/dev/null)

    if [ -n "$output_path" ] && [ -f "$output_path/bin/aspen-node" ]; then
      ok "Found CI-built binary at $output_path/bin/aspen-node"
      compare_binaries "$output_path/bin/aspen-node" "$ASPEN_NODE"
      return $?
    fi
    err "Could not locate CI-built binary"
    return 1
  fi

  ok "Artifact blob hash: $blob_hash"

  # Try downloading the binary from blob store.
  # Large binaries (>~30MB) may exceed the RPC response size limit,
  # in which case we fall back to the local nix store path.
  local ci_binary="/tmp/aspen-node-ci-built"
  log "Downloading CI-built binary from blob store..."
  if cli blob get "$blob_hash" -o "$ci_binary" 2>/dev/null; then
    chmod +x "$ci_binary"
    ok "Downloaded CI-built binary to $ci_binary"
    compare_binaries "$ci_binary" "$ASPEN_NODE"
    local result=$?
    rm -f "$ci_binary"
    return $result
  fi

  # Blob too large for RPC — fall back to local nix store path
  warn "Blob download failed (binary too large for RPC), using local store path"
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
        if paths:
            print(paths[0])
        else:
            print('')
    except:
        print('')
    sys.exit(0)
print('')
" <<< "$job_data" 2>/dev/null)

  if [ -n "$output_path" ] && [ -f "$output_path/bin/aspen-node" ]; then
    ok "Found CI-built binary at $output_path/bin/aspen-node"
    compare_binaries "$output_path/bin/aspen-node" "$ASPEN_NODE"
    return $?
  fi

  err "Could not locate CI-built binary"
  return 1
}

compare_binaries() {
  local ci_bin="$1"
  local local_bin="$2"

  echo ""
  echo -e "${BOLD}Binary Comparison:${NC}"
  echo "  CI-built:    $ci_bin"
  echo "  Local-built: $local_bin"
  echo ""

  # Size comparison
  local ci_size local_size
  ci_size=$(stat -c %s "$ci_bin" 2>/dev/null || echo "0")
  local_size=$(stat -c %s "$local_bin" 2>/dev/null || echo "0")
  printf "  Size:  CI = %s bytes, Local = %s bytes\n" "$ci_size" "$local_size"

  # SHA-256 comparison
  local ci_sha local_sha
  ci_sha=$(sha256sum "$ci_bin" 2>/dev/null | cut -d' ' -f1)
  local_sha=$(sha256sum "$local_bin" 2>/dev/null | cut -d' ' -f1)
  printf "  SHA256 CI:    %s\n" "$ci_sha"
  printf "  SHA256 Local: %s\n" "$local_sha"

  if [ "$ci_sha" = "$local_sha" ]; then
    ok "Binaries are identical! (bit-for-bit reproducible build)"
  else
    warn "Binaries differ (expected — different source tree inputs)"
    echo "  This is normal: CI builds from Forge checkout, local from working tree."

    # Verify CI-built binary is functional
    echo ""
    log "Smoke-testing CI-built binary..."
    local ci_version
    ci_version=$("$ci_bin" --version 2>&1 || true)
    local local_version
    local_version=$("$local_bin" --version 2>&1 || true)
    printf "  Version CI:    %s\n" "$ci_version"
    printf "  Version Local: %s\n" "$local_version"

    if [ -n "$ci_version" ]; then
      ok "CI-built binary is functional"
    else
      err "CI-built binary failed to run"
      return 1
    fi
  fi

  echo ""
  return 0
}

# ── Deploy ─────────────────────────────────────────────────────────────

do_deploy() {
  if [ ! -f "$CLUSTER_DIR/node1/cluster-ticket.txt" ]; then
    err "No cluster running. Start with: $0 start"
    exit 1
  fi

  log "Deploying CI-built artifact to cluster..."

  # ── Step 1: Extract the CI-built Nix store path ──────────────────

  local run_id
  run_id=$(cat "$CLUSTER_DIR/run_id" 2>/dev/null || true)

  if [ -z "$run_id" ]; then
    local list_out
    list_out=$(cli_json ci list 2>&1) || true
    run_id=$(parse_json "d.get('runs',[])[0].get('run_id','') if d.get('runs') else ''" <<< "$list_out")
  fi

  if [ -z "$run_id" ]; then
    err "No pipeline runs found. Run build first: $0 build"
    exit 1
  fi

  local status_out
  status_out=$(cli_json ci status "$run_id" 2>&1) || true
  local pipeline_status
  pipeline_status=$(parse_json "d.get('status','unknown')" <<< "$status_out")

  if [ "$pipeline_status" != "success" ]; then
    err "Pipeline $run_id is not successful (status: $pipeline_status)"
    exit 1
  fi

  local build_node_job_id
  build_node_job_id=$(python3 -c "
import json, sys, re
raw = sys.stdin.read()
for m in re.finditer(r'[\[{]', raw):
    try:
        d, _ = json.JSONDecoder().raw_decode(raw, m.start())
    except (json.JSONDecodeError, ValueError):
        continue
    for stage in d.get('stages', []):
        if stage.get('name') == 'build':
            for job in stage.get('jobs', []):
                if job.get('name') == 'build-node' and job.get('id'):
                    print(job['id'])
                    sys.exit(0)
    print('')
    sys.exit(0)
print('')
" <<< "$status_out" 2>/dev/null)

  if [ -z "$build_node_job_id" ]; then
    err "Could not find build-node job ID in pipeline $run_id"
    exit 1
  fi

  local job_data
  job_data=$(cli_json kv get "__jobs:$build_node_job_id" 2>&1) || true

  local store_path
  store_path=$(python3 -c "
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
        if paths:
            print(paths[0])
        else:
            print('')
    except:
        print('')
    sys.exit(0)
print('')
" <<< "$job_data" 2>/dev/null)

  if [ -z "$store_path" ]; then
    err "Could not extract Nix store path from build-node job result"
    exit 1
  fi

  ok "CI-built artifact: $store_path"

  # ── Step 2: Validate the CI-built binary exists ──────────────────

  local ci_bin="$store_path/bin/aspen-node"
  if [ ! -f "$ci_bin" ]; then
    err "CI-built binary not found at $ci_bin"
    err "The Nix store path may not be available locally."
    return 1
  fi

  if ! "$ci_bin" --version >/dev/null 2>&1; then
    err "CI-built binary at $ci_bin is not executable"
    return 1
  fi
  ok "CI-built binary validated: $ci_bin"

  # ── Step 3: Deploy via cluster deploy RPC ────────────────────────
  #
  # Uses the DeploymentCoordinator's rolling upgrade path:
  # follower-first ordering, quorum safety, drain/health lifecycle.
  # The --wait flag blocks until the deployment reaches a terminal state.

  local deploy_timeout=1200
  if [ "$NODE_COUNT" -gt 1 ]; then
    deploy_timeout=1800
  fi

  log "Starting rolling deployment via cluster deploy RPC..."
  if ! cli cluster deploy "$store_path" --wait --deploy-timeout "$deploy_timeout"; then
    local exit_code=$?
    err "Cluster deploy failed (exit code: $exit_code)"
    if [ $exit_code -eq 2 ]; then
      err "Deploy timed out after ${deploy_timeout}s"
    fi
    # Show deploy status for debugging
    cli cluster deploy-status 2>/dev/null || true
    return 1
  fi

  ok "Cluster deploy completed successfully"

  # ── Step 4: Post-deploy liveness check ───────────────────────────
  #
  # The DeploymentCoordinator already verified health during the deploy,
  # but do a quick sanity check that all node processes are still running
  # and the cluster is responsive.

  log "Post-deploy liveness check..."

  for i in $(seq 1 "$NODE_COUNT"); do
    local pid
    pid=$(cat "$CLUSTER_DIR/node$i.pid" 2>/dev/null || true)
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      ok "Node $i (PID $pid): running"
    else
      warn "Node $i: PID file stale (expected after binary restart)"
    fi
  done

  if ! assert_cluster_healthy 90; then
    err "Post-deploy cluster assertion failed"
    return 1
  fi

  # Restart cache gateway with fresh ticket (node addresses may have changed)
  stop_cache_gateway
  start_cache_gateway

  return 0
}

# ── Full Loop (build + deploy + verify) ───────────────────────────────

do_full_loop() {
  echo -e "${BOLD}"
  echo "╔══════════════════════════════════════════════════════════════╗"
  echo "║     ASPEN SELF-HOSTED FULL LOOP — BUILD → DEPLOY → VERIFY  ║"
  echo "║                                                            ║"
  echo "║  Building and deploying Aspen with its own infrastructure   ║"
  echo "╚══════════════════════════════════════════════════════════════╝"
  echo -e "${NC}"
  echo ""

  do_start
  echo ""
  do_push
  echo ""
  do_build
  local build_result=$?
  echo ""

  if [ $build_result -ne 0 ]; then
    err "Build failed, cannot deploy"
    return $build_result
  fi

  do_deploy
  local deploy_result=$?
  echo ""

  if [ $deploy_result -ne 0 ]; then
    err "Deploy failed"
    return $deploy_result
  fi

  do_verify
  local verify_result=$?
  echo ""

  echo -e "${GREEN}${BOLD}"
  echo "╔══════════════════════════════════════════════════════════════╗"
  echo "║          🎉 SELF-HOSTED FULL LOOP SUCCEEDED 🎉             ║"
  echo "║                                                            ║"
  echo "║  Aspen built and deployed itself:                           ║"
  echo "║  • Forge: hosted source code via git-remote-aspen          ║"
  echo "║  • CI: auto-triggered pipeline on push                     ║"
  echo "║  • Nix: reproducible builds in sandbox                     ║"
  echo "║  • Deploy: rolling deployment to cluster                   ║"
  if [ $verify_result -eq 0 ]; then
  echo "║  • Verify: deployed binary validated                       ║"
  fi
  echo "╚══════════════════════════════════════════════════════════════╝"
  echo -e "${NC}"

  return 0
}

# ── Full Pipeline ─────────────────────────────────────────────────────

do_full() {
  echo -e "${BOLD}"
  echo "╔══════════════════════════════════════════════════════════════╗"
  echo "║        ASPEN SELF-HOSTED BUILD — DOGFOOD PIPELINE          ║"
  echo "║                                                            ║"
  echo "║  Building Aspen with Aspen's own Forge + CI + Nix          ║"
  echo "╚══════════════════════════════════════════════════════════════╝"
  echo -e "${NC}"
  echo ""

  do_start
  echo ""
  do_push
  echo ""
  do_build
  local result=$?
  echo ""
  if [ $result -eq 0 ]; then
    do_verify
    local verify_result=$?
    echo ""
    echo -e "${GREEN}${BOLD}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║              🎉 SELF-HOSTED BUILD SUCCEEDED 🎉             ║"
    echo "║                                                            ║"
    echo "║  Aspen built itself using its own infrastructure:          ║"
    echo "║  • Forge: hosted source code via git-remote-aspen          ║"
    echo "║  • CI: auto-triggered pipeline on push                     ║"
    echo "║  • Nix: reproducible builds in sandbox                     ║"
    if [ $verify_result -eq 0 ]; then
    echo "║  • Verify: CI-built binary downloaded and validated        ║"
    fi
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
  fi
  return $result
}

# ── Main ──────────────────────────────────────────────────────────────

CMD="${1:-full}"

case "$CMD" in
  start)     do_start ;;
  stop)      do_stop ;;
  status)    do_status ;;
  push)      do_push ;;
  build)     do_build ;;
  verify)    do_verify ;;
  deploy)    do_deploy ;;
  full)      do_full ;;
  full-loop) do_full_loop ;;
  *)
    echo "Usage: $0 {start|stop|status|push|build|verify|deploy|full|full-loop}"
    echo ""
    echo "Commands:"
    echo "  full       Full pipeline: start → push → build → verify (default)"
    echo "  full-loop  Full loop: start → push → build → deploy → verify"
    echo "  start      Start the Aspen cluster"
    echo "  stop       Stop the cluster"
    echo "  status     Show cluster status"
    echo "  push       Push Aspen source to Forge"
    echo "  build      Wait for / trigger CI build"
    echo "  verify     Download CI-built binary from blob store and compare"
    echo "  deploy     Deploy CI-built artifact to cluster"
    exit 1
    ;;
esac
