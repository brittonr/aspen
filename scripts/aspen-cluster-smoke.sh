#!/usr/bin/env bash
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

echo "Cleaning up old data directories"
rm -rf "$ROOT/data"

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
    local log_file=$3
    printf -v secret "%064x" "$((1000 + id))"
    local cmd=( "$BIN"
        --node-id "$id"
        --http-addr "127.0.0.1:$http"
        --host "127.0.0.1"
        --cookie "aspen-dev-cookie"
        --iroh-secret-key "$secret"
        --control-backend "deterministic"
        --data-dir "$ROOT/data/node-$id"
        --storage-backend "sqlite"
    )
    if (( ${#EXTRA_NODE_ARGS[@]} > 0 )); then
        cmd+=("${EXTRA_NODE_ARGS[@]}")
    fi
    # Create data directory
    mkdir -p "$ROOT/data/node-$id"
    nohup "${cmd[@]}" >"$log_file" 2>&1 &
    sleep 1
    wait_for_http_port "$http"
    echo "Node $id listening on 127.0.0.1:$http"
}

echo "Starting 5 nodes..."
start_node 1 21001 "$ROOT/n1.log"
start_node 2 21002 "$ROOT/n2.log"
start_node 3 21003 "$ROOT/n3.log"
start_node 4 21004 "$ROOT/n4.log"
start_node 5 21005 "$ROOT/n5.log"

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
rpc 21001/write '{"command":{"Set":{"key":"foo","value":"bar"}}}'

echo "Reading foo from node 1"
rpc 21001/read '{"key":"foo"}'

echo "Shrinking membership to just node 3"
rpc 21001/change-membership '{"members":[3]}'
rpc 21003/metrics

echo "Writing foo=zoo via node 3"
rpc 21003/write '{"command":{"Set":{"key":"foo","value":"zoo"}}}'

echo "Reading foo from node 3"
rpc 21003/read '{"key":"foo"}'

echo "Cluster smoke test complete. Logs written to n*.log."
