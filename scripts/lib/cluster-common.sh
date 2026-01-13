#!/bin/bash
# Shared functions for cluster launch scripts
# Source this file in cluster.sh and kitty-cluster.sh

# =============================================================================
# CONFIGURABLE DEFAULTS (override via environment variables)
# =============================================================================

# Timeout for RPC operations (milliseconds)
ASPEN_RPC_TIMEOUT="${ASPEN_RPC_TIMEOUT:-30000}"

# Timeout for cluster stabilization (seconds)
ASPEN_CLUSTER_STABLE_TIMEOUT="${ASPEN_CLUSTER_STABLE_TIMEOUT:-60}"

# Timeout for subsystem initialization (seconds)
ASPEN_SUBSYSTEM_TIMEOUT="${ASPEN_SUBSYSTEM_TIMEOUT:-90}"

# Timeout for waiting on conditions (seconds)
ASPEN_CONDITION_TIMEOUT="${ASPEN_CONDITION_TIMEOUT:-30}"

# Exponential backoff configuration
ASPEN_BACKOFF_INITIAL="${ASPEN_BACKOFF_INITIAL:-1}"     # Initial delay (seconds)
ASPEN_BACKOFF_MAX="${ASPEN_BACKOFF_MAX:-8}"             # Maximum delay (seconds)
ASPEN_BACKOFF_MULTIPLIER="${ASPEN_BACKOFF_MULTIPLIER:-2}"  # Backoff multiplier

# Poll interval for fixed-interval waits (seconds)
ASPEN_POLL_INTERVAL="${ASPEN_POLL_INTERVAL:-2}"

# Diagnostic logging (set to "1" or "true" to enable verbose wait logging)
ASPEN_WAIT_VERBOSE="${ASPEN_WAIT_VERBOSE:-0}"

# =============================================================================
# UTILITY FUNCTIONS
# =============================================================================

# Log a diagnostic message if verbose mode is enabled
# Usage: log_wait_diagnostic "message"
log_wait_diagnostic() {
    if [ "$ASPEN_WAIT_VERBOSE" = "1" ] || [ "$ASPEN_WAIT_VERBOSE" = "true" ]; then
        printf "[WAIT] %s\n" "$1" >&2
    fi
}

# Log a warning message (always shown)
# Usage: log_wait_warning "message"
log_wait_warning() {
    printf "[WAIT WARNING] %s\n" "$1" >&2
}

# Log an error message (always shown)
# Usage: log_wait_error "message"
log_wait_error() {
    printf "[WAIT ERROR] %s\n" "$1" >&2
}

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

