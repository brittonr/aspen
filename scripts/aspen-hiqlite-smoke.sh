#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
HIQLITE_NODE_COUNT="${HIQLITE_NODE_COUNT:-3}"
HIQLITE_API_BASE_PORT="${HIQLITE_API_BASE_PORT:-29501}"
HIQLITE_RAFT_BASE_PORT="${HIQLITE_RAFT_BASE_PORT:-39501}"
HIQLITE_SECRET="${HIQLITE_SECRET:-aspen-hiqlite-smoke}"
HIQLITE_LOG="${ROOT}/target/hiqlite-smoke.log"

wait_for_port() {
    local port=$1
    local label=$2
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
            echo "Timed out waiting for ${label} on 127.0.0.1:${port}" >&2
            exit 1
        fi
    done
}

HIQLITE_WRAPPER_PID=""
cleanup() {
    if [[ -n "${HIQLITE_WRAPPER_PID}" ]]; then
        kill "${HIQLITE_WRAPPER_PID}" >/dev/null 2>&1 || true
        wait "${HIQLITE_WRAPPER_PID}" 2>/dev/null || true
    fi
}
trap cleanup EXIT

echo "Starting Hiqlite cluster (logs: ${HIQLITE_LOG})"
env \
    HIQLITE_NODE_COUNT="${HIQLITE_NODE_COUNT}" \
    HIQLITE_API_BASE_PORT="${HIQLITE_API_BASE_PORT}" \
    HIQLITE_RAFT_BASE_PORT="${HIQLITE_RAFT_BASE_PORT}" \
    HIQLITE_SECRET="${HIQLITE_SECRET}" \
    "${ROOT}/scripts/run-hiqlite-cluster.sh" \
    >"${HIQLITE_LOG}" 2>&1 &
HIQLITE_WRAPPER_PID=$!

for id in $(seq 1 "${HIQLITE_NODE_COUNT}"); do
    api_port=$((HIQLITE_API_BASE_PORT + id - 1))
    wait_for_port "${api_port}" "Hiqlite API node ${id}"
done

node_args=()
for id in $(seq 1 "${HIQLITE_NODE_COUNT}"); do
    api_port=$((HIQLITE_API_BASE_PORT + id - 1))
    node_args+=("--hiqlite-node" "127.0.0.1:${api_port}")
done
node_args+=("--hiqlite-api-secret" "${HIQLITE_SECRET}")

echo "Running Aspen cluster smoke against Hiqlite backend"
"${ROOT}/scripts/aspen-cluster-smoke.sh" \
    --control-backend hiqlite \
    "${node_args[@]}"

echo "Hiqlite-backed smoke test complete."
