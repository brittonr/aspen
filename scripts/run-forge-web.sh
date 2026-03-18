#!/usr/bin/env bash
# Start an Aspen cluster + Forge web UI accessible at http://localhost:8080
#
# Usage:
#   ./scripts/run-forge-web.sh          # Start with a demo repo
#   ./scripts/run-forge-web.sh stop     # Stop everything
set -euo pipefail

PORT="${FORGE_WEB_PORT:-8080}"
CLUSTER_DIR="/tmp/aspen-forge-web"
COOKIE="forge-web-$$"

RED='\033[0;31m'; GREEN='\033[0;32m'; BLUE='\033[0;34m'
BOLD='\033[1m'; NC='\033[0m'

log() { echo -e "${BLUE}[forge-web]${NC} $*"; }
ok()  { echo -e "${GREEN}  ✅ $*${NC}"; }
err() { echo -e "${RED}  ❌ $*${NC}"; }

CLI="./target/debug/aspen-cli"

do_stop() {
  log "Stopping..."
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

cleanup() {
  log "Shutting down..."
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

# ── Build ─────────────────────────────────────────────────────────────

log "Building..."
cargo build \
  --features ci,docs,hooks,shell-worker,automerge,git-bridge \
  --bin aspen-node --bin aspen-cli --bin git-remote-aspen \
  -p aspen -p aspen-cli -p aspen-forge-web \
  2>&1 | tail -3
ok "Build complete"

# ── Start cluster ────────────────────────────────────────────────────

mkdir -p "$CLUSTER_DIR/node1"
KEY=$(printf '%064x' 9001)

log "Starting cluster..."
ASPEN_DOCS_ENABLED=true \
ASPEN_DOCS_IN_MEMORY=true \
ASPEN_HOOKS_ENABLED=true \
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

for i in $(seq 1 60); do
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
    cd "$TMPGIT"
    git init -q

    cat > README.md << 'INNER_MD'
# Hello World

A demo repository hosted on **Aspen Forge**.

## Features

- Decentralized hosting — no single point of failure
- BLAKE3 content-addressed objects
- Raft-consistent refs
- P2P replication via iroh QUIC
- Web UI served over HTTP/3
INNER_MD

    mkdir -p src
    printf 'fn main() {\n    println!("Hello from Aspen Forge!");\n}\n' > src/main.rs
    printf '[package]\nname = "hello"\nversion = "0.1.0"\nedition = "2024"\n' > Cargo.toml

    git add -A && git commit -q -m "Initial commit"
    RUST_LOG=warn git push "aspen://${TICKET}/${REPO_ID}" HEAD:main 2>/dev/null
  ) && ok "Content pushed" || log "Push failed (git-remote-aspen may need fixes)"
else
  log "git-remote-aspen not found — repo created but empty"
  log "To populate: build git-remote-aspen or use nix run .#dogfood-local"
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

# ── Open browser ─────────────────────────────────────────────────────

URL="http://localhost:$PORT"

echo ""
echo -e "${BOLD}╔══════════════════════════════════════════════════╗${NC}"
echo -e "${BOLD}║  🌲 Aspen Forge                                  ║${NC}"
echo -e "${BOLD}║                                                  ║${NC}"
echo -e "${BOLD}║     ${GREEN}$URL${NC}${BOLD}                            ║${NC}"
echo -e "${BOLD}║                                                  ║${NC}"
echo -e "${BOLD}║  Click the repo name to browse files & commits.  ║${NC}"
echo -e "${BOLD}║  Press Ctrl-C to stop.                           ║${NC}"
echo -e "${BOLD}╚══════════════════════════════════════════════════╝${NC}"
echo ""

# Try to open browser
if command -v xdg-open >/dev/null 2>&1; then
  xdg-open "$URL" 2>/dev/null &
elif command -v open >/dev/null 2>&1; then
  open "$URL" 2>/dev/null &
fi

# Wait until Ctrl-C
wait
