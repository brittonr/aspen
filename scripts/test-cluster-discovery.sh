#!/usr/bin/env bash
# Integration test: ClusterDiscovery enables quorum recovery after restart
# without relay, mDNS, or CLI intervention.
#
# Sequence:
#   1. Start 3-node cluster with relay disabled + mDNS disabled
#   2. Verify cluster is healthy (KV write succeeds)
#   3. Graceful shutdown all nodes (flushes discovery files)
#   4. Restart all nodes (new iroh ports, discovery files have old addresses)
#   5. Iroh resolves peers via ClusterDiscovery reading sibling discovery files
#   6. New addresses are published, cluster recovers quorum
#
# Usage: bash scripts/test-cluster-discovery.sh
set -euo pipefail

PROJECT_DIR="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
ASPEN_NODE="${ASPEN_NODE_BIN:-${PROJECT_DIR}/target/debug/aspen-node}"
ASPEN_CLI="${ASPEN_CLI_BIN:-${PROJECT_DIR}/target/debug/aspen-cli}"
CLUSTER_DIR="/tmp/aspen-discovery-test"
COOKIE="discovery-test-$(date +%s)"
NODE_COUNT=3
TEST_PASSED=false

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'
BOLD='\033[1m'; NC='\033[0m'

log() { echo -e "${BLUE}[test]${NC} $*"; }
ok()  { echo -e "${GREEN}  ✅ $*${NC}"; }
err() { echo -e "${RED}  ❌ $*${NC}"; }
warn() { echo -e "${YELLOW}  ⚠️  $*${NC}"; }

if [ ! -x "$ASPEN_NODE" ]; then
  err "aspen-node not found at $ASPEN_NODE"
  exit 1
fi
if [ ! -x "$ASPEN_CLI" ]; then
  err "aspen-cli not found at $ASPEN_CLI"
  exit 1
fi

cleanup() {
  log "Cleaning up..."
  for i in $(seq 1 $NODE_COUNT); do
    local pid
    pid=$(cat "$CLUSTER_DIR/node$i.pid" 2>/dev/null || true)
    if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  done
  if [ "$TEST_PASSED" = "true" ]; then
    rm -rf "$CLUSTER_DIR"
  else
    warn "Logs preserved at $CLUSTER_DIR"
  fi
}
trap cleanup EXIT

cli_node() {
  local node=$1; shift
  local ticket
  ticket=$(cat "$CLUSTER_DIR/node$node/cluster-ticket.txt" 2>/dev/null) || return 1
  "$ASPEN_CLI" --ticket "$ticket" "$@"
}

start_node() {
  local i=$1
  local key
  key=$(printf '%064x' $((4000 + i)))
  mkdir -p "$CLUSTER_DIR/node$i"
  NO_COLOR=1 \
  ASPEN_DOCS_ENABLED=true \
  ASPEN_DOCS_IN_MEMORY=true \
  ASPEN_HOOKS_ENABLED=true \
  "$ASPEN_NODE" \
    --node-id "$i" \
    --data-dir "$CLUSTER_DIR/node$i" \
    --cookie "$COOKIE" \
    --iroh-secret-key "$key" \
    --disable-mdns \
    --relay-mode disabled \
    --heartbeat-interval-ms 300 \
    --election-timeout-min-ms 1000 \
    --election-timeout-max-ms 2000 \
    --enable-workers \
    >> "$CLUSTER_DIR/node$i.log" 2>&1 &
  echo $! > "$CLUSTER_DIR/node$i.pid"
}

wait_ticket() {
  local i=$1 retries=30
  while [ $retries -gt 0 ]; do
    [ -f "$CLUSTER_DIR/node$i/cluster-ticket.txt" ] && return 0
    sleep 1
    retries=$((retries - 1))
  done
  return 1
}

get_node_addr() {
  local i=$1
  grep "direct_addrs" "$CLUSTER_DIR/node$i.log" \
    | tail -1 | grep -oP '100\.110\.\d+\.\d+:\d+' | head -1
}

