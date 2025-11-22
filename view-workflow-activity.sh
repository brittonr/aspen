#!/bin/bash

# Show only workflow execution activity
NODE1_COLOR="\033[0;32m"
NODE2_COLOR="\033[0;34m"
NODE3_COLOR="\033[0;35m"
RESET_COLOR="\033[0m"

echo "Workflow Activity Monitor"
echo "========================="
echo ""

follow_activity() {
    local node=$1
    local color=$2
    docker logs -f --tail=0 mvm-ci-node${node} 2>&1 | grep -E "Starting echo|Echo iteration|Finished echo|Job.*Processing" | while IFS= read -r line; do
        echo -e "${color}[Node${node}]${RESET_COLOR} $line"
    done &
}

follow_activity 1 $NODE1_COLOR
follow_activity 2 $NODE2_COLOR
follow_activity 3 $NODE3_COLOR

wait
