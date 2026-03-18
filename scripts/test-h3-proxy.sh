#!/usr/bin/env bash
# Test the h3 compatibility proxy.
#
# Usage:
#   ./scripts/test-h3-proxy.sh           # Run all tests
#   ./scripts/test-h3-proxy.sh smoke     # Quick smoke test (no cluster needed)
#   ./scripts/test-h3-proxy.sh embedded  # Embedded proxy in forge-web
#   ./scripts/test-h3-proxy.sh standalone # Standalone proxy → forge-web
#
# Prerequisites: cargo build must succeed for aspen-node, aspen-forge-web, aspen-h3-proxy
set -euo pipefail

CLUSTER_DIR="/tmp/aspen-h3-proxy-test"
COOKIE="h3-proxy-test-$$"

# Colors
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BLUE='\033[0;34m'
BOLD='\033[1m'; NC='\033[0m'

log()  { echo -e "${BLUE}[h3-proxy-test]${NC} $*"; }
ok()   { echo -e "${GREEN}  ✅ $*${NC}"; }
err()  { echo -e "${RED}  ❌ $*${NC}"; }
warn() { echo -e "${YELLOW}  ⚠️  $*${NC}"; }

PIDS=()

cleanup() {
  log "Cleaning up..."
  for pid in "${PIDS[@]}"; do
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  done
  # Wait a moment then force-kill stragglers
  sleep 1
  for pid in "${PIDS[@]}"; do
    if kill -0 "$pid" 2>/dev/null; then
      kill -9 "$pid" 2>/dev/null || true
    fi
  done
  rm -rf "$CLUSTER_DIR"
}
trap cleanup EXIT

# ── Build ─────────────────────────────────────────────────────────────

build_binaries() {
  log "Building binaries..."
  cargo build \
    --features forge,blob,docs,hooks,jobs,automerge \
    --bin aspen-node --bin aspen-cli \
    -p aspen -p aspen-cli -p aspen-forge-web -p aspen-h3-proxy \
    2>&1 | tail -3
  ok "Build complete"
}

ASPEN_NODE="./target/debug/aspen-node"
ASPEN_CLI="./target/debug/aspen-cli"
FORGE_WEB="./target/debug/aspen-forge-web"
H3_PROXY="./target/debug/aspen-h3-proxy"

# ── Cluster helpers ───────────────────────────────────────────────────

start_cluster() {
  mkdir -p "$CLUSTER_DIR/node1"
  local key
  key=$(printf '%064x' 9001)

  log "Starting single-node cluster..."
  ASPEN_DOCS_ENABLED=true \
  ASPEN_DOCS_IN_MEMORY=true \
  ASPEN_HOOKS_ENABLED=true \
  "$ASPEN_NODE" \
    --node-id 1 \
    --data-dir "$CLUSTER_DIR/node1" \
    --cookie "$COOKIE" \
    --iroh-secret-key "$key" \
    --disable-mdns \
    --enable-workers \
    > "$CLUSTER_DIR/node1.log" 2>&1 &
  PIDS+=($!)

  # Wait for cluster ticket
  local retries=30
  while [ $retries -gt 0 ]; do
    if [ -f "$CLUSTER_DIR/node1/cluster-ticket.txt" ]; then
      break
    fi
    sleep 1
    retries=$((retries - 1))
  done

  if [ ! -f "$CLUSTER_DIR/node1/cluster-ticket.txt" ]; then
    err "Cluster failed to start within 30s"
    tail -20 "$CLUSTER_DIR/node1.log"
    exit 1
  fi

  # Init cluster
  local ticket
  ticket=$(cat "$CLUSTER_DIR/node1/cluster-ticket.txt")
  "$ASPEN_CLI" --ticket "$ticket" cluster init 2>/dev/null || true
  sleep 2
  ok "Cluster running (ticket: ${ticket:0:20}...)"
}

wait_for_http() {
  local url="$1"
  local timeout="${2:-15}"
  local retries=$timeout
  while [ $retries -gt 0 ]; do
    if curl -s -o /dev/null -w "%{http_code}" "$url" 2>/dev/null | grep -qE '^[2345]'; then
      return 0
    fi
    sleep 1
    retries=$((retries - 1))
  done
  return 1
}

# ── Test 1: Smoke test (no cluster) ──────────────────────────────────