# Wait for a condition to become true (uses configurable timeout)
# Usage: wait_for_condition [timeout_secs] [poll_interval] command [args...]
wait_for_condition() {
    local timeout="${1:-$ASPEN_CONDITION_TIMEOUT}"
    local interval="${2:-$ASPEN_POLL_INTERVAL}"
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
# EXPONENTIAL BACKOFF HELPER
# =============================================================================

# Generic exponential backoff wait loop
# Usage: wait_with_backoff [max_wait_secs] [check_command_string] [context_name]
# Note: check_command_string is eval'd, so pass quoted command
#
# Uses configured ASPEN_BACKOFF_* environment variables
# Returns: 0 on success, 1 on timeout
wait_with_backoff() {
    local max_wait="${1:-$ASPEN_SUBSYSTEM_TIMEOUT}"
    local check_cmd="$2"
    local context="${3:-operation}"

    local elapsed=0
    local delay="$ASPEN_BACKOFF_INITIAL"
    local attempt=1

    log_wait_diagnostic "Starting wait for $context (max ${max_wait}s, initial delay ${delay}s)"

    while [ "$elapsed" -lt "$max_wait" ]; do
        log_wait_diagnostic "Attempt $attempt for $context (elapsed: ${elapsed}s)"

        if eval "$check_cmd" >/dev/null 2>&1; then
            log_wait_diagnostic "Success: $context ready after ${elapsed}s ($attempt attempts)"
            return 0
        fi

        # Sleep and update elapsed time
        sleep "$delay"
        elapsed=$((elapsed + delay))
        attempt=$((attempt + 1))

        # Exponential backoff with cap
        delay=$((delay * ASPEN_BACKOFF_MULTIPLIER))
        if [ "$delay" -gt "$ASPEN_BACKOFF_MAX" ]; then
            delay="$ASPEN_BACKOFF_MAX"
        fi
    done

    log_wait_error "Timeout waiting for $context after ${max_wait}s ($attempt attempts)"
    return 1
}

# =============================================================================
# CLUSTER STABILIZATION
# =============================================================================

# Retry a command with exponential backoff (improved version)
# Usage: retry_with_backoff [max_attempts] [initial_delay] [max_delay] command [args...]
# Returns: 0 on success, 1 on all attempts failed
retry_with_backoff() {
    local max_attempts="${1:-5}"
    local initial_delay="${2:-$ASPEN_BACKOFF_INITIAL}"
    local max_delay="${3:-$ASPEN_BACKOFF_MAX}"
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
            delay=$((delay * ASPEN_BACKOFF_MULTIPLIER))
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
# Uses exponential backoff for more robust waiting.
#
# Usage: wait_for_cluster_stable [cli_bin] [ticket] [timeout_ms] [max_wait_secs]
# Returns: 0 on success, 1 on timeout
wait_for_cluster_stable() {
    local cli_bin="${1:?CLI binary required}"
    local ticket="${2:?Ticket required}"
    local timeout_ms="${3:-$ASPEN_RPC_TIMEOUT}"
    local max_wait="${4:-$ASPEN_CLUSTER_STABLE_TIMEOUT}"

    local elapsed=0
    local delay="$ASPEN_BACKOFF_INITIAL"
    local attempt=1

    log_wait_diagnostic "Waiting for cluster stabilization (max ${max_wait}s)"

    while [ "$elapsed" -lt "$max_wait" ]; do
        log_wait_diagnostic "Cluster stable check attempt $attempt (elapsed: ${elapsed}s)"

        # Try a simple KV operation - this will fail with NOT_INITIALIZED
        # if the node hasn't received membership through Raft replication
        if "$cli_bin" --quiet --ticket "$ticket" --timeout "$timeout_ms" \
            kv set "__cluster_health_check" "ok" >/dev/null 2>&1; then
            # Clean up the health check key
            "$cli_bin" --quiet --ticket "$ticket" --timeout "$timeout_ms" \
                kv delete "__cluster_health_check" >/dev/null 2>&1 || true
            log_wait_diagnostic "Cluster stable after ${elapsed}s ($attempt attempts)"
            return 0
        fi

        sleep "$delay"
        elapsed=$((elapsed + delay))
        attempt=$((attempt + 1))

        # Exponential backoff with cap
        delay=$((delay * ASPEN_BACKOFF_MULTIPLIER))
        if [ "$delay" -gt "$ASPEN_BACKOFF_MAX" ]; then
            delay="$ASPEN_BACKOFF_MAX"
        fi
    done

    log_wait_error "Cluster did not stabilize within ${max_wait}s ($attempt attempts)"
    return 1
}

# Wait for ALL nodes in a cluster to be fully initialized
# This is more thorough than wait_for_cluster_stable - it verifies EACH node
# individually rather than just testing whichever node the CLI connects to first.
#
# Uses exponential backoff for more robust waiting.
#
# Usage: wait_for_all_nodes_ready [data_dir] [cli_bin] [ticket] [timeout_ms] [node_count] [max_wait_secs]
# Returns: 0 on success, 1 on timeout
wait_for_all_nodes_ready() {
    local data_dir="${1:?Data directory required}"
    local cli_bin="${2:?CLI binary required}"
    local ticket="${3:?Ticket required}"
    local timeout_ms="${4:-$ASPEN_RPC_TIMEOUT}"
    local node_count="${5:-3}"
    local max_wait="${6:-$ASPEN_CLUSTER_STABLE_TIMEOUT}"

    local elapsed=0
    local delay="$ASPEN_BACKOFF_INITIAL"
    local attempt=1

    log_wait_diagnostic "Waiting for all $node_count nodes to be ready (max ${max_wait}s)"

    while [ "$elapsed" -lt "$max_wait" ]; do
        local all_ready=true
        local failed_node=""

        # Check each node by extracting its endpoint from logs and testing directly
        for id in $(seq 1 "$node_count"); do
            local node_log="$data_dir/node$id/node.log"
            if [ ! -f "$node_log" ]; then
                all_ready=false
                failed_node="node$id (log not found)"
                break
            fi

            # Extract endpoint ID from node log
            local endpoint_id
            endpoint_id=$(sed 's/\x1b\[[0-9;]*m//g' "$node_log" 2>/dev/null | \
                grep -oE 'endpoint_id=[a-f0-9]{64}' | head -1 | cut -d= -f2 || true)

            if [ -z "$endpoint_id" ]; then
                all_ready=false
                failed_node="node$id (endpoint_id not found in log)"
                break
            fi

            # Try to get cluster metrics from this specific node
            # This validates that the node is initialized and responding
            if ! "$cli_bin" --quiet --timeout "$timeout_ms" \
                --bootstrap-peer "$endpoint_id" \
                cluster metrics >/dev/null 2>&1; then
                all_ready=false
                failed_node="node$id (metrics check failed)"
                break
            fi
        done

        if $all_ready; then
            log_wait_diagnostic "All $node_count nodes ready after ${elapsed}s ($attempt attempts)"
            return 0
        fi

        log_wait_diagnostic "Attempt $attempt: $failed_node (elapsed: ${elapsed}s)"

        sleep "$delay"
        elapsed=$((elapsed + delay))
        attempt=$((attempt + 1))

        # Exponential backoff with cap
        delay=$((delay * ASPEN_BACKOFF_MULTIPLIER))
        if [ "$delay" -gt "$ASPEN_BACKOFF_MAX" ]; then
            delay="$ASPEN_BACKOFF_MAX"
        fi
    done

    log_wait_error "Not all nodes ready within ${max_wait}s. Last failure: $failed_node"
    return 1
}

# =============================================================================
# SUBSYSTEM WAIT FUNCTIONS
# =============================================================================

# Internal helper for subsystem checks with consistent logging
# Usage: _check_subsystem [cli_bin] [ticket] [timeout_ms] [subsystem]
# Returns: 0 on success, 1 on failure (also sets _SUBSYSTEM_ERROR)
_check_subsystem() {
    local cli_bin="$1"
    local ticket="$2"
    local timeout_ms="$3"
    local subsystem="$4"
    local output

    _SUBSYSTEM_ERROR=""

    case "$subsystem" in
        hooks)
            output=$("$cli_bin" --quiet --ticket "$ticket" --timeout "$timeout_ms" \
                hook list 2>&1) && return 0
            _SUBSYSTEM_ERROR="$output"
            return 1
            ;;
        secrets)
            output=$("$cli_bin" --quiet --ticket "$ticket" --timeout "$timeout_ms" \
                secrets kv list "/" 2>&1) && return 0
            _SUBSYSTEM_ERROR="$output"
            return 1
            ;;
        forge)
            output=$("$cli_bin" --quiet --ticket "$ticket" --timeout "$timeout_ms" \
                git list 2>&1) && return 0
            _SUBSYSTEM_ERROR="$output"
            return 1
            ;;
        dns)
            output=$("$cli_bin" --quiet --ticket "$ticket" --timeout "$timeout_ms" \
                dns zone list 2>&1) && return 0
            _SUBSYSTEM_ERROR="$output"
            return 1
            ;;
        blob)
            output=$("$cli_bin" --quiet --ticket "$ticket" --timeout "$timeout_ms" \
                blob list --limit 1 2>&1) && return 0
            _SUBSYSTEM_ERROR="$output"
            return 1
            ;;
        job)
            output=$("$cli_bin" --quiet --ticket "$ticket" --timeout "$timeout_ms" \
                job list --limit 1 2>&1) && return 0
            _SUBSYSTEM_ERROR="$output"
            return 1
            ;;
        sql)
            output=$("$cli_bin" --quiet --ticket "$ticket" --timeout "$timeout_ms" \
                sql "SELECT 1" 2>&1) && return 0
            _SUBSYSTEM_ERROR="$output"
            return 1
            ;;
        *)
            _SUBSYSTEM_ERROR="Unknown subsystem: $subsystem"
            return 1
            ;;
    esac
}

