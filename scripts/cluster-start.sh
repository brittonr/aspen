#!/usr/bin/env bash
# Start a 3-node aspen cluster for testing.
# Usage: ./scripts/cluster-start.sh
set -euo pipefail

cd "$(dirname "$0")/.."
CLUSTER_DIR="/tmp/aspen-test-cluster"

# Kill any existing cluster
if [ -d "$CLUSTER_DIR" ]; then
  for i in 1 2 3; do
    PID=$(cat "$CLUSTER_DIR/node$i.pid" 2>/dev/null || true)
    [ -n "$PID" ] && kill "$PID" 2>/dev/null || true
  done
  sleep 1
  rm -rf "$CLUSTER_DIR"
fi

mkdir -p "$CLUSTER_DIR"

COOKIE="real-cluster-test-$(date +%s)"
echo "$COOKIE" > "$CLUSTER_DIR/cookie"

echo "Starting 3-node cluster..."
for i in 1 2 3; do
  KEY=$(printf '%064x' $((1000 + i)))
  ./target/debug/aspen-node \
    --node-id "$i" \
    --data-dir "$CLUSTER_DIR/node$i" \
    --cookie "$COOKIE" \
    --iroh-secret-key "$KEY" \
    --disable-mdns \
    --heartbeat-interval-ms 500 \
    --election-timeout-min-ms 1500 \
    --election-timeout-max-ms 3000 \
    > "$CLUSTER_DIR/node$i.log" 2>&1 &
  echo $! > "$CLUSTER_DIR/node$i.pid"
done

echo "Waiting for nodes to start (5s)..."
sleep 5

for i in 1 2 3; do
  PID=$(cat "$CLUSTER_DIR/node$i.pid")
  if kill -0 "$PID" 2>/dev/null; then
    echo "  Node $i (PID $PID): RUNNING"
  else
    echo "  Node $i (PID $PID): FAILED — check $CLUSTER_DIR/node$i.log"
    exit 1
  fi
done

echo ""
echo "All nodes running. Now run: ./scripts/cluster-form.sh"
