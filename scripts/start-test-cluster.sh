#!/bin/bash
set -e

# Start a 3-node Aspen cluster for testing with CLI
#
# This script starts a cluster with:
# - Blob storage enabled
# - Job workers enabled
# - VM executor support (if feature is enabled)
# - Client RPC endpoints for CLI access

echo "========================================"
echo "    Starting Aspen Test Cluster"
echo "========================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
CLUSTER_DIR="/tmp/aspen-test-cluster-$$"
LOG_LEVEL=${LOG_LEVEL:-info}
WORKERS_PER_NODE=${WORKERS_PER_NODE:-2}

# Create directories
echo -e "${YELLOW}Creating cluster directories...${NC}"
mkdir -p "$CLUSTER_DIR"/{node1,node2,node3,logs}

# Function to check if a port is open
wait_for_port() {
    local port=$1
    local max_wait=30
    local count=0

    while ! nc -z localhost "$port" 2>/dev/null; do
        count=$((count + 1))
        if [ $count -ge $max_wait ]; then
            echo -e "${RED}✗${NC} Port $port did not open after ${max_wait}s"
            return 1
        fi
        sleep 1
    done
    echo -e "${GREEN}✓${NC} Port $port is open"
}

# Build aspen-node
echo -e "${YELLOW}Building aspen-node...${NC}"
cargo build --release --bin aspen-node 2>&1 | tail -5

# Start node 1 (bootstrap)
echo ""
echo -e "${YELLOW}Starting Node 1 (bootstrap)...${NC}"
RUST_LOG=$LOG_LEVEL cargo run --release --bin aspen-node -- \
    --node-id 1 \
    --data-dir "$CLUSTER_DIR/node1" \
    --iroh-port 7701 \
    --http-addr 127.0.0.1:8081 \
    --enable-blob-storage \
    --workers.enabled true \
    --workers.worker-count $WORKERS_PER_NODE \
    init > "$CLUSTER_DIR/logs/node1.log" 2>&1 &
NODE1_PID=$!

echo "Node 1 PID: $NODE1_PID"
echo "Waiting for node 1 to start..."
sleep 3

# Check if node 1 started
if ! kill -0 $NODE1_PID 2>/dev/null; then
    echo -e "${RED}✗${NC} Node 1 failed to start. Check logs:"
    tail -20 "$CLUSTER_DIR/logs/node1.log"
    exit 1
fi

wait_for_port 8081

