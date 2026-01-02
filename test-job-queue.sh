#!/bin/bash
set -e

echo "=== Aspen Job Queue Test ==="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Start a single node for testing
echo -e "${YELLOW}Starting test node...${NC}"
DATA_DIR="/tmp/aspen-job-test-$(date +%s)"
mkdir -p "$DATA_DIR"

# Start node in background
./target/release/aspen-node \
    --node-id 1 \
    --data-dir "$DATA_DIR" \
    --bind-address 127.0.0.1:8081 \
    > /tmp/aspen-job-test.log 2>&1 &

NODE_PID=$!
echo "Node started with PID: $NODE_PID"

# Wait for node to start
echo -e "${YELLOW}Waiting for node to initialize...${NC}"
sleep 5

# Get cluster ticket
echo -e "${YELLOW}Getting cluster ticket...${NC}"
TICKET=$(curl -s http://127.0.0.1:8081/cluster-ticket | jq -r '.ticket' || echo "")

if [ -z "$TICKET" ]; then
    echo -e "${RED}Failed to get cluster ticket${NC}"
    kill $NODE_PID 2>/dev/null || true
    exit 1
fi

echo "Cluster ticket: ${TICKET:0:50}..."

# Export ticket for CLI
export ASPEN_TICKET="$TICKET"

# Test 1: Submit a job
echo ""
echo -e "${GREEN}Test 1: Submit a job${NC}"
./target/release/aspen-cli job submit \
    "test-job" \
    '{"action": "process", "data": "test-data"}' \
    --priority 2 \
    --tags test,demo \
    --json | jq '.'

# Test 2: Get job statistics
echo ""
echo -e "${GREEN}Test 2: Get job queue statistics${NC}"
./target/release/aspen-cli job stats --json | jq '.'

# Test 3: List jobs
echo ""
echo -e "${GREEN}Test 3: List jobs${NC}"
./target/release/aspen-cli job list --json | jq '.'

# Test 4: Worker status
echo ""
echo -e "${GREEN}Test 4: Check worker status${NC}"
./target/release/aspen-cli job workers --json | jq '.'

# Cleanup
echo ""
echo -e "${YELLOW}Cleaning up...${NC}"
kill $NODE_PID 2>/dev/null || true
rm -rf "$DATA_DIR"

echo -e "${GREEN}Job queue test completed!${NC}"