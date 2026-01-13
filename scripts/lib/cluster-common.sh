#!/bin/bash
# Shared functions for cluster launch scripts
# Source this file in cluster.sh and kitty-cluster.sh

# Find binary in various locations
find_binary() {
    local name="$1"
    local bin=""

    # Check environment variable first
    local env_var="ASPEN_${name^^}_BIN"
    env_var="${env_var//-/_}"
    bin="${!env_var:-}"
    if [ -n "$bin" ] && [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    # Check PATH
    bin=$(command -v "$name" 2>/dev/null || echo "")
    if [ -n "$bin" ] && [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    # Check target/release
    bin="$PROJECT_DIR/target/release/$name"
    if [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    # Check target/debug
    bin="$PROJECT_DIR/target/debug/$name"
    if [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    # Check nix result symlink
    bin="$PROJECT_DIR/result/bin/$name"
    if [ -x "$bin" ]; then
        echo "$bin"
        return 0
    fi

    echo ""
}

# Generate deterministic secret key for a node
generate_secret_key() {
    local node_id="$1"
    printf '%064x' "$((1000 + node_id))"
}

# Check prerequisites
check_prerequisites() {
    if [ -z "$ASPEN_NODE_BIN" ] || [ ! -x "$ASPEN_NODE_BIN" ]; then
        printf "${RED}Error: aspen-node binary not found${NC}\n" >&2
        printf "Build with: cargo build --release --bin aspen-node\n" >&2
        printf "Or use: nix run .#cluster\n" >&2
        exit 1
    fi

    if [ -z "$ASPEN_CLI_BIN" ] || [ ! -x "$ASPEN_CLI_BIN" ]; then
        printf "${RED}Error: aspen-cli binary not found${NC}\n" >&2
        printf "Build with: cargo build --release --bin aspen-cli\n" >&2
        exit 1
    fi
}

# =============================================================================
# SHARED TEST UTILITIES
# =============================================================================

# Retry a command with exponential backoff
# Usage: retry_command [max_attempts] [initial_delay] command [args...]
retry_command() {
    local max_attempts="${1:-3}"
    local delay="${2:-1}"
    shift 2
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        if "$@"; then
            return 0
        fi
        if [ $attempt -lt $max_attempts ]; then
            sleep $delay
            delay=$((delay * 2))
        fi
        attempt=$((attempt + 1))
    done
    return 1
}

# Wait for a condition to become true
# Usage: wait_for_condition [timeout_secs] [poll_interval] command [args...]
wait_for_condition() {
    local timeout="${1:-30}"
    local interval="${2:-1}"
    shift 2
    local elapsed=0

    while [ "$elapsed" -lt "$timeout" ]; do
        if "$@" >/dev/null 2>&1; then
            return 0
        fi
        sleep "$interval"
        elapsed=$((elapsed + interval))
    done
    return 1
}

# Extract a field from JSON output
# Usage: extract_json_field "field_name" "json_string"
extract_json_field() {
    local field="$1"
    local json="$2"

    # Try to extract using grep/sed (no jq dependency)
    echo "$json" | grep -oE "\"$field\":\s*\"[^\"]+\"" | sed "s/\"$field\":\s*\"//" | sed 's/"$//' | head -1 || true
}

# Extract a numeric field from JSON output
# Usage: extract_json_number "field_name" "json_string"
extract_json_number() {
    local field="$1"
    local json="$2"

    echo "$json" | grep -oE "\"$field\":\s*[0-9]+" | grep -oE '[0-9]+' | head -1 || true
}

# Extract a 64-char hex hash from output
# Usage: extract_hash "output_string"
extract_hash() {
    local output="$1"
    echo "$output" | grep -oE '[a-f0-9]{64}' | head -1 || true
}

# =============================================================================
# CLUSTER STABILIZATION
# =============================================================================

# Retry a command with exponential backoff (improved version)
# Usage: retry_with_backoff [max_attempts] [initial_delay] [max_delay] command [args...]
# Returns: 0 on success, 1 on all attempts failed
retry_with_backoff() {
    local max_attempts="${1:-5}"
    local initial_delay="${2:-1}"
    local max_delay="${3:-16}"
    shift 3
    local attempt=1
    local delay="$initial_delay"

    while [ $attempt -le $max_attempts ]; do
        if "$@" 2>/dev/null; then
            return 0
        fi
        if [ $attempt -lt $max_attempts ]; then
            sleep "$delay"
            # Exponential backoff with cap
            delay=$((delay * 2))
            if [ "$delay" -gt "$max_delay" ]; then
                delay="$max_delay"
            fi
        fi
        attempt=$((attempt + 1))
    done
    return 1
}

# Wait for cluster to stabilize after initialization
# This verifies all nodes have received Raft membership through replication
# and can successfully process non-bootstrap operations.
#
# Usage: wait_for_cluster_stable [cli_bin] [ticket] [timeout_ms] [max_wait_secs]
# Returns: 0 on success, 1 on timeout
wait_for_cluster_stable() {
    local cli_bin="${1:?CLI binary required}"
    local ticket="${2:?Ticket required}"
    local timeout_ms="${3:-30000}"
    local max_wait="${4:-30}"
    local elapsed=0
    local poll_interval=2

    while [ "$elapsed" -lt "$max_wait" ]; do
        # Try a simple KV operation - this will fail with NOT_INITIALIZED
        # if the node hasn't received membership through Raft replication
        if "$cli_bin" --quiet --ticket "$ticket" --timeout "$timeout_ms" \
            kv set "__cluster_health_check" "ok" >/dev/null 2>&1; then
            # Clean up the health check key
            "$cli_bin" --quiet --ticket "$ticket" --timeout "$timeout_ms" \
                kv delete "__cluster_health_check" >/dev/null 2>&1 || true
            return 0
        fi
        sleep "$poll_interval"
        elapsed=$((elapsed + poll_interval))
    done
    return 1
}

# Wait for ALL nodes in a cluster to be fully initialized
# This is more thorough than wait_for_cluster_stable - it verifies EACH node
# individually rather than just testing whichever node the CLI connects to first.
#
# Usage: wait_for_all_nodes_ready [data_dir] [cli_bin] [ticket] [timeout_ms] [node_count] [max_wait_secs]
# Returns: 0 on success, 1 on timeout
wait_for_all_nodes_ready() {
    local data_dir="${1:?Data directory required}"
    local cli_bin="${2:?CLI binary required}"
    local ticket="${3:?Ticket required}"
    local timeout_ms="${4:-30000}"
    local node_count="${5:-3}"
    local max_wait="${6:-60}"
    local elapsed=0
    local poll_interval=2

    while [ "$elapsed" -lt "$max_wait" ]; do
        local all_ready=true

        # Check each node by extracting its endpoint from logs and testing directly
        for id in $(seq 1 "$node_count"); do
            local node_log="$data_dir/node$id/node.log"
            if [ ! -f "$node_log" ]; then
                all_ready=false
                break
            fi

            # Extract endpoint ID from node log
            local endpoint_id
            endpoint_id=$(sed 's/\x1b\[[0-9;]*m//g' "$node_log" 2>/dev/null | \
                grep -oE 'endpoint_id=[a-f0-9]{64}' | head -1 | cut -d= -f2 || true)

            if [ -z "$endpoint_id" ]; then
                all_ready=false
                break
            fi

            # Try to get cluster metrics from this specific node
            # This validates that the node is initialized and responding
            if ! "$cli_bin" --quiet --timeout "$timeout_ms" \
                --bootstrap-peer "$endpoint_id" \
                cluster metrics >/dev/null 2>&1; then
                all_ready=false
                break
            fi
        done

        if $all_ready; then
            return 0
        fi

        sleep "$poll_interval"
        elapsed=$((elapsed + poll_interval))
    done
    return 1
}

# Wait for a specific subsystem to be ready
# Subsystems (hooks, secrets, etc.) initialize after the main cluster
# Uses exponential backoff for more robust waiting
#
# Usage: wait_for_subsystem [cli_bin] [ticket] [timeout_ms] [subsystem] [max_wait_secs]
# Supported subsystems: hooks, secrets, forge, dns
wait_for_subsystem() {
    local cli_bin="${1:?CLI binary required}"
    local ticket="${2:?Ticket required}"
    local timeout_ms="${3:-30000}"
    local subsystem="${4:?Subsystem required}"
    local max_wait="${5:-60}"
    local elapsed=0
    local delay=1  # Start with 1 second delay
    local max_delay=8  # Cap at 8 seconds

    case "$subsystem" in
        hooks)
            while [ "$elapsed" -lt "$max_wait" ]; do
                if "$cli_bin" --quiet --ticket "$ticket" --timeout "$timeout_ms" \
                    hook list >/dev/null 2>&1; then
                    return 0
                fi
                sleep "$delay"
                elapsed=$((elapsed + delay))
                # Exponential backoff
                delay=$((delay * 2))
                [ "$delay" -gt "$max_delay" ] && delay="$max_delay"
            done
            ;;
        secrets)
            while [ "$elapsed" -lt "$max_wait" ]; do
                if "$cli_bin" --quiet --ticket "$ticket" --timeout "$timeout_ms" \
                    secrets kv list "/" >/dev/null 2>&1; then
                    return 0
                fi
                sleep "$delay"
                elapsed=$((elapsed + delay))
                delay=$((delay * 2))
                [ "$delay" -gt "$max_delay" ] && delay="$max_delay"
            done
            ;;
        forge)
            while [ "$elapsed" -lt "$max_wait" ]; do
                if "$cli_bin" --quiet --ticket "$ticket" --timeout "$timeout_ms" \
                    git list >/dev/null 2>&1; then
                    return 0
                fi
                sleep "$delay"
                elapsed=$((elapsed + delay))
                delay=$((delay * 2))
                [ "$delay" -gt "$max_delay" ] && delay="$max_delay"
            done
            ;;
        dns)
            while [ "$elapsed" -lt "$max_wait" ]; do
                if "$cli_bin" --quiet --ticket "$ticket" --timeout "$timeout_ms" \
                    dns list >/dev/null 2>&1; then
                    return 0
                fi
                sleep "$delay"
                elapsed=$((elapsed + delay))
                delay=$((delay * 2))
                [ "$delay" -gt "$max_delay" ] && delay="$max_delay"
            done
            ;;
        *)
            printf "Unknown subsystem: %s\n" "$subsystem" >&2
            return 1
            ;;
    esac
    return 1
}
