# Testing Guide for MVM-CI

## Overview

Comprehensive testing infrastructure for mvm-ci with integration tests, fixtures, and helpers.

## Test Architecture

### Layered Testing Strategy

```
Scenario-Based Integration Tests (tests/scenario_tests.rs)
    ↓
Test Fixtures (ControlPlaneFixture, TestWorkerFixture)
    ↓
Test Helpers (wait functions, job utils)
    ↓
Domain Layer (Mock repositories, Services)
```

## Quick Start

### Running Tests

```bash
# Unit tests (fast, no infrastructure)
cargo test --lib

# Integration tests (requires infrastructure)
cargo test --test scenario_tests

# All tests
cargo test
```

## Test Fixtures

### ControlPlaneFixture

Manages a real control plane server for integration testing.

**Features:**
- Deploys test WASM module to flawless server
- Creates iroh P2P endpoint
- Starts dual-listener server (HTTP + P2P)
- Provides real endpoint tickets
- Graceful shutdown

**Usage:**
```rust
let control_plane = ControlPlaneFixture::new(3030).await?;
let client = control_plane.client().await;
// ... tests ...
control_plane.shutdown().await?;
```

### TestWorkerFixture

Simulates worker with full lifecycle.

**Features:**
- Worker registration
- Periodic heartbeats
- Job claiming
- Job completion/failure reporting
- Automatic cleanup

**Usage:**
```rust
let mut worker = TestWorkerFixture::new(ticket, WorkerType::Wasm).await?;
worker.register().await?;
worker.start_heartbeat();
let job = worker.claim_work().await?;
worker.shutdown().await;
```

## Test Coverage

### ✅ Implemented (10 Scenario Tests)

1. **Worker Registration** (2 tests)
   - Single worker registration
   - Multiple workers of different types

2. **Heartbeat Monitoring** (2 tests)
   - Heartbeat lifecycle
   - Active job count tracking

3. **Job Claiming** (2 tests)
   - Worker type filtering
   - Compatibility matching

4. **Orphaned Job Recovery** (2 tests)
   - Worker failure recovery
   - Heartbeat keepalive

5. **End-to-End Execution** (2 tests)
   - Complete workflow
   - Error handling

### 📝 TODO

- Job submission helpers
- Worker stats verification
- Property-based tests (scheduler invariants)
- Firecracker backend tests

## Files Created

```
tests/
├── common/
│   ├── mod.rs              # Module exports
│   ├── fixtures.rs         # ControlPlaneFixture, TestWorkerFixture (305 lines)
│   └── helpers.rs          # Test utilities (170 lines)
├── integration_tests.rs    # Domain layer tests
└── scenario_tests.rs       # Integration scenarios (380 lines)
```

## Troubleshooting

**"Failed to deploy test module"**
- Ensure flawless server is running at http://localhost:27288

**"Failed to parse endpoint ticket"**
- Server startup failed, check logs
- Increase startup wait time

**Tests timeout**
- Run with: `cargo test -- --test-threads=1 --nocapture`

**Port conflicts**
- Each test uses unique port (3030+)
- Kill hanging processes: `pkill -f mvm-ci`

## Best Practices

1. **Use unique ports** for parallel tests
2. **Always clean up** fixtures
3. **Use test helpers** for common operations
4. **Assert specific state**, not just existence

## Performance

- Unit tests: <100ms
- Integration fixture setup: ~2-3s
- Full scenario test: ~3-5s
- All scenarios: ~30-60s


## Nix-Based Testing (Recommended)

The project includes a Nix-based test infrastructure that automatically manages all dependencies, including the flawless server.

### Quick Start with Nix

```bash
# Run integration tests (starts flawless server automatically)
nix run .#integration-tests

# Run as a check (for CI/CD)
nix flake check -L

# Run specific check
nix build .#checks.x86_64-linux.integration-tests -L
```

### What the Nix Test Runner Does

1. **Automatically starts flawless server** in the background
2. **Waits for server to be ready** (health check polling)
3. **Runs integration tests** with proper environment setup
4. **Cleans up** server and temporary files on exit
5. **Reports results** with colored output

### Benefits of Nix-Based Testing

✅ **No manual setup** - All dependencies managed by Nix
✅ **Reproducible** - Same environment every time
✅ **Isolated** - No conflicts with system packages
✅ **CI-ready** - Can be used in CI/CD pipelines
✅ **Declarative** - Test infrastructure as code

### Script Location

The test runner script is at: `scripts/run-integration-tests.sh`

### Environment Variables

The Nix runner automatically sets:
- `FLAWLESS_URL=http://localhost:27288`
- `RUST_LOG=info` (can be overridden)
- `SNIX_BUILD_SANDBOX_SHELL` (for snix crates)

### Troubleshooting Nix Tests

**"flawless server failed to start"**
```bash
# Check flawless logs
cat /tmp/flawless-test-*.log

# Ensure port 27288 is available
lsof -i :27288
```

**"Nix sandbox issues"**
- The integration test check uses `__noChroot = true` to allow network access
- This is required for flawless server to bind to ports

**"Rebuild flake cache"**
```bash
# Clear flake evaluation cache
nix flake update
nix build .#checks.x86_64-linux.integration-tests -L --rebuild
```