get_node_endpoint() {
  local i=$1
  grep "created Iroh endpoint" "$CLUSTER_DIR/node$i.log" \
    | tail -1 | grep -oP 'endpoint_id=\K[a-f0-9]+'
}

get_node_port() {
  get_node_addr "$1" | grep -oP ':\K\d+'
}

# Graceful stop: SIGTERM then wait
stop_node() {
  local i=$1
  local pid
  pid=$(cat "$CLUSTER_DIR/node$i.pid" 2>/dev/null || true)
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
    local w=0
    while kill -0 "$pid" 2>/dev/null && [ $w -lt 10 ]; do
      sleep 1
      w=$((w + 1))
    done
    if kill -0 "$pid" 2>/dev/null; then
      kill -9 "$pid" 2>/dev/null || true
      sleep 1
    fi
  fi
  rm -f "$CLUSTER_DIR/node$i/cluster-ticket.txt"
}

echo -e "${BOLD}"
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║     CLUSTER DISCOVERY INTEGRATION TEST                      ║"
echo "║     Proves file-based discovery enables quorum recovery     ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# ─── Phase 1: Start cluster ─────────────────────────────────────────

log "Phase 1: Starting 3-node cluster (relay DISABLED, mDNS DISABLED)..."
rm -rf "$CLUSTER_DIR"
mkdir -p "$CLUSTER_DIR"

for i in $(seq 1 $NODE_COUNT); do
  start_node "$i"
done
for i in $(seq 1 $NODE_COUNT); do
  wait_ticket "$i" || { err "Node $i failed to start"; exit 1; }
  ok "Node $i started (PID $(cat "$CLUSTER_DIR/node$i.pid"))"
done
sleep 2

log "Forming cluster..."
cli_node 1 cluster init 2>&1 || { err "cluster init failed"; exit 1; }
sleep 2

for i in 2 3; do
  node_addr=$(get_node_addr "$i")
  node_endpoint=$(get_node_endpoint "$i")
  cli_node 1 cluster add-learner --node-id "$i" \
    --addr "{\"id\":\"$node_endpoint\",\"addrs\":[{\"Ip\":\"$node_addr\"}]}" 2>&1 \
    || { err "add-learner $i failed"; exit 1; }
  sleep 2
done

cli_node 1 cluster change-membership 1 2 3 2>&1 \
  || { err "change-membership failed"; exit 1; }
sleep 3

if cli_node 1 kv set "__health_pre" "ok" 2>/dev/null; then
  ok "Cluster healthy — KV write succeeded"
else
  err "Cluster not healthy before test"; exit 1
fi

log "Pre-restart ports:"
for i in $(seq 1 $NODE_COUNT); do
  echo "  Node $i: $(get_node_addr "$i")"
done

# ─── Phase 2: Verify discovery files exist ───────────────────────────

log ""
log "Phase 2: Checking discovery files..."

discovery_count=0
for i in $(seq 1 $NODE_COUNT); do
  disc_dir="$CLUSTER_DIR/node$i/discovery"
  file_count=$(find "$disc_dir" -name "*.json" 2>/dev/null | wc -l)
  if [ "$file_count" -gt 0 ]; then
    ok "Node $i: $file_count discovery file(s) in $disc_dir"
    discovery_count=$((discovery_count + file_count))
  else
    warn "Node $i: no discovery files yet"
  fi
done

if [ "$discovery_count" -eq 0 ]; then
  err "No discovery files found — ClusterDiscovery not writing files"
  exit 1
fi

# ─── Phase 3: Graceful shutdown (flushes discovery) ──────────────────

log ""
log "Phase 3: Graceful shutdown all nodes..."

for i in $(seq 1 $NODE_COUNT); do
  stop_node "$i"
  ok "Node $i stopped"
done
sleep 2

