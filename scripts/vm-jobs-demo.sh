#!/bin/bash
set -e

# VM Jobs Demo - Demonstrates VM-based job execution with Aspen cluster

echo "========================================"
echo "    Aspen VM Jobs Demo"
echo "========================================"
echo ""
echo "This demo will:"
echo "1. Build the echo-worker binary (freestanding ELF for VMs)"
echo "2. Start a 3-node Aspen cluster with VM executor support"
echo "3. Submit the echo-worker binary as a VM job"
echo "4. Show the execution result"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to check if command succeeded
check_success() {
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓${NC} $1"
    else
        echo -e "${RED}✗${NC} $1 failed"
        exit 1
    fi
}

# Step 1: Build the echo-worker binary
echo -e "${YELLOW}Step 1:${NC} Building echo-worker binary..."
(cd examples/vm-jobs/echo-worker && nix develop -c cargo build --release)
check_success "Echo-worker build"

# Check the binary
BINARY_PATH="target/x86_64-unknown-none/release/echo-worker"
if [ -f "$BINARY_PATH" ]; then
    BINARY_SIZE=$(ls -lh "$BINARY_PATH" | awk '{print $5}')
    echo -e "${GREEN}✓${NC} Binary built: $BINARY_PATH (size: $BINARY_SIZE)"

    # Verify symbols are exported
    echo "Checking exported symbols..."
    nm "$BINARY_PATH" | grep -E "T (execute|get_result_len|_start)" | head -3
else
    echo -e "${RED}✗${NC} Binary not found at $BINARY_PATH"
    exit 1
fi

# Step 2: Start the cluster
echo ""
echo -e "${YELLOW}Step 2:${NC} Starting 3-node Aspen cluster with VM executor support..."

# Clean up any existing cluster
pkill -f "aspen-node" 2>/dev/null || true
sleep 1

# Create temp directory for cluster data
CLUSTER_DIR="/tmp/aspen-vm-demo-$$"
mkdir -p "$CLUSTER_DIR"
echo "Cluster data directory: $CLUSTER_DIR"

# Build aspen-node with vm-executor feature
echo "Building aspen-node with VM executor support..."
cargo build --release --bin aspen-node --features vm-executor
check_success "Aspen-node build"

# Start node 1 (bootstrap)
echo "Starting node-1 (bootstrap)..."
RUST_LOG=info cargo run --release --bin aspen-node --features vm-executor -- \
    --node-id node-1 \
    --data-dir "$CLUSTER_DIR/node-1" \
    --listen-addr 127.0.0.1:7701 \
    --http-addr 127.0.0.1:8081 \
    --workers.enabled true \
    --workers.worker-count 2 \
    init > "$CLUSTER_DIR/node-1.log" 2>&1 &
NODE1_PID=$!

sleep 3

# Check if node-1 started successfully
if kill -0 $NODE1_PID 2>/dev/null; then
    echo -e "${GREEN}✓${NC} Node-1 started (PID: $NODE1_PID)"
else
    echo -e "${RED}✗${NC} Node-1 failed to start"
    cat "$CLUSTER_DIR/node-1.log" | tail -20
    exit 1
fi

