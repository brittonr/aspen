#!/usr/bin/env bash
# Form a 3-node raft cluster from running nodes.
# Prerequisite: ./scripts/cluster-start.sh
set -euo pipefail

cd "$(dirname "$0")/.."
CLUSTER_DIR="/tmp/aspen-test-cluster"
CLI="./target/debug/aspen-cli -q"

TICKET=$(cat "$CLUSTER_DIR/node1/cluster-ticket.txt")

echo "Initializing cluster on node 1..."
$CLI --ticket "$TICKET" cluster init
sleep 3

# Extract endpoint IDs and addresses from logs (strip ANSI codes)
strip_ansi() { sed 's/\x1b\[[0-9;]*m//g'; }

for i in 2 3; do
  ENDPOINT=$(grep "created Iroh endpoint" "$CLUSTER_DIR/node$i.log" \
    | strip_ansi | grep -oP 'endpoint_id=\K[a-f0-9]+')
  ADDR=$(grep "direct_addrs" "$CLUSTER_DIR/node$i.log" \
    | strip_ansi | grep -oP '\d+\.\d+\.\d+\.\d+:\d+' | head -1)

  echo "Adding node $i as learner (endpoint=${ENDPOINT:0:16}..., addr=$ADDR)..."
  $CLI --ticket "$TICKET" cluster add-learner \
    --node-id "$i" \
    --addr "{\"id\":\"$ENDPOINT\",\"addrs\":[{\"Ip\":\"$ADDR\"}]}"
  sleep 2
done

echo "Promoting all nodes to voters..."
$CLI --ticket "$TICKET" cluster change-membership 1 2 3
sleep 2

echo ""
echo "Cluster formed:"
$CLI --ticket "$TICKET" cluster status
echo ""
$CLI --ticket "$TICKET" cluster metrics
echo ""
echo "Cluster ready. Now run: ./scripts/cluster-test.sh"
