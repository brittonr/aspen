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
  repo_id=$(echo "$create_out" | python3 -c "
import json, sys
try:
    d = json.loads(sys.stdin.read())
    print(d.get('id') or d.get('repo_id', ''))
except:
    print('')
" 2>/dev/null)

  if [ -z "$repo_id" ]; then
    # Maybe repo already exists, try listing
    log "Repo may already exist, checking..."
    local list_out
    list_out=$(cli_json git list 2>&1)
    repo_id=$(echo "$list_out" | python3 -c "
import json, sys
try:
    d = json.loads(sys.stdin.read())
    repos = d.get('repos') or d.get('repositories') or []
    for r in repos:
        if r.get('name') == 'aspen':
            print(r.get('id') or r.get('repo_id', ''))
            break
except:
    print('')
" 2>/dev/null)
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
    list_out=$(cli_json ci list 2>&1)
    run_id=$(echo "$list_out" | python3 -c "
import json, sys
try:
    d = json.loads(sys.stdin.read())
    runs = d.get('runs', [])
    if runs:
        print(runs[0].get('run_id', ''))
except:
    print('')
" 2>/dev/null)

    if [ -n "$run_id" ]; then
      ok "Pipeline found: $run_id"
      break
    fi
    sleep 3
  done

  if [ -z "$run_id" ]; then
    warn "No auto-triggered pipeline found, manually triggering..."
    local trigger_out
    trigger_out=$(cli_json ci trigger "$repo_id" 2>&1)
    run_id=$(echo "$trigger_out" | python3 -c "
import json, sys
try:
    d = json.loads(sys.stdin.read())
    print(d.get('run_id', ''))
except:
    print('')
" 2>/dev/null)

    if [ -z "$run_id" ]; then
      err "Failed to trigger pipeline"
      echo "Output: $trigger_out"
      exit 1
    fi
    ok "Pipeline triggered: $run_id"
  fi

  # Wait for pipeline completion
  log "Waiting for pipeline to complete..."
  deadline=$((SECONDS + 7200))  # 2 hour timeout for full build

  while [ $SECONDS -lt $deadline ]; do
    local status_out
    status_out=$(cli_json ci status "$run_id" 2>&1)
    local status
    status=$(echo "$status_out" | python3 -c "
import json, sys
try:
    d = json.loads(sys.stdin.read())
    status = d.get('status', 'unknown')
    print(status)
except:
    print('unknown')
" 2>/dev/null)

    case "$status" in
      success)
        echo ""
        ok "Pipeline completed successfully! 🎉"
        echo ""
        echo -e "${BOLD}Pipeline details:${NC}"
        echo "$status_out" | python3 -m json.tool 2>/dev/null || echo "$status_out"
        return 0
        ;;
      failed|cancelled)
        echo ""
        err "Pipeline $status"
        echo ""
        echo -e "${BOLD}Pipeline details:${NC}"
        echo "$status_out" | python3 -m json.tool 2>/dev/null || echo "$status_out"
        return 1
        ;;
      *)
        printf "\r  ⏳ Status: %-20s (elapsed: %ds)" "$status" "$SECONDS"
        sleep 5
        ;;
    esac
  done

  err "Pipeline timed out after 2 hours"
  return 1
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
    echo -e "${GREEN}${BOLD}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║              🎉 SELF-HOSTED BUILD SUCCEEDED 🎉             ║"
    echo "║                                                            ║"
    echo "║  Aspen built itself using its own infrastructure:          ║"
    echo "║  • Forge: hosted source code via git-remote-aspen          ║"
    echo "║  • CI: auto-triggered pipeline on push                     ║"
    echo "║  • Nix: reproducible builds in sandbox                     ║"
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
  full)   do_full ;;
  *)
    echo "Usage: $0 {start|stop|status|push|build|full}"
    echo ""
    echo "Commands:"
    echo "  full    Full pipeline: start → push → build (default)"
    echo "  start   Start the Aspen cluster"
    echo "  stop    Stop the cluster"
    echo "  status  Show cluster status"
    echo "  push    Push Aspen source to Forge"
    echo "  build   Wait for / trigger CI build"
    exit 1
    ;;
esac