# Get cluster ticket
echo ""
echo -e "${YELLOW}Getting cluster ticket...${NC}"
TICKET=$(curl -s http://127.0.0.1:8081/api/cluster/ticket | jq -r '.ticket')

if [ -z "$TICKET" ] || [ "$TICKET" = "null" ]; then
    echo -e "${RED}✗${NC} Failed to get cluster ticket"
    echo "Node 1 logs:"
    tail -20 "$CLUSTER_DIR/logs/node1.log"
    kill $NODE1_PID 2>/dev/null
    exit 1
fi

echo -e "${GREEN}✓${NC} Got cluster ticket"
echo ""
echo -e "${BLUE}Cluster Ticket:${NC}"
echo "$TICKET"
echo ""

# Save ticket for easy access
echo "$TICKET" > "$CLUSTER_DIR/ticket.txt"

# Start node 2
echo -e "${YELLOW}Starting Node 2...${NC}"
RUST_LOG=$LOG_LEVEL cargo run --release --bin aspen-node -- \
    --node-id 2 \
    --data-dir "$CLUSTER_DIR/node2" \
    --iroh-port 7702 \
    --http-addr 127.0.0.1:8082 \
    --enable-blob-storage \
    --workers.enabled true \
    --workers.worker-count $WORKERS_PER_NODE \
    join "$TICKET" > "$CLUSTER_DIR/logs/node2.log" 2>&1 &
NODE2_PID=$!

echo "Node 2 PID: $NODE2_PID"

# Start node 3
echo -e "${YELLOW}Starting Node 3...${NC}"
RUST_LOG=$LOG_LEVEL cargo run --release --bin aspen-node -- \
    --node-id 3 \
    --data-dir "$CLUSTER_DIR/node3" \
    --iroh-port 7703 \
    --http-addr 127.0.0.1:8083 \
    --enable-blob-storage \
    --workers.enabled true \
    --workers.worker-count $WORKERS_PER_NODE \
    join "$TICKET" > "$CLUSTER_DIR/logs/node3.log" 2>&1 &
NODE3_PID=$!

echo "Node 3 PID: $NODE3_PID"
echo ""

# Wait for all nodes
echo "Waiting for all nodes to start..."
wait_for_port 8082
wait_for_port 8083

sleep 3

# Check cluster state
echo ""
echo -e "${YELLOW}Cluster State:${NC}"
curl -s http://127.0.0.1:8081/api/cluster/state | jq '.' || true

# Create PID file for easy shutdown
cat > "$CLUSTER_DIR/pids" <<EOF
$NODE1_PID
$NODE2_PID
$NODE3_PID
EOF

# Create stop script
cat > "$CLUSTER_DIR/stop-cluster.sh" <<'EOF'
#!/bin/bash
echo "Stopping Aspen cluster..."
if [ -f pids ]; then
    while read pid; do
        if kill -0 $pid 2>/dev/null; then
            echo "Stopping PID $pid"
            kill $pid
        fi
    done < pids
    rm pids
    echo "Cluster stopped"
else
    echo "No PID file found"
fi
EOF
chmod +x "$CLUSTER_DIR/stop-cluster.sh"

# Print summary
echo ""
echo "========================================"
echo -e "${GREEN}Cluster Started Successfully!${NC}"
echo "========================================"
echo ""
echo "Cluster Info:"
echo "  Data directory: $CLUSTER_DIR"
echo "  Ticket file:    $CLUSTER_DIR/ticket.txt"
echo "  Logs:           $CLUSTER_DIR/logs/"
echo ""
echo "Node Endpoints:"
echo "  Node 1: http://127.0.0.1:8081 (Iroh: 7701)"
echo "  Node 2: http://127.0.0.1:8082 (Iroh: 7702)"
echo "  Node 3: http://127.0.0.1:8083 (Iroh: 7703)"
echo ""
echo "CLI Usage Examples:"
echo ""
echo "  # Export ticket for easy use"
echo "  export ASPEN_TICKET='$TICKET'"
echo ""
echo "  # Or use --ticket flag"
echo "  aspen-cli --ticket '$TICKET' cluster status"
echo ""
echo "  # Key-value operations"
echo "  aspen-cli kv set mykey 'hello world'"
echo "  aspen-cli kv get mykey"
echo ""
echo "  # Blob operations"
echo "  aspen-cli blob add /path/to/file --tag my-file"
echo "  aspen-cli blob list"
echo ""
echo "  # Submit VM job"
echo "  aspen-cli job submit-vm /path/to/binary --input 'test data'"
echo "  aspen-cli job status <job-id> --follow"
echo ""
echo "To view logs:"
echo "  tail -f $CLUSTER_DIR/logs/node1.log"
echo ""
echo "To stop the cluster:"
echo "  $CLUSTER_DIR/stop-cluster.sh"
echo ""
echo "Or press Ctrl+C now to stop immediately"
echo "========================================"

# Function to cleanup on exit
cleanup() {
    echo ""
    echo -e "${YELLOW}Stopping cluster...${NC}"
    kill $NODE1_PID $NODE2_PID $NODE3_PID 2>/dev/null || true
    echo -e "${GREEN}✓${NC} Cluster stopped"
    echo "Data preserved at: $CLUSTER_DIR"
}

trap cleanup EXIT

# Keep running until interrupted
echo ""
echo "Cluster is running. Press Ctrl+C to stop."
while true; do
    sleep 1
    # Check if processes are still running
    if ! kill -0 $NODE1_PID 2>/dev/null; then
        echo -e "${RED}Node 1 crashed!${NC} Check logs at $CLUSTER_DIR/logs/node1.log"
        exit 1
    fi
    if ! kill -0 $NODE2_PID 2>/dev/null; then
        echo -e "${RED}Node 2 crashed!${NC} Check logs at $CLUSTER_DIR/logs/node2.log"
        exit 1
    fi
    if ! kill -0 $NODE3_PID 2>/dev/null; then
        echo -e "${RED}Node 3 crashed!${NC} Check logs at $CLUSTER_DIR/logs/node3.log"
        exit 1
    fi
done