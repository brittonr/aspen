#!/usr/bin/env bash
# Test script to verify server starts and is accessible

echo "=== Testing Server Startup ==="

# Clean up any existing data and processes
rm -rf /tmp/test-hiqlite-$$
pkill -f "mvm-ci" 2>/dev/null || true
sleep 1

# Set up test environment
export HQL_DATA_DIR=/tmp/test-hiqlite-$$
export SKIP_FLAWLESS=1
export SKIP_VM_MANAGER=1
export RUST_LOG=info

echo "Starting server with test configuration..."
echo "Data directory: $HQL_DATA_DIR"
echo

# Start the server in background
cargo run --bin mvm-ci 2>&1 | tee /tmp/server-$$.log &
SERVER_PID=$!

echo "Server PID: $SERVER_PID"
echo "Waiting for server to start..."

# Wait for server to start (max 30 seconds)
MAX_WAIT=30
WAITED=0
while [ $WAITED -lt $MAX_WAIT ]; do
    if grep -q "Listening\|3020\|Dashboard\|Health" /tmp/server-$$.log 2>/dev/null; then
        echo "Server appears to have started!"
        break
    fi

    # Check if server process is still running
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo "Server process died!"
        echo "Last 20 lines of log:"
        tail -20 /tmp/server-$$.log
        exit 1
    fi

    sleep 1
    WAITED=$((WAITED + 1))
    echo -n "."
done
echo

if [ $WAITED -eq $MAX_WAIT ]; then
    echo "Server failed to start within $MAX_WAIT seconds"
    echo "Last 20 lines of log:"
    tail -20 /tmp/server-$$.log
    kill $SERVER_PID 2>/dev/null
    exit 1
fi

echo "Testing localhost HTTP endpoint..."
sleep 2

# Test health endpoint
echo -n "Testing health endpoint: "
RESPONSE=$(curl -s -w "\n%{http_code}" http://localhost:3020/api/health 2>/dev/null | tail -1)
if [ "$RESPONSE" = "200" ]; then
    echo "✓ OK (200)"
else
    echo "✗ Failed (HTTP $RESPONSE)"
    echo "Server might not be listening on port 3020"
fi

# Test root endpoint
echo -n "Testing root endpoint: "
RESPONSE=$(curl -s -w "\n%{http_code}" http://localhost:3020/ 2>/dev/null | tail -1)
if [ "$RESPONSE" = "200" ] || [ "$RESPONSE" = "404" ]; then
    echo "✓ Server responding (HTTP $RESPONSE)"
else
    echo "✗ No response"
fi

# Check if port is actually open
echo -n "Checking port 3020: "
if nc -z localhost 3020 2>/dev/null; then
    echo "✓ Port is open"
else
    echo "✗ Port is closed"
fi

echo
echo "Server log tail:"
tail -10 /tmp/server-$$.log

# Clean up
echo
echo "Cleaning up..."
kill $SERVER_PID 2>/dev/null
rm -rf /tmp/test-hiqlite-$$
rm -f /tmp/server-$$.log

echo "=== Test Complete ==="