# Wait for a specific subsystem to be ready
# Subsystems (hooks, secrets, etc.) initialize after the main cluster
# Uses exponential backoff for more robust waiting with diagnostic logging
#
# Usage: wait_for_subsystem [cli_bin] [ticket] [timeout_ms] [subsystem] [max_wait_secs]
# Supported subsystems: hooks, secrets, forge, dns, blob, job, sql
#
# Environment variables:
#   ASPEN_SUBSYSTEM_TIMEOUT - Default max wait time (default: 90)
#   ASPEN_RPC_TIMEOUT - RPC timeout in ms (default: 30000)
#   ASPEN_BACKOFF_INITIAL - Initial backoff delay (default: 1)
#   ASPEN_BACKOFF_MAX - Maximum backoff delay (default: 8)
#   ASPEN_WAIT_VERBOSE - Enable diagnostic logging (default: 0)
#
# Returns: 0 on success, 1 on timeout
wait_for_subsystem() {
    local cli_bin="${1:?CLI binary required}"
    local ticket="${2:?Ticket required}"
    local timeout_ms="${3:-$ASPEN_RPC_TIMEOUT}"
    local subsystem="${4:?Subsystem required}"
    local max_wait="${5:-$ASPEN_SUBSYSTEM_TIMEOUT}"

    local elapsed=0
    local delay="$ASPEN_BACKOFF_INITIAL"
    local attempt=1
    local last_error=""

    # Validate subsystem name
    case "$subsystem" in
        hooks|secrets|forge|dns|blob|job|sql) ;;
        *)
            log_wait_error "Unknown subsystem: $subsystem"
            log_wait_error "Supported subsystems: hooks, secrets, forge, dns, blob, job, sql"
            return 1
            ;;
    esac

    log_wait_diagnostic "Waiting for '$subsystem' subsystem (max ${max_wait}s, timeout ${timeout_ms}ms)"

    while [ "$elapsed" -lt "$max_wait" ]; do
        log_wait_diagnostic "Subsystem '$subsystem' check attempt $attempt (elapsed: ${elapsed}s, next delay: ${delay}s)"

        if _check_subsystem "$cli_bin" "$ticket" "$timeout_ms" "$subsystem"; then
            log_wait_diagnostic "Subsystem '$subsystem' ready after ${elapsed}s ($attempt attempts)"
            return 0
        fi

        last_error="$_SUBSYSTEM_ERROR"
        log_wait_diagnostic "Subsystem '$subsystem' not ready: $last_error"

        sleep "$delay"
        elapsed=$((elapsed + delay))
        attempt=$((attempt + 1))

        # Exponential backoff with cap
        delay=$((delay * ASPEN_BACKOFF_MULTIPLIER))
        if [ "$delay" -gt "$ASPEN_BACKOFF_MAX" ]; then
            delay="$ASPEN_BACKOFF_MAX"
        fi
    done

    log_wait_error "Subsystem '$subsystem' not ready after ${max_wait}s ($attempt attempts)"
    if [ -n "$last_error" ]; then
        log_wait_error "Last error: $last_error"
    fi
    return 1
}

