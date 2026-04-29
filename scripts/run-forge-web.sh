#!/usr/bin/env bash
# Start an Aspen cluster + Forge web UI accessible at http://localhost:8080
#
# Usage:
#   ./scripts/run-forge-web.sh                # Start locally (localhost only)
#   ./scripts/run-forge-web.sh --tailscale    # Serve over Tailscale (tailnet HTTPS)
#   ./scripts/run-forge-web.sh --funnel       # Serve publicly via Tailscale Funnel
#   ./scripts/run-forge-web.sh stop           # Stop everything
set -euo pipefail

PORT="${FORGE_WEB_PORT:-8080}"
CLUSTER_DIR="/tmp/aspen-forge-web"
COOKIE="forge-web-$$"
TAILSCALE_MODE=""

RED='\033[0;31m'; GREEN='\033[0;32m'; BLUE='\033[0;34m'
BOLD='\033[1m'; NC='\033[0m'

log() { echo -e "${BLUE}[forge-web]${NC} $*"; }
ok()  { echo -e "${GREEN}  ✅ $*${NC}"; }
err() { echo -e "${RED}  ❌ $*${NC}"; }

CLI="./target/debug/aspen-cli"
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)

# Check if port is available before doing expensive work
check_port() {
  if ss -tln 2>/dev/null | grep -q ":${PORT} "; then
    local holder holder_suffix
    holder=$(ss -tlnp 2>/dev/null | grep ":${PORT} " | sed 's/.*users:(("\([^"]*\)".*/\1/' | head -1)
    holder_suffix=""
    if [ -n "$holder" ]; then
      holder_suffix=" by '$holder'"
    fi
    err "Port $PORT already in use$holder_suffix"
    echo -e "  ${BLUE}Fix: FORGE_WEB_PORT=8081 $0${NC}"
    echo -e "  ${BLUE} or: kill the process using port $PORT${NC}"
    exit 1
  fi
}

# Parse flags
for arg in "$@"; do
  case "$arg" in
    --tailscale) TAILSCALE_MODE="serve" ;;
    --funnel)    TAILSCALE_MODE="funnel" ;;
    stop)        ;; # handled below
    *) err "Unknown argument: $arg"; exit 1 ;;
  esac
done

tailscale_teardown() {
  if [ -n "$TAILSCALE_MODE" ]; then
    log "Removing tailscale serve config for port $PORT..."
    tailscale serve reset 2>/dev/null || true
  fi
}

