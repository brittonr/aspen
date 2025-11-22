#!/bin/sh
set -e

echo "=== Starting Node ${NODE_ID} ==="
echo "IP: ${NODE_ID}"
echo "Hostname: node${NODE_ID}"
echo ""

# Create directories
mkdir -p /data/hiqlite /data/iroh /var/log /tmp

# Generate hiqlite config from template
envsubst < /etc/hiqlite.toml.template > /data/hiqlite.toml
# mvm-ci looks for hiqlite.toml in current directory
ln -s /data/hiqlite.toml /hiqlite.toml
echo "✓ Generated hiqlite config"

# Staggered startup for Raft cluster formation
# Node1 starts immediately as the bootstrap leader
# Node2 and Node3 wait to allow Node1 to initialize
if [ "${NODE_ID}" != "1" ]; then
  DELAY=$((5 * ${NODE_ID}))
  echo "Waiting ${DELAY} seconds for node1 to initialize Raft cluster..."
  sleep ${DELAY}
fi

# Start flawless in background
echo "Starting flawless server..."
cd /data
flawless up > /var/log/flawless.log 2>&1 &
FLAWLESS_PID=$!
echo "✓ Flawless started (PID: $FLAWLESS_PID)"

# Wait for flawless to be ready
sleep 3

# Start mvm-ci (it will look for hiqlite.toml in /)
echo "Starting mvm-ci..."
cd /
exec mvm-ci