# Get the bootstrap ticket
echo "Getting bootstrap ticket..."
BOOTSTRAP_TICKET=$(curl -s http://127.0.0.1:8081/api/cluster/ticket | jq -r '.ticket')
if [ -z "$BOOTSTRAP_TICKET" ] || [ "$BOOTSTRAP_TICKET" = "null" ]; then
    echo -e "${RED}✗${NC} Failed to get bootstrap ticket"
    exit 1
fi
echo -e "${GREEN}✓${NC} Got bootstrap ticket"

# Start node 2
echo "Starting node-2..."
RUST_LOG=info cargo run --release --bin aspen-node --features vm-executor -- \
    --node-id node-2 \
    --data-dir "$CLUSTER_DIR/node-2" \
    --listen-addr 127.0.0.1:7702 \
    --http-addr 127.0.0.1:8082 \
    --workers.enabled true \
    --workers.worker-count 2 \
    join "$BOOTSTRAP_TICKET" > "$CLUSTER_DIR/node-2.log" 2>&1 &
NODE2_PID=$!

# Start node 3
echo "Starting node-3..."
RUST_LOG=info cargo run --release --bin aspen-node --features vm-executor -- \
    --node-id node-3 \
    --data-dir "$CLUSTER_DIR/node-3" \
    --listen-addr 127.0.0.1:7703 \
    --http-addr 127.0.0.1:8083 \
    --workers.enabled true \
    --workers.worker-count 2 \
    join "$BOOTSTRAP_TICKET" > "$CLUSTER_DIR/node-3.log" 2>&1 &
NODE3_PID=$!

sleep 5

# Check cluster health
echo ""
echo -e "${YELLOW}Step 3:${NC} Checking cluster health..."
CLUSTER_STATE=$(curl -s http://127.0.0.1:8081/api/cluster/state)
echo "$CLUSTER_STATE" | jq '.'

# Check for VM executor registration
echo ""
echo "Checking for VM executor workers..."
for port in 8081 8082 8083; do
    echo -n "Node on port $port: "
    grep -q "VM executor worker registered" "$CLUSTER_DIR/node-*.log" && echo -e "${GREEN}VM executor available${NC}" || echo -e "${YELLOW}VM executor not available${NC}"
done

# Step 4: Create and run the job submission client
echo ""
echo -e "${YELLOW}Step 4:${NC} Creating job submission client..."

cat > "$CLUSTER_DIR/submit_vm_job.rs" << 'EOF'
use std::fs;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Loading echo-worker binary...");
    let binary = fs::read("target/x86_64-unknown-none/release/echo-worker")?;
    println!("Binary size: {} bytes", binary.len());

    println!("Connecting to cluster...");
    let client = aspen_client::AspenClient::connect("http://127.0.0.1:8081").await?;

    println!("Submitting VM job...");
    let job_spec = aspen_jobs::JobSpec::with_native_binary(binary)
        .timeout(Duration::from_secs(10))
        .tag("vm-demo");

    // Add input data
    let mut job = aspen_jobs::Job::from_spec(job_spec);
    job.input = Some(b"Hello from the Aspen cluster!".to_vec());

    let job_id = client.submit_job(job).await?;
    println!("Job submitted with ID: {}", job_id);

    // Wait for result
    println!("Waiting for job result...");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let result = client.get_job_result(&job_id).await?;
    match result {
        aspen_jobs::JobResult::Success(output) => {
            println!("Job succeeded!");
            println!("Output: {}", String::from_utf8_lossy(&output.data));
        }
        aspen_jobs::JobResult::Failure(f) => {
            println!("Job failed: {}", f.reason);
        }
        aspen_jobs::JobResult::Cancelled => {
            println!("Job was cancelled");
        }
    }

    Ok(())
}
EOF

echo -e "${GREEN}✓${NC} Job submission client created"

# Step 5: Submit the job
echo ""
echo -e "${YELLOW}Step 5:${NC} Submitting VM job to cluster..."
echo ""

# Note: For now, we'll simulate the job submission since the client API isn't fully integrated
echo -e "${BLUE}Expected output when fully integrated:${NC}"
echo "Job submitted with ID: job_12345"
echo "Job succeeded!"
echo 'Output: "Echo: Hello from the Aspen cluster!"'

# Cleanup function
cleanup() {
    echo ""
    echo -e "${YELLOW}Cleaning up...${NC}"
    kill $NODE1_PID $NODE2_PID $NODE3_PID 2>/dev/null || true
    rm -rf "$CLUSTER_DIR"
    echo -e "${GREEN}✓${NC} Cleanup complete"
}

# Set trap for cleanup
trap cleanup EXIT

echo ""
echo "========================================"
echo -e "${GREEN}Demo setup complete!${NC}"
echo ""
echo "The cluster is running with VM executor support."
echo "Nodes are logging to: $CLUSTER_DIR/*.log"
echo ""
echo "You can check the logs with:"
echo "  tail -f $CLUSTER_DIR/node-1.log"
echo ""
echo "Press Ctrl+C to stop the cluster and clean up."
echo "========================================"

# Keep the script running
read -r -d '' _ </dev/tty