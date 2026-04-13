#!/usr/bin/env bash
set -euo pipefail
set -x

run_exact_test() {
    local crate="$1"
    local test_name="$2"
    shift 2

    local output
    output=$(cargo test -p "$crate" "$@" "$test_name" -- --exact --nocapture 2>&1)
    printf '%s\n' "$output"

    grep -F "running 1 test" <<<"$output" >/dev/null || {
        echo "expected exactly 1 test run for $crate::$test_name" >&2
        exit 1
    }
    grep -F "test $test_name ... ok" <<<"$output" >/dev/null || {
        echo "expected passing test line for $crate::$test_name" >&2
        exit 1
    }
}

cargo test -p aspen-client-api --test wire_format_golden
run_exact_test aspen-rpc-handlers registry::tests::handler_registry_new_with_minimal_context_registers_core_handlers
run_exact_test aspen-rpc-handlers registry::tests::handler_registry_declared_blob_plan_requires_context_capability --features blob
cargo check -p aspen --all-targets --no-default-features
cargo check -p aspen --all-targets --no-default-features --features node-runtime
cargo check -p aspen --all-targets --no-default-features --features node-runtime-apps
run_exact_test aspen node::tests::test_nodebuilder_start_creates_node --features node-runtime
cargo check -p aspen-rpc-core --all-targets --features forge,hooks,git-bridge
cargo check -p aspen-rpc-handlers --all-targets --features blob,docs,forge,jobs,ci,secrets,net,snix
