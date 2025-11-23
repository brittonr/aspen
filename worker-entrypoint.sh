#!/bin/sh
set -e

echo "=== Starting Worker ${WORKER_ID:-1} ==="
echo ""

# Create directories
mkdir -p /data /var/log /tmp

# Start flawless in background
echo "Starting flawless server..."
cd /data
flawless up > /var/log/flawless.log 2>&1 &
FLAWLESS_PID=$!
echo "✓ Flawless started (PID: $FLAWLESS_PID)"

# Wait for flawless to be ready
sleep 3

# Verify CONTROL_PLANE_TICKET is set
if [ -z "$CONTROL_PLANE_TICKET" ]; then
    echo "ERROR: CONTROL_PLANE_TICKET environment variable is required"
    exit 1
fi

echo "Connecting to control plane:"
echo "  Ticket: ${CONTROL_PLANE_TICKET:0:50}..."
echo ""

# Start worker (connects to control plane via iroh+h3)
echo "Starting worker..."
cd /
exec worker
