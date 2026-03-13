## Why

Aspen's distributed testing has two layers today: madsim for deterministic simulation (fakes the network) and NixOS VM tests for full-system integration (coarse network control via iptables). Neither tests real iroh QUIC connections under realistic network topologies — NAT traversal, hole-punching through different NAT types, link degradation, cross-region latency. These are exactly the conditions that break production deployments. n0-computer built [patchbay](https://github.com/n0-computer/patchbay) specifically to test iroh under these conditions using Linux network namespaces, and it runs unprivileged with no VM overhead.

## What Changes

- Add `patchbay` as a dev dependency in a new `aspen-testing-patchbay` crate
- Build a test harness that spawns Aspen nodes inside patchbay network namespaces with real iroh endpoints
- Write integration tests covering Raft cluster formation across NAT boundaries, KV replication under link degradation, and partition/heal recovery
- Extend existing `aspen-testing-network` fault injection with patchbay-backed implementations that work on real iroh traffic instead of iptables stubs
- Add CI profile support for patchbay tests (require unprivileged user namespaces)

## Capabilities

### New Capabilities

- `patchbay-harness`: Test harness for spawning Aspen nodes inside patchbay network namespaces with configurable topologies (NAT types, link conditions, regions)
- `patchbay-nat-tests`: Integration tests verifying Raft cluster formation and KV operations across Home, Corporate, CGNAT, and Public NAT types
- `patchbay-fault-tests`: Fault injection tests using patchbay's dynamic link control — partitions, latency spikes, packet loss, link down/up — against real iroh connections

### Modified Capabilities

## Impact

- **New crate**: `crates/aspen-testing-patchbay/` with `patchbay` dependency
- **Cargo workspace**: Add new crate to workspace members
- **CI**: Patchbay tests need `kernel.unprivileged_userns_clone=1` (default on most distros, may need sysctl in CI)
- **Existing tests**: No changes to madsim or NixOS VM tests — patchbay is a complementary layer
- **Dependencies**: `patchbay` crate from n0-computer (Apache-2.0/MIT, same as iroh)
