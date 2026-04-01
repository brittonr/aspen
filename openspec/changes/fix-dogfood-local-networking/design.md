## Approach

### Local Connectivity Fix

The root cause: when relay is disabled and mDNS is off, iroh endpoints can only connect if the target's direct socket address is known. The ticket must carry the node's listen address.

**Node side**: After `Endpoint::bind()`, the node knows its local addresses. The ticket writer already includes `endpoint_addr.addrs` — but with relay disabled, the endpoint may not discover its own addresses quickly enough before the ticket is written.

**Fix**: The dogfood binary reads the ticket, parses the bootstrap peer address, and passes it explicitly when creating the `AspenClient`. If the ticket only has a public key (no addrs), fall back to `127.0.0.1` with a known port range.

**Alternative (simpler)**: Use iroh's `ClusterDiscovery` — nodes in the same `--data-dir` parent directory auto-discover each other via filesystem-based discovery. The dogfood binary already places alice and bob under `/tmp/aspen-dogfood/`. Both nodes and the client need to point at the same cluster discovery dir.

We go with the ClusterDiscovery approach — it's already implemented in aspen-cluster and just needs wiring.

### Large-Repo VM Test

Expand `federation-git-clone.nix` with a subtest that creates 100+ files in a nested directory structure (3 levels deep), pushes to alice, federates, and verifies bob's clone has all files. This exercises:

- Multi-batch federation sync (100+ objects > batch_size=2000 threshold when including trees)
- Convergent import across batch boundaries
- DAG integrity verification post-import

## Key Decisions

1. **ClusterDiscovery for local dogfood** — reuses existing infrastructure, no custom address passing
2. **In-test repo generation** — create files programmatically in the VM test script, not from a fixture
3. **Verify via file count + content spot-checks** — not SHA-1 comparison (too complex for VM test)

## Risks

- ClusterDiscovery relies on filesystem atomicity for the discovery file — safe on ext4/tmpfs
- 100-file repo is still small compared to the real 34K-object Aspen repo — but sufficient to exercise batch boundaries