do_stop() {
  log "Stopping..."
  tailscale_teardown
  for pidfile in "$CLUSTER_DIR"/*.pid; do
    [ -f "$pidfile" ] || continue
    pid=$(cat "$pidfile")
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  done
  sleep 1
  rm -rf "$CLUSTER_DIR"
  ok "Stopped"
  exit 0
}

if [ "${1:-}" = "stop" ]; then
  do_stop
fi

# Validate tailscale is usable before spending time on the build
if [ -n "$TAILSCALE_MODE" ]; then
  if ! command -v tailscale >/dev/null 2>&1; then
    err "tailscale CLI not found"
    exit 1
  fi
  if ! tailscale status --self --json >/dev/null 2>&1; then
    err "tailscale is not running or not logged in"
    exit 1
  fi
  TS_HOSTNAME=$(tailscale status --self --json | python3 -c "import json,sys; print(json.load(sys.stdin)['Self']['DNSName'].rstrip('.'))")
  ok "Tailscale: $TS_HOSTNAME (mode: $TAILSCALE_MODE)"
fi

cleanup() {
  log "Shutting down..."
  tailscale_teardown
  for pidfile in "$CLUSTER_DIR"/*.pid; do
    [ -f "$pidfile" ] || continue
    pid=$(cat "$pidfile")
    kill "$pid" 2>/dev/null || true
  done
  sleep 1
  rm -rf "$CLUSTER_DIR"
}
trap cleanup EXIT

cli() { "$CLI" --ticket "$TICKET" --quiet "$@"; }
cli_json() { "$CLI" --ticket "$TICKET" --quiet --json "$@"; }

# shellcheck source=scripts/lib/run-forge-web-demo.sh
source "$SCRIPT_DIR/lib/run-forge-web-demo.sh"

# Parse a JSON field. Usage: parse_field "key" <<< "$json"
parse_field() {
  python3 -c "
import json, sys, re
raw = sys.stdin.read()
for m in re.finditer(r'[\[{]', raw):
    try:
        d = json.loads(raw[m.start():])
        print(d.get('$1', ''))
        break
    except: pass
"
}

# ── Pre-flight checks ────────────────────────────────────────────────

check_port

# ── Build ─────────────────────────────────────────────────────────────

log "Building..."
cargo build \
  --features ci,nix-cli-fallback,docs,hooks,shell-worker,automerge,git-bridge,node-runtime-apps,blob \
  -p aspen --bin aspen-node --bin git-remote-aspen \
  2>&1 | tail -3
cargo build -p aspen-cli --features ci --bin aspen-cli 2>&1 | tail -3
cargo build -p aspen-forge-web --bin aspen-forge-web 2>&1 | tail -3
ok "Build complete"

# ── Start cluster ────────────────────────────────────────────────────

mkdir -p "$CLUSTER_DIR/node1"
KEY=$(printf '%064x' 9001)

log "Starting cluster..."
./target/debug/aspen-node \
  --node-id 1 \
  --data-dir "$CLUSTER_DIR/node1" \
  --cookie "$COOKIE" \
  --iroh-secret-key "$KEY" \
  --disable-mdns \
  --enable-workers \
  --enable-ci \
  --ci-auto-trigger \
  > "$CLUSTER_DIR/node.log" 2>&1 &
echo $! > "$CLUSTER_DIR/node.pid"

for _i in $(seq 1 60); do
  [ -f "$CLUSTER_DIR/node1/cluster-ticket.txt" ] && break
  sleep 1
done

if [ ! -f "$CLUSTER_DIR/node1/cluster-ticket.txt" ]; then
  err "Cluster failed to start"
  tail -20 "$CLUSTER_DIR/node.log"
  exit 1
fi

TICKET=$(cat "$CLUSTER_DIR/node1/cluster-ticket.txt")
cli cluster init 2>/dev/null || true
sleep 2
ok "Cluster running"

# ── Create demo repository ───────────────────────────────────────────

log "Creating demo repository..."

REPO_OUT=$(cli_json git init hello-world 2>&1)
REPO_ID=$(parse_field "id" <<< "$REPO_OUT")

if [ -z "$REPO_ID" ]; then
  err "Failed to create repository"
  echo "$REPO_OUT"
  exit 1
fi
ok "Repository: $REPO_ID"

# Put our built binary on PATH so git can find it
DEBUG_PATH="$(pwd)/target/debug"
export PATH="$DEBUG_PATH:$PATH"

# Push content via git-remote-aspen (git bridge protocol)
if command -v git-remote-aspen >/dev/null 2>&1; then
  log "Pushing demo content via git-remote-aspen..."
  TMPGIT="$CLUSTER_DIR/demo-git"
  mkdir -p "$TMPGIT"
  (
    cd "$TMPGIT" || exit 1
    git init -q || exit 1
    commit_forge_web_demo_repo
  )
  if RUST_LOG=warn git -C "$TMPGIT" push "aspen://${TICKET}/${REPO_ID}" HEAD:main 2>/dev/null; then
    ok "Content pushed"
  else
    log "Push failed (git-remote-aspen may need fixes)"
  fi
else
  log "git-remote-aspen not found — repo created but empty"
  log "To populate: build git-remote-aspen or use nix run .#dogfood-local"
fi

# ── Enable CI for the repository ─────────────────────────────────────

log "Enabling CI for repository..."
if cli ci watch "$REPO_ID" 2>/dev/null; then
  ok "CI watch enabled"
else
  log "CI watch failed (will trigger manually)"
fi
sleep 2

# Check if CI auto-triggered from the replay buffer, otherwise trigger manually
CI_RUNS=$(cli_json ci list --repo-id "$REPO_ID" --limit 1 2>/dev/null || echo "")
if echo "$CI_RUNS" | grep -q 'run_id'; then
  ok "CI pipeline auto-triggered"
else
  log "Triggering CI pipeline manually..."
  TRIGGER_OUT=$(cli_json ci run "$REPO_ID" 2>/dev/null || echo "")
  RUN_ID=$(parse_field "run_id" <<< "$TRIGGER_OUT")
  if [ -n "$RUN_ID" ]; then
    ok "CI pipeline triggered: $RUN_ID"
  else
    log "CI trigger returned no run ID (CI may still start asynchronously)"
  fi
fi

# ── Start forge-web with embedded TCP proxy ──────────────────────────

log "Starting Forge web on port $PORT..."
./target/debug/aspen-forge-web \
  --ticket "$TICKET" \
  --tcp-port "$PORT" \
  > "$CLUSTER_DIR/forge-web.log" 2>&1 &
echo $! > "$CLUSTER_DIR/forge-web.pid"

for _i in $(seq 1 20); do
  if curl -s -o /dev/null --max-time 2 "http://127.0.0.1:$PORT/" 2>/dev/null; then
    break
  fi
  sleep 1
done

if ! curl -s -o /dev/null --max-time 2 "http://127.0.0.1:$PORT/" 2>/dev/null; then
  err "Forge web not responding"
  tail -10 "$CLUSTER_DIR/forge-web.log"
  exit 1
fi
ok "Forge web ready"

# ── Tailscale serve / funnel ─────────────────────────────────────────

if [ -n "$TAILSCALE_MODE" ]; then
  log "Setting up tailscale $TAILSCALE_MODE for port $PORT..."
  tailscale "$TAILSCALE_MODE" --bg "$PORT"
  TS_URL="https://$TS_HOSTNAME"
  ok "Tailscale $TAILSCALE_MODE active"
fi

# ── Open browser ─────────────────────────────────────────────────────

LOCAL_URL="http://localhost:$PORT"

if [ -n "$TAILSCALE_MODE" ]; then
  URL="$TS_URL"
  VISIBILITY="tailnet"
  [ "$TAILSCALE_MODE" = "funnel" ] && VISIBILITY="public internet"

  echo ""
  echo -e "${BOLD}  🌲 Aspen Forge${NC}  (via Tailscale ${TAILSCALE_MODE})"
  echo ""
  echo -e "     ${GREEN}${TS_URL}${NC}"
  echo -e "     ${BLUE}${LOCAL_URL}${NC}  (local)"
  echo ""
  echo -e "  Accessible to: ${BOLD}${VISIBILITY}${NC}"
  echo ""
  echo "  Browse files, commits, issues, patches, and search."
  echo "  Login with a Nostr NIP-07 browser extension."
  echo "  CI auto-triggers on push (.aspen/ci.ncl)."
  echo ""
  echo -e "  Press ${BOLD}Ctrl-C${NC} to stop."
  echo ""
else
  URL="$LOCAL_URL"

  echo ""
  echo -e "${BOLD}  🌲 Aspen Forge${NC}"
  echo ""
  echo -e "     ${GREEN}${URL}${NC}"
  echo ""
  echo "  Browse files, commits, issues, patches, and search."
  echo "  Login with a Nostr NIP-07 browser extension."
  echo "  CI auto-triggers on push (.aspen/ci.ncl)."
  echo ""
  echo -e "  Press ${BOLD}Ctrl-C${NC} to stop."
  echo ""
fi

# Try to open browser
if command -v xdg-open >/dev/null 2>&1; then
  xdg-open "$URL" 2>/dev/null &
elif command -v open >/dev/null 2>&1; then
  open "$URL" 2>/dev/null &
fi

# Wait until Ctrl-C
wait
