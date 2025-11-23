#!/usr/bin/env bash
set -e

echo "=== Testing Option A: Workers with Isolated Flawless ==="
echo ""

# Rebuild and reload image
echo "1. Loading Docker image..."
docker compose down -v 2>/dev/null || true
./result | docker load
echo "✓ Image loaded"
echo ""

# Start cluster
echo "2. Starting cluster..."
./start-cluster-with-workers.sh
echo ""

# Wait for workers to initialize
echo "3. Waiting for workers to initialize (10 seconds)..."
sleep 10

# Check worker status
echo "4. Checking worker processes..."
WORKER1_RUNNING=$(docker exec mvm-ci-worker1 ps aux | grep -c "worker" || echo "0")
WORKER2_RUNNING=$(docker exec mvm-ci-worker2 ps aux | grep -c "worker" || echo "0")

if [ "$WORKER1_RUNNING" -gt "0" ]; then
    echo "✓ Worker1 is running"
else
    echo "✗ Worker1 exited - checking logs:"
    docker logs mvm-ci-worker1 | tail -20
fi

if [ "$WORKER2_RUNNING" -gt "0" ]; then
    echo "✓ Worker2 is running"
else
    echo "✗ Worker2 exited"
fi
echo ""

# Submit test jobs
echo "5. Submitting 3 test jobs..."
for i in {1..3}; do
    curl -s -X POST "http://localhost:3020/new-job" \
        -H "Content-Type: application/x-www-form-urlencoded" \
        -d "url=https://test-job-$i.com" > /dev/null
    echo "  ✓ Job $i submitted"
done
echo ""

# Wait and check results
echo "6. Waiting for job processing (15 seconds)..."
sleep 15

echo "7. Job Queue Status:"
curl -s http://localhost:3020/queue/list | jq -r '.[] | "  \(.job_id): \(.status) (claimed_by: \(.claimed_by[:10] // "none"))"'
echo ""

echo "8. Queue Statistics:"
curl -s http://localhost:3020/queue/stats | jq '.'
echo ""

echo "9. Worker Logs (last 10 lines):"
echo "Worker1:"
docker logs mvm-ci-worker1 2>&1 | tail -10
echo ""
echo "Worker2:"
docker logs mvm-ci-worker2 2>&1 | tail -10
echo ""

echo "=== Test Complete ==="
echo ""
echo "Cluster is running! To monitor:"
echo "  ./monitor-cluster.sh              # Interactive monitor (recommended)"
echo "  ./monitor-cluster.sh --once       # Single snapshot"
echo ""
echo "Other useful commands:"
echo "  curl -X POST http://localhost:3020/new-job -H 'Content-Type: application/x-www-form-urlencoded' -d 'url=https://example.com'"
echo "  docker logs -f mvm-ci-worker1"
echo ""
echo "To cleanup:"
echo "  docker compose down -v"
