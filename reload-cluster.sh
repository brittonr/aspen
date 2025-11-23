#!/usr/bin/env bash
set -e

echo "=== Reloading Cluster with New Image ==="
echo ""

echo "1. Loading new Docker image..."
./result | docker load
echo "✓ Image loaded"
echo ""

echo "2. Stopping cluster..."
docker compose down
echo "✓ Cluster stopped"
echo ""

echo "3. Starting cluster..."
./start-cluster-with-workers.sh
echo ""

echo "=== Cluster Reloaded ==="
echo ""
echo "Dashboard available at:"
echo "  http://localhost:3020/dashboard"
echo ""
echo "Monitor with:"
echo "  ./monitor-cluster.sh"
