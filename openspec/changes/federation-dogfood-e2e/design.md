## Context

`dogfood-local.sh` runs a single-cluster self-build loop: start N nodes, push Aspen to Forge, CI builds it, verify/deploy the artifact. This proves the core pipeline.

Federation (sync protocol, auth tokens, bidi sync) was built and tested in NixOS VM integration tests (`federation-ci-dogfood-test`), which spin up two QEMU VMs with separate clusters connected over a veth bridge. That test proves the protocol works in isolation but doesn't exercise the path on real host processes with real iroh relay discovery, real filesystem I/O, and real nix builds.

The gap: nobody has run two `aspen-node` processes on the same host (different ports, different data dirs, different cookies) and exercised the full federated dogfood pipeline end-to-end.

## Goals / Non-Goals

**Goals:**

- Shell script (`scripts/dogfood-federation.sh`) that starts two independent clusters on localhost, pushes source to cluster A's Forge, federates the repo, syncs it on cluster B, triggers CI build on cluster B, and verifies the binary.
- Bootable serial VM image for interactive testing via `vm_boot` + `vm_serial` (avoids NixOS test framework rebuild cycles).
- Expose and fix any bugs in the real-process federation + CI pipeline.

**Non-Goals:**

- Multi-host federation (both clusters run on localhost with different ports).
- DHT/global-discovery (clusters connect directly via iroh NodeId + address hints).
- Replacing the existing NixOS VM tests (those stay as regression tests).
- Federation of CI pipelines themselves (cluster B runs its own independent CI, just building mirrored content).

## Decisions

**Two clusters on one host**: Each cluster gets a unique cookie, secret key, data dir, and port range. They're fully independent — separate Raft groups, separate KV stores, separate Forge instances. The script manages both lifecycles. This matches the `federation-ci-dogfood-test` NixOS test topology but without the VM overhead.

**Sync via direct address, not DHT**: The script extracts cluster A's iroh NodeId and address from `cluster health`, then passes `ASPEN_ORIGIN_ADDR` to cluster B's CLI commands. DHT discovery is unreliable on localhost and irrelevant to the dogfood goal.

**CI on cluster B only**: Cluster A is just a Forge host. Cluster B has `--enable-ci` and `--enable-workers` to run the nix build. This keeps the test focused: federation provides the content, local CI does the build.

**Flake.nix integration**: New `dogfood-federation` app in `flake.nix` wrapping the script (like `dogfood-local`). New `dogfood-serial-federation-vm` package producing a qcow2 with two systemd services (aspen-node-alice, aspen-node-bob) and helper scripts.

**Script structure mirrors dogfood-local.sh**: Same subcommand pattern (`start`, `push`, `federate`, `sync`, `build`, `verify`, `full`). Reuses helpers where possible. Separate `CLUSTER_DIR_ALICE` and `CLUSTER_DIR_BOB`.

## Risks / Trade-offs

**Port conflicts on localhost**: Two clusters binding iroh QUIC listeners on the same host. Mitigation: Let iroh pick random ports (default behavior), extract actual ports from health output.

**Nix build resource contention**: Two clusters + a nix build on one machine is memory-heavy. Mitigation: Only cluster B runs CI; keep cluster A minimal (no workers, no CI).

**Federation sync timing**: The sync protocol may need retries since both clusters just started and iroh connections take time to establish. Mitigation: Retry loop with backoff in the script, matching the NixOS test pattern.

**Flake archive hang**: `nix flake archive` with no internet hangs forever (known issue in napkin). Mitigation: Script uses same 120s timeout + `kill_on_drop` pattern from CI executor.
