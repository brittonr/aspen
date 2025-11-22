#!/bin/bash

# Color codes for different nodes
NODE1_COLOR="\033[0;32m"  # Green
NODE2_COLOR="\033[0;34m"  # Blue
NODE3_COLOR="\033[0;35m"  # Magenta
RESET_COLOR="\033[0m"

echo "Aggregating Flawless workflow logs from all nodes"
echo "Press Ctrl+C to stop"
echo ""

# Function to follow flawless logs from a specific node
follow_flawless() {
    local node=$1
    local color=$2
    docker exec mvm-ci-node${node} tail -f /var/log/flawless.log 2>/dev/null | while IFS= read -r line; do
        echo -e "${color}[Node${node}]${RESET_COLOR} $line"
    done &
}

# Start log followers for each node
follow_flawless 1 $NODE1_COLOR
follow_flawless 2 $NODE2_COLOR
follow_flawless 3 $NODE3_COLOR

# Wait for all background processes
wait
