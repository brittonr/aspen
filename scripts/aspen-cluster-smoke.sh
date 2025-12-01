#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/debug/aspen-node"

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
        if command -v jq >/dev/null 2>&1; then
            jq
        else
            cat
        fi
    }
    echo
    echo
}

kill_nodes() {
    if [[ "$(uname)" == "Darwin" ]]; then
        if pgrep -xq -- "aspen-node"; then
            pkill -f "aspen-node"
        fi
    else
        set +e
        killall aspen-node >/dev/null 2>&1
        set -e
    fi
}

trap kill_nodes EXIT

export RUST_LOG="${RUST_LOG:-info}"
export RUST_BACKTRACE=full

echo "Building aspen-node..."
cargo build --bin aspen-node >/dev/null

echo "Killing any existing nodes"
kill_nodes
sleep 1

start_node() {
    local id=$1
    local http=$2
    local cluster_port=$3
    local log_file=$4
    nohup "$BIN" \
        --id "$id" \
        --http-addr "127.0.0.1:$http" \
        --cluster-host "127.0.0.1" \
        --cluster-port "$cluster_port" \
        --cookie "aspen-dev-cookie" \
        >"$log_file" 2>&1 &
    sleep 1
    echo "Node $id listening on 127.0.0.1:$http"
}

echo "Starting 5 nodes..."
start_node 1 21001 26001 "$ROOT/n1.log"
start_node 2 21002 26002 "$ROOT/n2.log"
start_node 3 21003 26003 "$ROOT/n3.log"
start_node 4 21004 26004 "$ROOT/n4.log"
start_node 5 21005 26005 "$ROOT/n5.log"

echo "Initializing nodes 1-3 as a cluster"
rpc 21001/init '{"initial_members":[{"id":1,"addr":"127.0.0.1:21001"},{"id":2,"addr":"127.0.0.1:21002"},{"id":3,"addr":"127.0.0.1:21003"}]}'

echo "Current metrics on node 1"
rpc 21001/metrics

echo "Adding nodes 4 and 5 as learners"
rpc 21001/add-learner '{"learner":{"id":4,"addr":"127.0.0.1:21004"}}'
rpc 21001/add-learner '{"learner":{"id":5,"addr":"127.0.0.1:21005"}}'
rpc 21001/metrics

echo "Expanding membership to all five nodes"
rpc 21001/change-membership '{"members":[1,2,3,4,5]}'
rpc 21001/metrics

echo "Writing foo=bar via node 1"
rpc 21001/write '{"command":{"type":"set","key":"foo","value":"bar"}}'

echo "Reading foo from node 1"
rpc 21001/read '{"key":"foo"}'

echo "Shrinking membership to just node 3"
rpc 21001/change-membership '{"members":[3]}'
rpc 21003/metrics

echo "Writing foo=zoo via node 3"
rpc 21003/write '{"command":{"type":"set","key":"foo","value":"zoo"}}'

echo "Reading foo from node 3"
rpc 21003/read '{"key":"foo"}'

echo "Cluster smoke test complete. Logs written to n*.log."
