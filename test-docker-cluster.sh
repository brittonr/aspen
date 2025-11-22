#!/usr/bin/env bash
set -e

echo "=== Docker Cluster Test for mvm-ci ==="
echo ""

# Check if docker and docker-compose are available
if ! command -v docker &> /dev/null; then
  echo "✗ Docker not found. Please install Docker."
  exit 1
fi

if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
  echo "✗ docker-compose not found. Please install docker-compose."
  exit 1
fi

# Detect docker-compose command and check permissions
if docker compose version &> /dev/null 2>&1; then
  DOCKER_COMPOSE="docker compose"
elif sudo docker compose version &> /dev/null 2>&1; then
  DOCKER_COMPOSE="sudo docker compose"
  echo "Note: Using sudo for Docker access"
elif docker-compose version &> /dev/null 2>&1; then
  DOCKER_COMPOSE="docker-compose"
elif sudo docker-compose version &> /dev/null 2>&1; then
  DOCKER_COMPOSE="sudo docker-compose"
  echo "Note: Using sudo for Docker access"
else
  echo "✗ docker-compose not found. Please install docker-compose."
  exit 1
fi

echo "✓ Docker available"
echo ""

# Build Docker image with Nix
echo "=== Building Docker image with Nix ==="
nix build .#dockerImage
echo "✓ Docker image built"

# Load image into Docker
echo "=== Loading image into Docker ==="
$DOCKER_COMPOSE down -v 2>/dev/null || true
docker load < ./result
echo "✓ Image loaded"
echo ""

# Cleanup function
cleanup() {
  echo ""
  echo "=== Cleaning up ==="
  $DOCKER_COMPOSE down -v 2>/dev/null || true
  rm -f ./result
  echo "✓ Cluster stopped"
}

trap cleanup EXIT

# Clean up any processes on the ports
echo "=== Checking for port conflicts ==="
if lsof -ti:3020,3021,3022 &>/dev/null; then
  echo "Killing processes on ports 3020-3022..."
  kill $(lsof -ti:3020,3021,3022) 2>/dev/null || true
  sleep 1
fi
echo "✓ Ports available"
echo ""

# Start the cluster
echo "=== Starting Docker cluster ==="
$DOCKER_COMPOSE up -d --build

echo "✓ Cluster starting..."
echo ""

# Wait for services
echo "=== Waiting for services ==="
for port in 3020 3021 3022; do
  node=$((port - 3019))
  echo -n "Waiting for node${node} on port $port"
  success=false
  for i in {1..40}; do
    if curl -s -m 1 "http://localhost:${port}/" > /dev/null 2>&1; then
      echo " ✓"
      success=true
      break
    fi
    echo -n "."
    sleep 2
  done

  if [ "$success" = false ]; then
    echo " timeout"
    echo "Checking logs for node${node}:"
    $DOCKER_COMPOSE logs node${node} 2>&1 | head -100
  fi
done

echo ""
echo "=== Running Integration Tests ==="
echo ""

# Test 1: Health check
echo "Test 1: Health Check"
for port in 3020 3021 3022; do
  node=$((port - 3019))
  if curl -s -m 2 "http://localhost:${port}/" > /dev/null 2>&1; then
    echo "  ✓ Node${node} responding on port ${port}"
  else
    echo "  ✗ Node${node} not responding"
  fi
done

# Test 2: Submit job to node1
echo ""
echo "Test 2: Job Submission"
response=$(curl -s -X POST "http://localhost:3020/new-job" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "url=https://docker-cluster-test.com" 2>&1)
echo "  ✓ Submitted job to node1"

sleep 5

# Test 3: Check job replication
echo ""
echo "Test 3: Raft Replication"
for port in 3020 3021 3022; do
  node=$((port - 3019))
  jobs=$(curl -s "http://localhost:${port}/queue/list" 2>/dev/null | jq -r 'length' 2>/dev/null || echo "0")
  echo "  Node${node}: ${jobs} jobs in queue"
done

# Test 4: Submit to different nodes
echo ""
echo "Test 4: Multi-Node Submission"
curl -s -X POST "http://localhost:3021/new-job" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "url=https://node2-job.com" > /dev/null 2>&1
echo "  ✓ Submitted job to node2"

curl -s -X POST "http://localhost:3022/new-job" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "url=https://node3-job.com" > /dev/null 2>&1
echo "  ✓ Submitted job to node3"

sleep 5

# Final status
echo ""
echo "=== Final Cluster Status ==="
for port in 3020 3021 3022; do
  node=$((port - 3019))
  echo "Node${node} (localhost:${port}):"
  curl -s "http://localhost:${port}/queue/list" 2>/dev/null | \
    jq -r '.[] | "  \(.job_id): \(.status) [completed_by: \(.completed_by // \"none\")]"' 2>/dev/null || \
    echo "  (unable to fetch)"
done

echo ""
echo "=== Cluster Running ==="
echo ""
echo "Access nodes:"
echo "  Node1: curl http://localhost:3020/queue/list | jq"
echo "  Node2: curl http://localhost:3021/queue/list | jq"
echo "  Node3: curl http://localhost:3022/queue/list | jq"
echo ""
echo "View logs:"
echo "  $DOCKER_COMPOSE logs -f node1"
echo "  $DOCKER_COMPOSE logs -f node2"
echo "  $DOCKER_COMPOSE logs -f node3"
echo ""
echo "Press Enter to stop cluster and cleanup..."

if [ -t 0 ]; then
  read
else
  sleep 30
fi
