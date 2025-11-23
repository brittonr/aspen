#!/usr/bin/env bash
set -e

echo "=== Testing Cargo-Built Binaries (Bypassing Nix Cache) ===" echo ""

# Stop existing cluster
echo "1. Stopping existing cluster..."
docker compose down -v 2>/dev/null || true

# Update docker-compose temporarily to use cargo-test image
echo "2. Temporarily updating docker-compose.yml to use cargo-test image..."
sed -i.bak 's/image: mvm-ci-cluster:latest/image: mvm-ci-cluster:cargo-test/g' docker-compose.yml
echo "✓ Updated to cargo-test image"
echo ""

# Start control plane
echo "3. Starting control plane (3 nodes)..."
docker compose up -d node1 node2 node3
sleep 15

# Check health
HEALTH=$(curl -s http://localhost:3020/hiqlite/health | jq -r '.is_healthy')
if [ "$HEALTH" != "true" ]; then
    echo "ERROR: Control plane not healthy"
    docker logs mvm-ci-node1 | tail -20
    mv docker-compose.yml.bak docker-compose.yml
    exit 1
fi
echo "✓ Control plane healthy"

# Extract ticket
TICKET=$(docker logs mvm-ci-node1 2>&1 | grep "iroh+h3://endpoint" | tail -1 | awk '{print $1}')
if [ -z "$TICKET" ]; then
    echo "ERROR: Could not extract ticket"
    mv docker-compose.yml.bak docker-compose.yml
    exit 1
fi

echo "Control plane ticket: ${TICKET:0:60}..."
export CONTROL_PLANE_TICKET="$TICKET"

# Start workers
echo "4. Starting workers (2 nodes)..."
docker compose up -d worker1 worker2
sleep 10

# Check worker processes
echo "5. Checking worker status..."
docker ps --filter "name=worker" --format "table {{.Names}}\t{{.Status}}"
echo ""

# Submit test jobs
echo "6. Submitting 3 test jobs..."
for i in {1..3}; do
    curl -s -X POST "http://localhost:3020/new-job" \
        -H "Content-Type: application/x-www-form-urlencoded" \
        -d "url=https://test-job-$i.com" > /dev/null
    echo "  ✓ Job $i submitted"
done
echo ""

# Wait and monitor
echo "7. Waiting for job processing (20 seconds)..."
sleep 20

echo "8. Job Queue Status:"
curl -s http://localhost:3020/queue/list | jq -r '.[] | "  \(.job_id): \(.status) (claimed_by: \(.claimed_by[:10] // \"none\"))"'
echo ""

echo "9. Queue Statistics:"
curl -s http://localhost:3020/queue/stats | jq '.'
echo ""

echo "10. Worker Logs (last 15 lines):"
echo "Worker1:"
docker logs mvm-ci-worker1 2>&1 | grep -E "(Worker|claim|job|INFO)" | tail -15
echo ""
echo "Worker2:"
docker logs mvm-ci-worker2 2>&1 | grep -E "(Worker|claim|job|INFO)" | tail -15
echo ""

# Restore docker-compose.yml
echo "Restoring docker-compose.yml..."
mv docker-compose.yml.bak docker-compose.yml

echo "=== Test Complete ===\"
echo ""
echo "If workers claimed jobs successfully, this proves the h3 fix works!"
echo "The issue is Nix caching the old binary."