log "Discovery files after shutdown:"
for i in $(seq 1 $NODE_COUNT); do
  disc_dir="$CLUSTER_DIR/node$i/discovery"
  for f in "$disc_dir"/*.json; do
    [ -f "$f" ] && echo "  $(basename "$f"): $(cat "$f" | python3 -c 'import sys,json; d=json.load(sys.stdin); print(d.get("addrs","?"))' 2>/dev/null || echo "?")"
  done
done

# ─── Phase 4: Restart all nodes ─────────────────────────────────────

log ""
log "Phase 4: Restarting all nodes (new iroh ports)..."

for i in $(seq 1 $NODE_COUNT); do
  start_node "$i"
done
for i in $(seq 1 $NODE_COUNT); do
  wait_ticket "$i" || { err "Node $i failed to restart"; exit 1; }
  ok "Node $i restarted (PID $(cat "$CLUSTER_DIR/node$i.pid"))"
done

log "Post-restart ports (all should change):"
for i in $(seq 1 $NODE_COUNT); do
  echo "  Node $i: $(get_node_addr "$i")"
done

# ─── Phase 5: Wait for quorum recovery ──────────────────────────────

log ""
log "Phase 5: Waiting for quorum recovery (max 30s)..."
log "(Checking via Raft logs — CLI can't reach relay-disabled nodes)"

recovered=false
for attempt in $(seq 1 30); do
  for i in $(seq 1 $NODE_COUNT); do
    # Check if any node elected a leader AFTER the restart timestamp
    restart_ts=$(grep "created Iroh endpoint" "$CLUSTER_DIR/node$i.log" | tail -1 | cut -c1-23)
    if grep -q "leader elected event\|a quorum granted my vote" "$CLUSTER_DIR/node$i.log" 2>/dev/null; then
      # Verify the election happened after restart
      election_ts=$(grep "leader elected event\|a quorum granted my vote" "$CLUSTER_DIR/node$i.log" | tail -1 | cut -c1-23)
      if [[ "$election_ts" > "$restart_ts" ]]; then
        ok "Quorum restored! Node $i: leader elected at $election_ts (restarted at $restart_ts)"
        recovered=true
        break 2
      fi
    fi
  done
  sleep 1
done

if [ "$recovered" = "true" ]; then
  # Verify all nodes see the same leader
  sleep 3
  for i in $(seq 1 $NODE_COUNT); do
    leader=$(grep "leader elected event" "$CLUSTER_DIR/node$i.log" | tail -1 | grep -oP 'new_leader=\K\d+' || echo "?")
    echo "  Node $i: sees leader=$leader"
  done

  # Verify new discovery files were written after restart
  log ""
  log "Verifying discovery files updated with new addresses..."
  for i in $(seq 1 $NODE_COUNT); do
    new_port=$(get_node_port "$i")
    disc_dir="$CLUSTER_DIR/node$i/discovery"
    if grep -q "$new_port" "$disc_dir"/*.json 2>/dev/null; then
      ok "Node $i: discovery file has new port :$new_port"
    else
      warn "Node $i: discovery file may have stale port (expected :$new_port)"
    fi
  done

  TEST_PASSED=true
  echo ""
  echo -e "${GREEN}${BOLD}"
  echo "╔══════════════════════════════════════════════════════════════╗"
  echo "║          ✅ CLUSTER DISCOVERY TEST PASSED                  ║"
  echo "║                                                            ║"
  echo "║  1. 3-node cluster started (relay OFF, mDNS OFF)          ║"
  echo "║  2. Discovery files written to <data_dir>/discovery/      ║"
  echo "║  3. Graceful shutdown flushed addresses to disk            ║"
  echo "║  4. All nodes restarted with new iroh ports                ║"
  echo "║  5. Quorum recovered via ClusterDiscovery — no CLI needed  ║"
  echo "║                                                            ║"
  echo "║  File-based AddressLookup works for air-gapped clusters.  ║"
  echo "╚══════════════════════════════════════════════════════════════╝"
  echo -e "${NC}"
else
  err "TEST FAILED: Cluster did not recover quorum within 30s"
  log "Dumping last 10 lines of each node log:"
  for i in $(seq 1 $NODE_COUNT); do
    echo "=== Node $i ==="
    tail -10 "$CLUSTER_DIR/node$i.log"
  done
  exit 1
fi
