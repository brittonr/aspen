#!/usr/bin/env bash
# Stop the test cluster and clean up.
# Usage: ./scripts/cluster-stop.sh
set -uo pipefail

CLUSTER_DIR="/tmp/aspen-test-cluster"

if [ ! -d "$CLUSTER_DIR" ]; then
  echo "No cluster running at $CLUSTER_DIR"
  exit 0
fi

for i in 1 2 3; do
  PID=$(cat "$CLUSTER_DIR/node$i.pid" 2>/dev/null || true)
  if [ -n "$PID" ] && kill -0 "$PID" 2>/dev/null; then
    kill "$PID"
    echo "Node $i (PID $PID): stopped"
  else
    echo "Node $i: not running"
  fi
done

sleep 1
rm -rf "$CLUSTER_DIR"
echo "Cluster cleaned up."
