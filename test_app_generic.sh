#!/usr/bin/env bash
# Test script to verify the application is working with generic payloads

echo "=== Testing Blixard Generic Job Orchestrator ==="
echo

# Clean up any existing data
rm -rf /tmp/blixard-test-data

# Set up environment
export HQL_DATA_DIR=/tmp/blixard-test-data
export SKIP_FLAWLESS=1
export SKIP_VM_MANAGER=1
export RUST_LOG=info

# Build the app
echo "Building application..."
cargo build --bin mvm-ci 2>/dev/null || {
    echo "Build failed!"
    exit 1
}

echo "Starting application..."
./target/debug/mvm-ci &
APP_PID=$!

# Wait for startup
echo "Waiting for application to start..."
sleep 5

# Check if app is running
if ! kill -0 $APP_PID 2>/dev/null; then
    echo "Application failed to start!"
    exit 1
fi

# Test health endpoint
echo
echo "Testing health endpoint..."
curl -s http://localhost:3020/api/health | jq . || echo "Health check failed"

# Test generic job submission
echo
echo "Testing generic job submission..."
echo "Submitting a generic JSON payload (not a CI/CD URL)..."
curl -X POST http://localhost:3020/queue/publish \
  -H "Content-Type: application/json" \
  -d '{
    "task_type": "data_processing",
    "input_data": {
      "source": "database",
      "query": "SELECT * FROM users",
      "transform": "aggregate_by_region"
    },
    "output_format": "json",
    "priority": "high"
  }' | jq . || echo "Job submission failed"

echo
echo "Checking queue status..."
curl -s http://localhost:3020/queue/list | jq . || echo "Queue list failed"

echo
echo "Checking workers..."
curl -s http://localhost:3020/api/workers | jq . || echo "Workers check failed"

# Clean up
echo
echo "Cleaning up..."
kill $APP_PID 2>/dev/null
rm -rf /tmp/blixard-test-data

echo
echo "=== Test Complete ==="