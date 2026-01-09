# Decision: Hooks CLI Integration Test Implementation

**Date**: 2026-01-08
**Status**: Implemented

## Context

The Aspen project has an event-driven hooks system (`aspen-hooks` crate) that allows handlers to respond to cluster events like writes, deletes, membership changes, and leader elections. The CLI (`aspen-cli`) exposes three hooks commands:

1. `hooks list` - List configured handlers
2. `hooks metrics` - View execution statistics
3. `hooks trigger` - Manually trigger events for testing

These commands needed comprehensive integration testing to ensure they work correctly against a real cluster.

## Decision

Implement hooks CLI testing in two complementary ways:

### 1. Add hooks tests to existing `cli-test.sh` (scripts/cli-test.sh)

Added a new "Hooks Commands" section with 12 tests covering:

- Basic list/metrics operations
- All 5 valid event types (write_committed, delete_committed, membership_changed, leader_elected, snapshot_created)
- JSON output format verification

### 2. Create standalone `kitty-hooks-test.sh` (scripts/kitty-hooks-test.sh)

A comprehensive 35+ test suite that:

- Can start its own cluster or use an existing one via `--ticket`
- Tests list, metrics, and trigger commands thoroughly
- Tests error cases (invalid event types, malformed JSON)
- Tests payload variations (empty, nested, large)
- Includes workflow tests simulating realistic usage patterns
- Reports timing per test
- Supports JSON output for CI integration

## Implementation

### Files Modified

1. **scripts/cli-test.sh** - Added hooks test section at lines 442-472
2. **scripts/kitty-hooks-test.sh** - New 550-line comprehensive test script
3. **flake.nix** - Added `kitty-hooks-test` app entry at lines 596-628

### Test Coverage

| Test Category | Count | Description |
|--------------|-------|-------------|
| List commands | 3 | Basic, JSON format, structure validation |
| Metrics commands | 5 | Basic, JSON, filtering, structure |
| Trigger (valid types) | 5 | All supported event types |
| Trigger (payloads) | 4 | Empty, nested, minimal, large |
| Trigger (errors) | 3 | Invalid event type, bad JSON |
| Output format | 4 | Human-readable and JSON verification |
| Workflow | 6 | Combined realistic sequences |
| **Total** | **30+** | |

### Usage

```bash
# Run hooks tests with existing cli-test.sh
nix run .#cli-test -- --ticket <ticket> --category hooks

# Run standalone hooks test (starts own cluster)
nix run .#kitty-hooks-test

# Run against existing cluster
nix run .#kitty-hooks-test -- --ticket <ticket>

# Keep cluster running after tests for debugging
nix run .#kitty-hooks-test -- --keep-cluster --verbose

# JSON output for CI
nix run .#kitty-hooks-test -- --json
```

### Test Functions

The test script provides three test primitives:

```bash
run_test "name" command args...           # Command must succeed
run_test_expect "name" "regex" cmd...     # Output must match regex
run_test_expect_fail "name" cmd...        # Command must fail
```

### Cluster Management

The standalone script handles full cluster lifecycle:

1. Starts N nodes with in-memory storage
2. Waits for gossip discovery
3. Extracts ticket from logs
4. Initializes cluster via CLI
5. Adds learners and promotes to voters
6. Runs tests
7. Cleans up (unless `--keep-cluster`)

## Rationale

- **Two-tier approach**: The cli-test.sh integration ensures hooks are tested alongside other CLI commands in the standard test suite. The standalone script enables deeper, isolated testing of hooks functionality.

- **Self-contained cluster**: The standalone test can start its own cluster, making it easy to run in CI or locally without manual cluster setup.

- **Error case coverage**: Testing invalid inputs ensures the hooks system fails gracefully with proper error messages.

- **Timing metrics**: Per-test timing helps identify performance regressions.

- **JSON output**: Enables programmatic consumption of test results in CI pipelines.

## Alternatives Considered

1. **Rust integration tests only**: The existing `tests/hooks_integration_test.rs` uses `RealClusterTester` but requires network access (marked `#[ignore]`). Shell scripts provide end-to-end testing through the actual CLI binary.

2. **Kitty terminal-based test**: Could have required kitty terminal for visual testing, but non-interactive testing is more suitable for CI.

## Related Files

- `crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs` - CLI implementation
- `crates/aspen-hooks/` - Hooks system crate
- `tests/hooks_integration_test.rs` - Rust integration tests
- `scripts/lib/cluster-common.sh` - Shared cluster utilities
