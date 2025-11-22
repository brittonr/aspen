# VM Cluster Testing with QEMU

This directory contains tests for running mvm-ci in true isolated VMs using NixOS and QEMU.

## Why VM Testing?

**Current Limitations (localhost testing):**
- All processes share same kernel/network stack
- Can't test real network partitions
- Can't test true node isolation
- Shared filesystem access

**VM Testing Benefits:**
- ✅ True network isolation between nodes
- ✅ Test network partitions and failures
- ✅ Each node has independent flawless server
- ✅ Realistic distributed system testing
- ✅ Test node crashes and recovery
- ✅ Test Raft leader election during failures
- ✅ Test split-brain scenarios
- ✅ Reproducible test environment

## Architecture

```
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│   QEMU VM 1     │  │   QEMU VM 2     │  │   QEMU VM 3     │
│                 │  │                 │  │                 │
│  ┌───────────┐  │  │  ┌───────────┐  │  │  ┌───────────┐  │
│  │ Flawless  │  │  │  │ Flawless  │  │  │  │ Flawless  │  │
│  │ :27288    │  │  │  │ :27288    │  │  │  │ :27288    │  │
│  └─────┬─────┘  │  │  └─────┬─────┘  │  │  └─────┬─────┘  │
│        │        │  │        │        │  │        │        │
│  ┌─────▼─────┐  │  │  ┌─────▼─────┐  │  │  ┌─────▼─────┐  │
│  │  mvm-ci   │  │  │  │  mvm-ci   │  │  │  │  mvm-ci   │  │
│  │  :3020    │  │  │  │  :3020    │  │  │  │  :3020    │  │
│  └─────┬─────┘  │  │  └─────┬─────┘  │  │  └─────┬─────┘  │
│        │        │  │        │        │  │        │        │
│  ┌─────▼─────┐  │  │  ┌─────▼─────┐  │  │  ┌─────▼─────┐  │
│  │ Hiqlite   │◄─┼──┼─►│ Hiqlite   │◄─┼──┼─►│ Hiqlite   │  │
│  │ node_id=1 │  │  │  │ node_id=2 │  │  │  │ node_id=3 │  │
│  └───────────┘  │  │  └───────────┘  │  │  └───────────┘  │
│                 │  │                 │  │                 │
└─────────────────┘  └─────────────────┘  └─────────────────┘
        │                    │                    │
        └────────────────────┴────────────────────┘
                    Virtual Network Bridge
                    (192.168.1.0/24)
```

## Test Scenarios

The VM tests cover:

1. **Cluster Formation**
   - 3 nodes start independently
   - Raft cluster forms automatically
   - Leader election completes

2. **Job Replication**
   - Submit job to Node 1
   - Verify it appears on Node 2 and Node 3
   - Check Raft log consistency

3. **Node Failure Recovery**
   - Kill Node 3
   - Submit job while it's down
   - Restart Node 3
   - Verify it replays Raft log and catches up

4. **Network Partition (Split-Brain)**
   - Isolate Node 1 with iptables
   - Nodes 2+3 maintain quorum (2/3)
   - Submit job to majority partition
   - Heal partition
   - Verify Node 1 rejoins and syncs

5. **Cross-Node Communication**
   - Each node can reach others via network
   - HTTP endpoints accessible across VMs
   - P2P iroh communication works

6. **Independent Flawless Servers**
   - Each node runs its own flawless instance
   - Workflow execution is isolated per node
   - No shared filesystem dependencies

## Running the Tests

### Prerequisites

```bash
# Ensure you have NixOS test support
nix-build '<nixpkgs/nixos>' -A tests
```

### Run Interactive Test (Recommended for Development)

```bash
# Build the test driver
nix-build tests/vm-cluster-test.nix -A driver

# Run interactive test VM
./result/bin/nixos-test-driver

# Inside the test driver:
>>> start_all()
>>> node1.shell_interact()  # Drop into node1 shell
>>> node2.succeed("curl http://node1:3020/queue/list")
```

