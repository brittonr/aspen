#!/bin/bash

# Color codes for different nodes
NODE1_COLOR="\033[0;32m"  # Green
NODE2_COLOR="\033[0;34m"  # Blue
NODE3_COLOR="\033[0;35m"  # Magenta
RESET_COLOR="\033[0m"

# Default to all nodes
NODES=${1:-"1 2 3"}

echo "Aggregating logs from nodes: $NODES"
echo "Press Ctrl+C to stop"
echo ""

# Function to follow logs from a specific node
follow_node() {
    local node=$1
    local color=$2
    docker logs -f --tail=20 mvm-ci-node${node} 2>&1 | while IFS= read -r line; do
        echo -e "${color}[Node${node}]${RESET_COLOR} $line"
    done
}

# Start log followers for each node in background
for node in $NODES; do
    case $node in
        1) color=$NODE1_COLOR ;;
        2) color=$NODE2_COLOR ;;
        3) color=$NODE3_COLOR ;;
    esac
    follow_node $node $color &
done

# Wait for all background processes
wait