# =============================================================================
# CONFIGURATION DISPLAY
# =============================================================================

# Print current wait configuration (useful for debugging)
# Usage: print_wait_config
print_wait_config() {
    printf "Aspen Wait Configuration:\n"
    printf "  ASPEN_RPC_TIMEOUT=%s (ms)\n" "$ASPEN_RPC_TIMEOUT"
    printf "  ASPEN_CLUSTER_STABLE_TIMEOUT=%s (s)\n" "$ASPEN_CLUSTER_STABLE_TIMEOUT"
    printf "  ASPEN_SUBSYSTEM_TIMEOUT=%s (s)\n" "$ASPEN_SUBSYSTEM_TIMEOUT"
    printf "  ASPEN_CONDITION_TIMEOUT=%s (s)\n" "$ASPEN_CONDITION_TIMEOUT"
    printf "  ASPEN_BACKOFF_INITIAL=%s (s)\n" "$ASPEN_BACKOFF_INITIAL"
    printf "  ASPEN_BACKOFF_MAX=%s (s)\n" "$ASPEN_BACKOFF_MAX"
    printf "  ASPEN_BACKOFF_MULTIPLIER=%s\n" "$ASPEN_BACKOFF_MULTIPLIER"
    printf "  ASPEN_POLL_INTERVAL=%s (s)\n" "$ASPEN_POLL_INTERVAL"
    printf "  ASPEN_WAIT_VERBOSE=%s\n" "$ASPEN_WAIT_VERBOSE"
}