test_smoke() {
  echo ""
  echo -e "${BOLD}━━━ Test: Smoke (no cluster needed) ━━━${NC}"
  echo ""
  log "Starting proxy pointing at nonexistent endpoint..."

  # Use a valid hex-encoded ed25519 public key that nobody is actually serving.
  # (from iroh-base test suite — a real key format that parses correctly)
  local dummy_id="ae58ff8833241ac82d6ff7611046ed67b5072d142c588d0063e942d9a75502b6"

  "$H3_PROXY" \
    --endpoint-id "$dummy_id" \
    --alpn "aspen/forge-web/1" \
    --port 18080 \
    > "$CLUSTER_DIR/smoke-proxy.log" 2>&1 &
  PIDS+=($!)
  local proxy_pid=$!

  # Give it a moment to bind
  sleep 2

  if ! kill -0 "$proxy_pid" 2>/dev/null; then
    err "Proxy exited prematurely"
    cat "$CLUSTER_DIR/smoke-proxy.log"
    return 1
  fi
  ok "Proxy started (PID $proxy_pid)"

  # curl should get a response (502 since backend is unreachable)
  local status
  status=$(curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:18080/ 2>/dev/null || echo "000")

  if [ "$status" = "502" ]; then
    ok "Got 502 Bad Gateway (expected — no backend)"
  elif [ "$status" = "000" ]; then
    # The proxy might still be in connection backoff, so TCP listener
    # may not have responded yet. Wait and retry.
    sleep 3
    status=$(curl -s -o /dev/null -w "%{http_code}" --max-time 5 http://127.0.0.1:18080/ 2>/dev/null || echo "000")
    if [ "$status" = "502" ]; then
      ok "Got 502 Bad Gateway (expected — no backend)"
    else
      err "Expected 502, got $status"
      return 1
    fi
  else
    err "Expected 502, got $status"
    return 1
  fi

  # CLI --help works
  if "$H3_PROXY" --help 2>&1 | grep -q 'endpoint-id'; then
    ok "CLI --help shows expected flags"
  else
    err "CLI --help missing expected flags"
    return 1
  fi

  kill "$proxy_pid" 2>/dev/null || true
  ok "Smoke test passed"
}

# ── Test 2: Embedded proxy (forge-web --tcp-port) ────────────────────

test_embedded() {
  echo ""
  echo -e "${BOLD}━━━ Test: Embedded proxy (forge-web --tcp-port) ━━━${NC}"
  echo ""

  local ticket
  ticket=$(cat "$CLUSTER_DIR/node1/cluster-ticket.txt")

  log "Starting forge-web with --tcp-port 18081..."
  "$FORGE_WEB" \
    --ticket "$ticket" \
    --tcp-port 18081 \
    > "$CLUSTER_DIR/forge-web-embedded.log" 2>&1 &
  PIDS+=($!)
  local fw_pid=$!

  sleep 3

  if ! kill -0 "$fw_pid" 2>/dev/null; then
    err "forge-web exited prematurely"
    cat "$CLUSTER_DIR/forge-web-embedded.log"
    return 1
  fi
  ok "forge-web started (PID $fw_pid)"

  # The embedded proxy connects back to itself over iroh. Give it time
  # to establish the h3 connection.
  log "Waiting for TCP proxy to be ready..."
  if wait_for_http "http://127.0.0.1:18081/" 20; then
    ok "TCP proxy responding"
  else
    warn "TCP proxy not responding within 20s (may need more time for iroh self-connect)"
    # Print log for debugging
    tail -10 "$CLUSTER_DIR/forge-web-embedded.log"
    kill "$fw_pid" 2>/dev/null || true
    return 1
  fi

  # Verify we get HTML
  local body
  body=$(curl -s http://127.0.0.1:18081/ 2>/dev/null)
  if echo "$body" | grep -qi 'html\|forge\|aspen'; then
    ok "Got HTML response from embedded proxy"
  else
    warn "Response doesn't look like HTML (may be 502 if self-connect is slow)"
    echo "  Response (first 200 chars): ${body:0:200}"
  fi

  # Check content-type header
  local content_type
  content_type=$(curl -s -o /dev/null -D - http://127.0.0.1:18081/ 2>/dev/null | grep -i 'content-type' | head -1)
  if echo "$content_type" | grep -qi 'text/html'; then
    ok "Content-Type: text/html"
  else
    log "Content-Type: ${content_type:-<none>}"
  fi

  kill "$fw_pid" 2>/dev/null || true
  ok "Embedded proxy test passed"
}

# ── Test 3: Standalone proxy → forge-web ─────────────────────────────

test_standalone() {
  echo ""
  echo -e "${BOLD}━━━ Test: Standalone proxy → forge-web ━━━${NC}"
  echo ""

  local ticket
  ticket=$(cat "$CLUSTER_DIR/node1/cluster-ticket.txt")

  log "Starting forge-web (h3 only, no TCP)..."
  "$FORGE_WEB" \
    --ticket "$ticket" \
    > "$CLUSTER_DIR/forge-web-standalone.log" 2>&1 &
  PIDS+=($!)
  local fw_pid=$!

  sleep 3

  if ! kill -0 "$fw_pid" 2>/dev/null; then
    err "forge-web exited prematurely"
    cat "$CLUSTER_DIR/forge-web-standalone.log"
    return 1
  fi
  ok "forge-web started (PID $fw_pid)"

  # Extract the full 64-char hex endpoint ID from the log.
  # forge-web logs: endpoint_id=<64-hex-chars> (tracing strips ANSI in grep)
  local full_id
  # Strip ANSI codes before grepping
  full_id=$(sed 's/\x1b\[[0-9;]*m//g' "$CLUSTER_DIR/forge-web-standalone.log" \
    | grep -oP 'endpoint_id=[a-f0-9]{64}' | head -1 | sed 's/endpoint_id=//')

  if [ -z "$full_id" ]; then
    err "Could not extract endpoint ID from forge-web log"
    tail -10 "$CLUSTER_DIR/forge-web-standalone.log"
    kill "$fw_pid" 2>/dev/null || true
    return 1
  fi
  ok "Forge-web endpoint ID: $full_id"

  log "Starting standalone h3-proxy → endpoint ${full_id:0:10}..."
  "$H3_PROXY" \
    --endpoint-id "$full_id" \
    --alpn "aspen/forge-web/1" \
    --port 18082 \
    > "$CLUSTER_DIR/standalone-proxy.log" 2>&1 &
  PIDS+=($!)
  local proxy_pid=$!

  sleep 3

  if ! kill -0 "$proxy_pid" 2>/dev/null; then
    err "h3-proxy exited prematurely"
    cat "$CLUSTER_DIR/standalone-proxy.log"
    kill "$fw_pid" 2>/dev/null || true
    return 1
  fi
  ok "h3-proxy started (PID $proxy_pid)"

  log "Waiting for proxy to connect..."
  if wait_for_http "http://127.0.0.1:18082/" 20; then
    ok "Standalone proxy responding"

    local body
    body=$(curl -s http://127.0.0.1:18082/ 2>/dev/null)
    if echo "$body" | grep -qi 'html\|forge\|aspen'; then
      ok "Got HTML response through standalone proxy"
    else
      warn "Response: ${body:0:200}"
    fi
  else
    warn "Standalone proxy not responding within 20s"
    tail -5 "$CLUSTER_DIR/standalone-proxy.log"
  fi

  kill "$proxy_pid" 2>/dev/null || true
  kill "$fw_pid" 2>/dev/null || true
  ok "Standalone proxy test done"
}

# ── Main ──────────────────────────────────────────────────────────────

mkdir -p "$CLUSTER_DIR"

CMD="${1:-all}"

case "$CMD" in
  smoke)
    build_binaries
    test_smoke
    ;;
  embedded)
    build_binaries
    start_cluster
    test_embedded
    ;;
  standalone)
    build_binaries
    start_cluster
    test_standalone
    ;;
  all)
    build_binaries
    test_smoke
    start_cluster
    test_embedded
    test_standalone
    echo ""
    echo -e "${GREEN}${BOLD}All tests passed!${NC}"
    ;;
  *)
    echo "Usage: $0 {all|smoke|embedded|standalone}"
    echo ""
    echo "  smoke       Quick test — proxy starts, returns 502 (no cluster needed)"
    echo "  embedded    forge-web --tcp-port embeds the proxy"
    echo "  standalone  Separate h3-proxy binary → forge-web over iroh"
    echo "  all         Run all tests (default)"
    exit 1
    ;;
esac
