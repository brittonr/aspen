#!/bin/bash
# Test script for Firecracker worker

set -e

echo "Setting up test environment for Firecracker worker..."

# Required environment variables
export BLIXARD_API_KEY="test-key-32-characters-minimum-length-for-security"
export HIQLITE_SECRET_RAFT="test-raft-secret-minimum-32-chars"
export HIQLITE_SECRET_API="test-api-secret-minimum-32-chars-"
export HIQLITE_ENC_KEY="test-encryption-key-for-hiqlite-storage"

# Generate test hiqlite.toml from template
echo "Generating hiqlite.toml..."
envsubst < hiqlite.toml.template > hiqlite.toml

# Start hiqlite in background
echo "Starting hiqlite database..."
hiqlite --config hiqlite.toml &
HIQLITE_PID=$!
echo "Hiqlite started with PID: $HIQLITE_PID"

# Wait for hiqlite to be ready
sleep 2

# Start the main server in background
echo "Starting mvm-ci server..."
cargo run --bin mvm-ci &
SERVER_PID=$!
echo "Server started with PID: $SERVER_PID"

# Wait for server to be ready
sleep 3

# Create a test job payload
TEST_JOB=$(cat <<'EOF'
{
  "id": "test-firecracker-job-001",
  "payload": "{\"task\": \"echo 'Hello from Firecracker VM'\", \"type\": \"simple\"}",
  "priority": 1,
  "status": "Pending"
}
EOF
)

echo "Submitting test job to queue..."
curl -X POST http://localhost:3020/api/queue/publish \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $BLIXARD_API_KEY" \
  -d "$TEST_JOB"

echo ""
echo "Server and database are running."
echo "To test Firecracker worker, run:"
echo "  CONTROL_PLANE_TICKET=http://localhost:3020 cargo run --bin worker -- --worker-type firecracker"
echo ""
echo "To stop services:"
echo "  kill $SERVER_PID $HIQLITE_PID"
echo ""
echo "Logs available at:"
echo "  Server: Check terminal output"
echo "  Worker VMs: ./data/firecracker-vms/<vm-id>/"