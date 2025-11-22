#!/usr/bin/env bash

# Color codes for different nodes
NODE1_COLOR="\033[0;32m"  # Green
NODE2_COLOR="\033[0;34m"  # Blue
NODE3_COLOR="\033[0;35m"  # Magenta
RESET_COLOR="\033[0m"

echo "Aggregating Flawless workflow logs from all nodes"
echo "Press Ctrl+C to stop"
echo ""

# Check if containers are running
for node in 1 2 3; do
    if ! docker ps --format '{{.Names}}' | grep -q "mvm-ci-node${node}"; then
        echo "Warning: Container mvm-ci-node${node} not running"
    fi
done
echo ""

# Function to follow flawless logs from a specific node
follow_flawless() {
    local node=$1
    local color=$2

    # Use docker exec to tail the flawless log file inside the container
    # If the file doesn't exist yet, wait for it to be created
    while ! docker exec mvm-ci-node${node} test -f /var/log/flawless.log 2>/dev/null; do
        sleep 1
    done

    docker exec mvm-ci-node${node} tail -f /var/log/flawless.log 2>/dev/null | while IFS= read -r line; do
        echo -e "${color}[Node${node}]${RESET_COLOR} $line"
    done &
}

# Start log followers for each node
follow_flawless 1 $NODE1_COLOR
follow_flawless 2 $NODE2_COLOR
follow_flawless 3 $NODE3_COLOR

# Keep the script running
trap 'kill $(jobs -p) 2>/dev/null' EXIT
wait
