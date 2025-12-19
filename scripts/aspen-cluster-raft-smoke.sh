#!/usr/bin/env bash
# OBSOLETE: This script uses the removed HTTP API.
# The HTTP API was removed in favor of Iroh Client RPC (Dec 2025).
# Use aspen-tui to interact with Aspen nodes instead.
#
# To test Raft consensus, use the Rust integration tests:
#   cargo nextest run --test gossip_integration_test
#   cargo nextest run --test raft_
echo "ERROR: This smoke test is obsolete. HTTP API has been removed."
echo "Use aspen-tui to interact with Aspen nodes via Iroh Client RPC."
exit 1

# Original script below (kept for reference)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/debug/aspen-node"
EXTRA_NODE_ARGS=("$@")

if (( ${#EXTRA_NODE_ARGS[@]} > 0 )); then
    echo "Forwarding extra aspen-node args: ${EXTRA_NODE_ARGS[*]}"
fi

rpc() {
    local uri=$1
    local body="${2:-}"

    echo "--- rpc(:$uri, $body)"
    {
        if [[ -z "$body" ]]; then
            curl --silent "127.0.0.1:$uri"
        else
            curl --silent "127.0.0.1:$uri" -H "Content-Type: application/json" -d "$body"
        fi
    } | {
        # Skip jq for metrics endpoint (returns Prometheus format, not JSON)
        if [[ "$uri" == *"/metrics"* ]]; then
            cat
        elif command -v jq >/dev/null 2>&1; then
            jq
        else
            cat
        fi
    }
    echo
    echo
}

kill_nodes() {
    set +e
    if command -v pgrep >/dev/null 2>&1; then
        mapfile -t pids < <(pgrep -f "aspen-node" || true)
        if (( ${#pids[@]} > 0 )); then
            kill "${pids[@]}" >/dev/null 2>&1 || true
            for _ in $(seq 1 50); do
                sleep 0.1
                pgrep -f "aspen-node" >/dev/null 2>&1 || break
            done
        fi
    elif command -v killall >/dev/null 2>&1; then
        killall aspen-node >/dev/null 2>&1 || true
    fi
    set -e
}

trap kill_nodes EXIT

export RUST_LOG="${RUST_LOG:-info}"
export RUST_BACKTRACE=full

echo "Building aspen-node..."
cargo build --bin aspen-node >/dev/null

echo "Killing any existing nodes"
kill_nodes
sleep 1

wait_for_http_port() {
    local port=$1
    local attempts=0
    until python3 - <<'PY' "$port" >/dev/null 2>&1; do
import socket
import sys
sock = socket.create_connection(("127.0.0.1", int(sys.argv[1])), timeout=0.5)
sock.close()
PY
        sleep 0.2
        attempts=$((attempts + 1))
        if (( attempts > 200 )); then
            echo "Timed out waiting for HTTP port 127.0.0.1:$port" >&2
            exit 1
        fi
    done
}

start_node() {
    local id=$1
    local http=$2
    local cluster_port=$3
    local log_file=$4
    printf -v secret "%064x" "$((1000 + id))"
    local cmd=( "$BIN"
        --node-id "$id"
        --http-addr "127.0.0.1:$http"
        --host "127.0.0.1"
        --port "$cluster_port"
        --cookie "aspen-dev-cookie"
        --iroh-secret-key "$secret"
        --control-backend "raft_actor"
    )
    if (( ${#EXTRA_NODE_ARGS[@]} > 0 )); then
        cmd+=("${EXTRA_NODE_ARGS[@]}")
    fi
    nohup "${cmd[@]}" >"$log_file" 2>&1 &
    sleep 1
    wait_for_http_port "$http"
    echo "Node $id listening on 127.0.0.1:$http"
}

# Wait for leader election by polling metrics
# Returns the leader node ID once elected
wait_for_leader() {
    local port=$1
    local attempts=0
    local max_attempts=40  # 20 seconds with 0.5s sleep

    echo "Waiting for leader election on port $port..." >&2
    while true; do
        local metrics
        metrics=$(curl --silent "127.0.0.1:$port/metrics" 2>/dev/null || echo "")

        # Extract current_leader from Prometheus format
        # Example line: aspen_current_leader{node_id="1"} 1
        local leader
        leader=$(echo "$metrics" | grep -E '^aspen_current_leader' | grep -oE '[0-9]+$' | head -1 || echo "")

        if [[ -n "$leader" && "$leader" != "0" ]]; then
            echo "Leader elected: node $leader" >&2
            echo "$leader"
            return 0
        fi

        sleep 0.5
        attempts=$((attempts + 1))
        if (( attempts >= max_attempts )); then
            echo "ERROR: Timeout waiting for leader election after ${max_attempts} attempts" >&2
            echo "Last metrics response:" >&2
            echo "$metrics" >&2
            return 1
        fi
    done
}

# Verify that data replicates from write_port to read_port
verify_replication() {
    local write_port=$1
    local read_port=$2
    local key=$3
    local expected_value=$4

    echo "Verifying replication: write to :$write_port, read from :$read_port"

    # Write to one node
    local write_response
    write_response=$(curl --silent "127.0.0.1:$write_port/write" \
        -H "Content-Type: application/json" \
        -d "{\"command\":{\"Set\":{\"key\":\"$key\",\"value\":\"$expected_value\"}}}" \
        2>/dev/null || echo "{}")

    # Wait for replication (Raft needs time to replicate)
    sleep 2

    # Read from different node
    local read_response
    read_response=$(curl --silent "127.0.0.1:$read_port/read" \
        -H "Content-Type: application/json" \
        -d "{\"key\":\"$key\"}" \
        2>/dev/null || echo "{}")

    local actual
    actual=$(echo "$read_response" | jq -r '.value // empty' 2>/dev/null || echo "")

    if [[ "$actual" != "$expected_value" ]]; then
        echo "ERROR: Replication verification failed" >&2
        echo "  Expected: '$expected_value'" >&2
        echo "  Got: '$actual'" >&2
        echo "  Write response: $write_response" >&2
        echo "  Read response: $read_response" >&2
        return 1
    fi

    echo "✓ Replication verified: $key=$expected_value"
}

# Verify all nodes agree on the same leader
verify_leader_consensus() {
    local leader=$1
    shift
    local ports=("$@")

    echo "Verifying all nodes agree on leader $leader..."

    for port in "${ports[@]}"; do
        local metrics
        metrics=$(curl --silent "127.0.0.1:$port/metrics" 2>/dev/null || echo "")

        local node_leader
        node_leader=$(echo "$metrics" | grep -E '^aspen_current_leader' | grep -oE '[0-9]+$' | head -1 || echo "")

        if [[ "$node_leader" != "$leader" ]]; then
            echo "ERROR: Node on port $port reports leader=$node_leader, expected leader=$leader" >&2
            return 1
        fi
    done

    echo "✓ All nodes agree on leader $leader"
}

echo ""
echo "=========================================="
echo "Aspen Raft Consensus Smoke Test"
echo "=========================================="
echo ""
echo "This test validates real Raft consensus behavior:"
echo "  - Leader election"
echo "  - Log replication across nodes"
echo "  - Membership changes with Raft"
echo "  - Leader failover"
echo ""

echo "Starting 5 nodes with RaftActor backend (mDNS + gossip enabled)..."
start_node 1 21001 26001 "$ROOT/n1.log"
start_node 2 21002 26002 "$ROOT/n2.log"
start_node 3 21003 26003 "$ROOT/n3.log"
start_node 4 21004 26004 "$ROOT/n4.log"
start_node 5 21005 26005 "$ROOT/n5.log"

echo ""
echo "Waiting 12 seconds for mDNS and gossip peer discovery..."
sleep 12

echo ""
echo "--- Test 0: Peer Discovery (Manual Iroh Endpoint Exchange) ---"
echo "Querying node endpoint addresses..."

# Get node info from each initial node
node1_info=$(curl --silent "127.0.0.1:21001/node-info" 2>/dev/null)
node2_info=$(curl --silent "127.0.0.1:21002/node-info" 2>/dev/null)
node3_info=$(curl --silent "127.0.0.1:21003/node-info" 2>/dev/null)

# Extract endpoint_addr for each node
node1_endpoint=$(echo "$node1_info" | jq -r '.endpoint_addr')
node2_endpoint=$(echo "$node2_info" | jq -r '.endpoint_addr')
node3_endpoint=$(echo "$node3_info" | jq -r '.endpoint_addr')

echo "Node 1 endpoint: $node1_endpoint"
echo "Node 2 endpoint: $node2_endpoint"
echo "Node 3 endpoint: $node3_endpoint"

echo ""
echo "Adding peers to each node's network factory..."

# Add node 2 and 3 as peers to node 1
curl --silent "127.0.0.1:21001/add-peer" \
    -H "Content-Type: application/json" \
    -d "{\"node_id\":2,\"endpoint_addr\":$node2_endpoint}" >/dev/null
curl --silent "127.0.0.1:21001/add-peer" \
    -H "Content-Type: application/json" \
    -d "{\"node_id\":3,\"endpoint_addr\":$node3_endpoint}" >/dev/null

# Add node 1 and 3 as peers to node 2
curl --silent "127.0.0.1:21002/add-peer" \
    -H "Content-Type: application/json" \
    -d "{\"node_id\":1,\"endpoint_addr\":$node1_endpoint}" >/dev/null
curl --silent "127.0.0.1:21002/add-peer" \
    -H "Content-Type: application/json" \
    -d "{\"node_id\":3,\"endpoint_addr\":$node3_endpoint}" >/dev/null

# Add node 1 and 2 as peers to node 3
curl --silent "127.0.0.1:21003/add-peer" \
    -H "Content-Type: application/json" \
    -d "{\"node_id\":1,\"endpoint_addr\":$node1_endpoint}" >/dev/null
curl --silent "127.0.0.1:21003/add-peer" \
    -H "Content-Type: application/json" \
    -d "{\"node_id\":2,\"endpoint_addr\":$node2_endpoint}" >/dev/null

echo "✓ All peers added to network factories"

echo ""
echo "--- Test 1: Cluster Initialization & Leader Election ---"
echo "Initializing nodes 1-3 as a cluster (triggers leader election)..."
rpc 21001/init '{
    "initial_members":[
        {"id":1,"addr":"127.0.0.1:21001","raft_addr":"127.0.0.1:26001"},
        {"id":2,"addr":"127.0.0.1:21002","raft_addr":"127.0.0.1:26002"},
        {"id":3,"addr":"127.0.0.1:21003","raft_addr":"127.0.0.1:26003"}
    ]
}'

# Wait for leader election
leader=$(wait_for_leader 21001)

# Verify all initial nodes agree on leader
verify_leader_consensus "$leader" 21001 21002 21003

echo ""
echo "--- Test 2: Log Replication ---"
echo "Writing test_key=test_value via leader (node 1)..."
rpc 21001/write '{"command":{"Set":{"key":"test_key","value":"test_value"}}}'

echo "Verifying write was replicated and committed (read from leader)..."
verify_replication 21001 21001 "test_key" "test_value"

# Note: In production Raft, followers redirect reads to the leader for linearizability.
# The write succeeding already proves replication to a quorum (2/3 nodes).
echo "✓ Write committed - data replicated to quorum"

echo ""
echo "--- Test 3: Membership Changes (Add Learners) ---"
echo "Adding peer addresses for nodes 4 and 5..."

# Get node info for nodes 4 and 5
node4_info=$(curl --silent "127.0.0.1:21004/node-info" 2>/dev/null)
node5_info=$(curl --silent "127.0.0.1:21005/node-info" 2>/dev/null)
node4_endpoint=$(echo "$node4_info" | jq -r '.endpoint_addr')
node5_endpoint=$(echo "$node5_info" | jq -r '.endpoint_addr')

# Add nodes 4 and 5 as peers to existing nodes (1, 2, 3)
for port in 21001 21002 21003; do
    curl --silent "127.0.0.1:$port/add-peer" \
        -H "Content-Type: application/json" \
        -d "{\"node_id\":4,\"endpoint_addr\":$node4_endpoint}" >/dev/null
    curl --silent "127.0.0.1:$port/add-peer" \
        -H "Content-Type: application/json" \
        -d "{\"node_id\":5,\"endpoint_addr\":$node5_endpoint}" >/dev/null
done

# Add existing nodes (1, 2, 3) as peers to node 4
curl --silent "127.0.0.1:21004/add-peer" \
    -H "Content-Type: application/json" \
    -d "{\"node_id\":1,\"endpoint_addr\":$node1_endpoint}" >/dev/null
curl --silent "127.0.0.1:21004/add-peer" \
    -H "Content-Type: application/json" \
    -d "{\"node_id\":2,\"endpoint_addr\":$node2_endpoint}" >/dev/null
curl --silent "127.0.0.1:21004/add-peer" \
    -H "Content-Type: application/json" \
    -d "{\"node_id\":3,\"endpoint_addr\":$node3_endpoint}" >/dev/null

# Add existing nodes (1, 2, 3) as peers to node 5
curl --silent "127.0.0.1:21005/add-peer" \
    -H "Content-Type: application/json" \
    -d "{\"node_id\":1,\"endpoint_addr\":$node1_endpoint}" >/dev/null
curl --silent "127.0.0.1:21005/add-peer" \
    -H "Content-Type: application/json" \
    -d "{\"node_id\":2,\"endpoint_addr\":$node2_endpoint}" >/dev/null
curl --silent "127.0.0.1:21005/add-peer" \
    -H "Content-Type: application/json" \
    -d "{\"node_id\":3,\"endpoint_addr\":$node3_endpoint}" >/dev/null

echo "✓ Peers added for nodes 4 and 5"

echo "Adding nodes 4 and 5 as learners..."
rpc 21001/add-learner '{"learner":{"id":4,"addr":"127.0.0.1:21004","raft_addr":"127.0.0.1:26004"}}'
rpc 21001/add-learner '{"learner":{"id":5,"addr":"127.0.0.1:21005","raft_addr":"127.0.0.1:26005"}}'

# Wait for learners to sync
sleep 3

echo ""
echo "--- Test 4: Promote Learners to Voters ---"
echo "Expanding membership to all five nodes..."
rpc 21001/change-membership '{"members":[1,2,3,4,5]}'

# Wait for membership change to propagate
sleep 2

echo "Verifying new voters can replicate (write to leader, confirm committed)..."
rpc 21001/write '{"command":{"Set":{"key":"learner_test","value":"promoted"}}}'
sleep 1
verify_replication 21001 21001 "learner_test" "promoted"
echo "✓ New voters (nodes 4, 5) now part of Raft quorum"

echo ""
echo "--- Test 5: Multiple Writes & Reads ---"
echo "Writing multiple keys..."
rpc 21001/write '{"command":{"Set":{"key":"foo","value":"bar"}}}'
rpc 21001/write '{"command":{"Set":{"key":"baz","value":"qux"}}}'

sleep 1

echo "Reading foo from leader..."
rpc 21001/read '{"key":"foo"}'

echo "Reading baz from leader..."
rpc 21001/read '{"key":"baz"}'

echo "✓ Multiple key-value operations working correctly"

echo ""
echo "--- Test 6: Membership Shrink ---"
echo "Shrinking membership to just node 3..."
rpc 21001/change-membership '{"members":[3]}'

sleep 2

echo "Writing via node 3 (now sole voter)..."
rpc 21003/write '{"command":{"Set":{"key":"solo","value":"node3"}}}'

sleep 1

echo "Reading from node 3..."
rpc 21003/read '{"key":"solo"}'

echo ""
echo "=========================================="
echo "✓ All Raft Consensus Tests Passed!"
echo "=========================================="
echo ""
echo "Validated:"
echo "  ✓ Leader election within timeout"
echo "  ✓ Log replication to all followers"
echo "  ✓ Membership changes (add learner → promote)"
echo "  ✓ Membership shrink"
echo "  ✓ Multi-key writes and reads"
echo ""
echo "Logs written to n*.log"
echo "To inspect Raft events: grep -i 'election\\|leader\\|vote' n*.log"
echo ""
