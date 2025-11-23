#!/usr/bin/env bash
set -e

echo "=== Starting Control Plane (3 nodes) ==="
docker compose up -d node1 node2 node3

echo "Waiting for control plane to be healthy..."
sleep 15

# Check cluster health
HEALTH=$(curl -s http://localhost:3020/hiqlite/health | jq -r '.is_healthy')
if [ "$HEALTH" != "true" ]; then
    echo "ERROR: Control plane not healthy"
    docker compose logs node1 | tail -20
    exit 1
fi

echo "✓ Control plane healthy"

# Get iroh+h3 ticket from node1
TICKET=$(docker logs mvm-ci-node1 2>&1 | grep "iroh+h3://endpoint" | tail -1 | awk '{print $1}')

if [ -z "$TICKET" ]; then
    echo "ERROR: Could not extract control plane ticket"
    exit 1
fi

echo "Control plane ticket:"
echo "  $TICKET"
echo ""

# Export ticket for docker-compose
export CONTROL_PLANE_TICKET="$TICKET"

echo "=== Starting Workers (2 nodes) ==="
docker compose up -d worker1 worker2

echo ""
echo "=== Cluster Status ==="
echo "Control Plane: 3 nodes (node1, node2, node3)"
echo "Workers: 2 nodes (worker1, worker2)"
echo ""
echo "Access:"
echo "  Control plane: http://localhost:3020"
echo "  Queue API: curl http://localhost:3020/queue/list | jq"
echo ""
echo "To view logs:"
echo "  docker logs mvm-ci-node1"
echo "  docker logs mvm-ci-worker1"
echo ""