### Run Automated Test

```bash
# Run all tests
nix-build tests/vm-cluster-test.nix

# View results
cat result/test-output.xml
```

### Run with Visible UI (Debugging)

```bash
# Set environment variable to show QEMU graphical console
QEMU_OPTS="-vga std" nix-build tests/vm-cluster-test.nix -A driver
./result/bin/nixos-test-driver
```

## Test Output

Successful test output:
```
=== Waiting for services to start ===
node1: waiting for unit flawless.service
node1: unit flawless.service reached active state
node2: waiting for unit flawless.service
...
=== Verifying job replication across cluster ===
Node 1 jobs: [{"job_id":"job-0","status":"InProgress",...}]
Node 2 jobs: [{"job_id":"job-0","status":"InProgress",...}]
Node 3 jobs: [{"job_id":"job-0","status":"InProgress",...}]
=== Testing node failure resilience ===
...
=== All tests passed! ===
test script finished in 45.23s
```

## Performance Considerations

VM tests are slower than localhost tests:
- **Localhost test**: ~30 seconds
- **VM test**: ~2-3 minutes

Breakdown:
- VM boot: 20-30s per node
- Service startup: 15-20s
- Test execution: 60-90s

Use VM tests for:
- Integration testing before releases
- Testing failure scenarios
- CI/CD quality gates

Use localhost tests for:
- Fast feedback during development
- Unit testing
- Basic smoke tests

## CI Integration

Add to your CI pipeline:

```yaml
# .github/workflows/vm-tests.yml
name: VM Cluster Tests
on: [push, pull_request]

jobs:
  vm-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: cachix/install-nix-action@v22
      - name: Run VM cluster tests
        run: nix-build tests/vm-cluster-test.nix
      - name: Upload test results
        uses: actions/upload-artifact@v3
        with:
          name: test-results
          path: result/
```

## Debugging Failed Tests

### Get test logs

```bash
# Run test and keep VM state
nix-build tests/vm-cluster-test.nix --keep-failed

# Logs are in /tmp/nix-build-*/
ls /tmp/nix-build-vm-cluster-test-*/
```

### Connect to running VM

```bash
# In test driver interactive mode:
>>> node1.shell_interact()

# Inside VM:
$ journalctl -u mvm-ci.service -f
$ systemctl status mvm-ci.service
$ curl http://localhost:3020/queue/list
```

### Common Issues

**Hiqlite cluster won't form:**
- Check firewall: `iptables -L -n`
- Verify DNS: `ping node2`
- Check logs: `journalctl -u mvm-ci.service`

**Flawless not starting:**
- Check: `systemctl status flawless.service`
- Logs: `journalctl -u flawless.service -f`
- Port: `ss -tlnp | grep 27288`

**Network partition not working:**
- Verify iptables rules: `iptables -L -v -n`
- Check connectivity: `ping node2`

## Advanced Scenarios

### Testing 5-Node Cluster

Modify `vm-cluster-test.nix`:
```nix
clusterNodes = [
  { id = 1; host = "node1"; ... }
  { id = 2; host = "node2"; ... }
  { id = 3; host = "node3"; ... }
  { id = 4; host = "node4"; ... }
  { id = 5; host = "node5"; ... }
];
```

### Simulating Slow Networks

```python
# In testScript
node1.succeed("tc qdisc add dev eth0 root netem delay 100ms")
```

### Testing Leader Election

```python
# Kill current leader and verify new election
node1.succeed("systemctl stop mvm-ci.service")
time.sleep(5)
# Verify cluster still accepts writes
node2.succeed('curl -X POST http://localhost:3020/new-job ...')
```

## Future Enhancements

- [ ] Chaos engineering tests (random node kills)
- [ ] Performance benchmarks across VMs
- [ ] Multi-region simulation (WAN latency)
- [ ] Docker/Kubernetes deployment tests
- [ ] Upgrade testing (rolling updates)
- [ ] Backup/restore testing
