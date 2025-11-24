# Test Scripts

## run-integration-tests.sh

Comprehensive integration test runner that manages all required services.

### Features

- ✅ **Automatic flawless server management** - Starts and stops server
- ✅ **Health check polling** - Waits for server to be ready
- ✅ **Error handling** - Graceful cleanup on failure
- ✅ **Colored output** - Clear status messages
- ✅ **Process management** - Tracks and kills background services

### Usage

#### Via Nix (Recommended)

```bash
# Run with all dependencies automatically managed
nix run .#integration-tests

# Run as a check (for CI/CD)
nix flake check -L
nix build .#checks.x86_64-linux.integration-tests -L
```

#### Direct Execution

```bash
# Ensure flawless and cargo are in PATH
./scripts/run-integration-tests.sh
```

#### From devShell

```bash
nix develop
./scripts/run-integration-tests.sh
```

### What It Does

1. **Pre-flight checks**
   - Verifies `flawless` is available
   - Verifies `cargo` is available

2. **Server startup**
   - Starts `flawless up` in background
   - Captures server logs to `/tmp/flawless-test-$$.log`
   - Polls health endpoint for up to 10 seconds
   - Fails fast if server doesn't start

3. **Build & Test**
   - Sets `FLAWLESS_URL=http://localhost:27288`
   - Runs `cargo build --tests`
   - Executes `cargo test --test scenario_tests`

4. **Cleanup**
   - Kills flawless server process
   - Removes temporary log files
   - Reports final test status

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `FLAWLESS_URL` | `http://localhost:27288` | Flawless server URL |
| `RUST_LOG` | `info` | Rust logging level |

### Exit Codes

- `0` - All tests passed
- `1` - Tests failed or error occurred

### Troubleshooting

**Server fails to start:**
```bash
# Check if port is in use
lsof -i :27288

# View server logs
cat /tmp/flawless-test-*.log
```

**Tests timeout:**
- Increase health check timeout in script (currently 10 seconds)
- Check network connectivity
- Verify firewall settings

**Permission denied:**
```bash
chmod +x scripts/run-integration-tests.sh
```

## Adding New Scripts

When adding new test scripts:

1. Make them executable: `chmod +x scripts/new-script.sh`
2. Add error handling with `set -euo pipefail`
3. Implement cleanup with `trap cleanup EXIT INT TERM`
4. Document in this README
5. Consider adding to flake.nix as an app or check
