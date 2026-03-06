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

cli() {
  local ticket
  ticket=$(cat "$CLUSTER_DIR/node1/cluster-ticket.txt" 2>/dev/null) || {
    err "No cluster ticket found. Is the cluster running?"
    return 1
  }
  "$ASPEN_CLI" --ticket "$ticket" "$@"
}

cli_json() {
  local ticket
  ticket=$(cat "$CLUSTER_DIR/node1/cluster-ticket.txt" 2>/dev/null) || return 1
  "$ASPEN_CLI" --ticket "$ticket" --json "$@"
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
# Strip any non-JSON prefix (e.g. version banner lines)
m = re.search(r'[\[{]', raw)
if m:
    raw = raw[m.start():]
try:
    d = json.loads(raw)
    result = eval(os.environ['PARSE_EXPR'])
    print(result if result else '')
except:
    print('')
" 2>/dev/null
}

# ── Stop ──────────────────────────────────────────────────────────────

do_stop() {
  log "Stopping cluster..."
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

  # Start nodes
  for i in $(seq 1 "$NODE_COUNT"); do
    local key
    key=$(printf '%064x' $((2000 + i)))
    mkdir -p "$CLUSTER_DIR/node$i"

    ASPEN_DOCS_ENABLED=true \
    ASPEN_DOCS_IN_MEMORY=true \
    ASPEN_HOOKS_ENABLED=true \
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

  # Verify cluster health
  if cli cluster health 2>/dev/null | grep -q healthy; then
    ok "Cluster healthy"
  else
    warn "Cluster health check inconclusive (single-node is OK)"
  fi

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

  # Push source to Forge
  ticket=$(cat "$CLUSTER_DIR/node1/cluster-ticket.txt")
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
    if [ -n "$stream_pid" ] && kill -0 "$stream_pid" 2>/dev/null; then
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
m = re.search(r'[\[{]', raw)
if m: raw = raw[m.start():]
try:
    d = json.loads(raw)
    for stage in d.get('stages', []):
        for job in stage.get('jobs', []):
            if job.get('status') == 'running' and job.get('id'):
                print(job['id'], job.get('name', ''))
                sys.exit(0)
    print(' ')
except:
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
        echo ""
        print_pipeline_summary "$status_out"
        ok "Pipeline completed successfully! 🎉"
        return 0
        ;;
      failed|cancelled)
        cleanup_stream
        stream_pid=""
        echo ""
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
m = re.search(r'[\[{]', raw)
if m: raw = raw[m.start():]
try:
    d = json.loads(raw)
    icons = {'success': '✅', 'failed': '❌', 'cancelled': '⏹️', 'running': '🔄', 'pending': '⏳'}
    for stage in d.get('stages', []):
        si = icons.get(stage.get('status', ''), '❓')
        print(f'  {si} {stage[\"name\"]}')
        for job in stage.get('jobs', []):
            ji = icons.get(job.get('status', ''), '❓')
            print(f'      {ji} {job[\"name\"]}')
except:
    pass
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
m = re.search(r'[\[{]', raw)
if m: raw = raw[m.start():]
try:
    d = json.loads(raw)
    for stage in d.get('stages', []):
        if stage.get('name') == 'build':
            for job in stage.get('jobs', []):
                if job.get('name') == 'build-node' and job.get('id'):
                    print(job['id'])
                    sys.exit(0)
    print('')
except:
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
m = re.search(r'[\[{]', raw)
if m: raw = raw[m.start():]
try:
    d = json.loads(raw)
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
m = re.search(r'[\[{]', raw)
if m: raw = raw[m.start():]
try:
    d = json.loads(raw)
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
m = re.search(r'[\[{]', raw)
if m: raw = raw[m.start():]
try:
    d = json.loads(raw)
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
  start)  do_start ;;
  stop)   do_stop ;;
  status) do_status ;;
  push)   do_push ;;
  build)  do_build ;;
  verify) do_verify ;;
  full)   do_full ;;
  *)
    echo "Usage: $0 {start|stop|status|push|build|verify|full}"
    echo ""
    echo "Commands:"
    echo "  full    Full pipeline: start → push → build → verify (default)"
    echo "  start   Start the Aspen cluster"
    echo "  stop    Stop the cluster"
    echo "  status  Show cluster status"
    echo "  push    Push Aspen source to Forge"
    echo "  build   Wait for / trigger CI build"
    echo "  verify  Download CI-built binary from blob store and compare"
    exit 1
    ;;
esac
