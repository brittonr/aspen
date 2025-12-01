#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="$ROOT/target/hiqlite"
SRC_DIR="$TARGET_DIR/src"
BIN_DIR="$TARGET_DIR/bin"
RUN_DIR="$TARGET_DIR/run"
LOG_DIR="$RUN_DIR/logs"

HQL_REPO="${HQL_REPO:-https://github.com/sebadob/hiqlite}"
HQL_TAG="${HQL_TAG:-v0.11.1}"
HQL_BIN="$BIN_DIR/hiqlite"

NODE_COUNT="${HIQLITE_NODE_COUNT:-3}"
API_BASE_PORT="${HIQLITE_API_BASE_PORT:-29001}"
RAFT_BASE_PORT="${HIQLITE_RAFT_BASE_PORT:-39001}"
HIQLITE_SECRET="${HIQLITE_SECRET:-aspen-hiqlite-dev}"

generate_enc_key_material() {
    if command -v python3 >/dev/null 2>&1; then
        python3 - <<'PY'
import base64, secrets
print(base64.b64encode(secrets.token_bytes(32)).decode())
PY
    else
        dd if=/dev/urandom bs=32 count=1 2>/dev/null | base64
    fi
}

generate_enc_key_id() {
    if command -v python3 >/dev/null 2>&1; then
        python3 - <<'PY'
import secrets, string
alphabet = string.ascii_letters + string.digits
print(''.join(secrets.choice(alphabet) for _ in range(16)))
PY
    else
        tr -dc 'A-Za-z0-9' </dev/urandom | head -c 16
    fi
}

HIQLITE_ENC_KEY_ID="${HIQLITE_ENC_KEY_ID:-$(generate_enc_key_id)}"
HIQLITE_ENC_KEY_MATERIAL="${HIQLITE_ENC_KEY_MATERIAL:-$(generate_enc_key_material)}"

mkdir -p "$BIN_DIR" "$RUN_DIR" "$LOG_DIR"

build_hiqlite() {
    if [[ ! -d "$SRC_DIR/.git" ]]; then
        rm -rf "$SRC_DIR"
        git clone --depth 1 --branch "$HQL_TAG" "$HQL_REPO" "$SRC_DIR"
    else
        git -C "$SRC_DIR" fetch --tags --force
        git -C "$SRC_DIR" checkout "$HQL_TAG"
    fi
    (
        cd "$SRC_DIR"
        cargo build --release --bin hiqlite --features server
    )
    cp "$SRC_DIR/target/release/hiqlite" "$HQL_BIN"
    chmod +x "$HQL_BIN"
}

generate_config() {
    local id=$1
    local config_path="$RUN_DIR/node${id}.toml"
    local data_dir="$RUN_DIR/node${id}"

    mkdir -p "$data_dir"

    {
        cat <<EOF
[hiqlite]
node_id = ${id}
listen_addr_api = "127.0.0.1"
listen_addr_raft = "127.0.0.1"
data_dir = "${data_dir}"
secret_api = "${HIQLITE_SECRET}"
secret_raft = "${HIQLITE_SECRET}"
nodes = [
EOF
        for idx in $(seq 1 "$NODE_COUNT"); do
            local api=$((API_BASE_PORT + idx - 1))
            local raft=$((RAFT_BASE_PORT + idx - 1))
            printf '    "%d 127.0.0.1:%s 127.0.0.1:%s"' "$idx" "$raft" "$api"
            if [[ $idx -lt $NODE_COUNT ]]; then
                printf ',\n'
            else
                printf '\n'
            fi
        done
        cat <<EOF
]
enc_keys = [
    "${HIQLITE_ENC_KEY_ID}/${HIQLITE_ENC_KEY_MATERIAL}"
]
enc_key_active = "${HIQLITE_ENC_KEY_ID}"
password_dashboard = "JGFyZ29uMmlkJHY9MTkkbT0zMix0PTIscD0xJE9FbFZURnAwU0V0bFJ6ZFBlSEZDT0EkTklCN0txTy8vanB4WFE5bUdCaVM2SlhraEpwaWVYOFRUNW5qdG9wcXkzQQ=="
insecure_cookie = true
EOF
    } >"$config_path"
}

start_node() {
    local id=$1
    local config_path="$RUN_DIR/node${id}.toml"
    local log_path="$LOG_DIR/node${id}.log"

    echo "Starting Hiqlite node ${id} (config: ${config_path})"
    nohup "$HQL_BIN" serve --config-file "$config_path" >"$log_path" 2>&1 &
    local pid=$!
    echo $pid >"$RUN_DIR/node${id}.pid"
    pids+=("$pid")
}

cleanup() {
    echo "Stopping Hiqlite nodes..."
    for pid_file in "$RUN_DIR"/node*.pid; do
        [[ -f "$pid_file" ]] || continue
        local pid
        pid=$(cat "$pid_file")
        if kill "$pid" >/dev/null 2>&1; then
            wait "$pid" 2>/dev/null || true
        fi
        rm -f "$pid_file"
    done
}

trap cleanup EXIT

build_hiqlite

pids=()
for id in $(seq 1 "$NODE_COUNT"); do
    api_port=$((API_BASE_PORT + id - 1))
    raft_port=$((RAFT_BASE_PORT + id - 1))
    generate_config "$id" "$api_port" "$raft_port"
    start_node "$id"
done

node_args=()
for id in $(seq 1 "$NODE_COUNT"); do
    api_port=$((API_BASE_PORT + id - 1))
    node_args+=("--hiqlite-node" "127.0.0.1:${api_port}")
done

echo
echo "Hiqlite cluster is running with secret '${HIQLITE_SECRET}'."
echo "Use these flags when launching aspen-node:"
echo "    ${node_args[*]} --hiqlite-api-secret ${HIQLITE_SECRET}"
echo
echo "Logs: ${LOG_DIR}"
echo "Press Ctrl-C to stop the cluster."

wait
