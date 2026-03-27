## Context

Federation is functionally complete: handshake, trust, sync, ref fetch, blob transfer, git clone, push, bidi sync, and auth tokens all work across two independent clusters in NixOS VM tests. The existing dogfood pipeline (`ci-dogfood-full-loop`) proves single-cluster Forge→CI→Nix build. What's missing is the combination: a federated CI build where one cluster triggers a build from a repo that originated on a different cluster.

The `aspen-federation` crate has 216 passing tests (177 non-ignored, 39 requiring network). The verified modules have Verus specs for `quorum` and `fork_detection` but not `ref_diff`. The policy modules (1,003 lines across 5 files) have 20 total tests — thin for business logic that decides trust, selection, and verification decisions.

## Goals / Non-Goals

**Goals:**

- Prove the full cross-cluster pipeline: Cluster A creates repo + pushes code → Cluster B federates + runs CI build → binary runs
- Add Verus formal specs for `compute_ref_diff` and `resolve_conflicts`
- Fill test gaps in policy modules (resource_policy, verification, selection)
- Ensure federation `#[ignore]` tests are discoverable via `cargo nextest run -P network --run-ignored all`

**Non-Goals:**

- Multi-node Raft within each cluster (both clusters are single-node for this test)
- NAR/binary cache verification across clusters (out of scope, separate snix work)
- Modifying federation protocol or adding new wire messages
- Performance benchmarks

## Decisions

### D1: VM test structure — two VMs, single-node clusters

Same pattern as `federation-git-clone.nix`: two independent VMs (alice, bob), each running a single-node Aspen cluster with Forge + CI + federation enabled. Alice creates a repo, pushes a Nix flake, federates it. Bob trusts Alice, syncs the federated repo, and the CI pipeline triggers on the mirrored content.

Alternative considered: three VMs (one per cluster + one observer). Rejected — adds complexity without testing new code paths. Two-VM pattern is proven across four existing federation tests.

### D2: CI trigger mechanism — manual `ci trigger` on mirrored repo

The existing dogfood tests rely on Forge's auto-trigger-on-push. For federated repos, the push happens on Alice's cluster, not Bob's. Bob's CI needs to be told to build from the mirrored content. Two options:

1. **Auto-trigger on federation sync** — would require new code in the CI trigger path to detect mirror updates. More work, more moving parts.
2. **Manual `ci trigger` on the mirrored repo** — Bob runs `aspen-cli ci trigger` against the mirrored repo after sync completes. Uses existing CLI plumbing.

Going with option 2. It tests the same CI pipeline code and avoids introducing a new trigger mechanism in this change.

### D3: Verus ref_diff spec — spec functions over sequences, not HashMaps

Verus doesn't support `HashMap` natively. The spec will model ref heads as `Seq<(Seq<char>, Seq<u8>)>` pairs and define set-theoretic properties: every ref in the output belongs to exactly one category (pull, push, in_sync, conflict), the categories partition the union of local and remote ref names, and conflict resolution preserves the partition invariant.

### D4: Policy test strategy — property-style assertions, not just happy-path

The policy modules make decisions about which seeders to trust, how to select sync targets, and how to verify content. Tests should cover boundary conditions: empty inputs, single seeder, all seeders untrusted, hash mismatches, expired credentials, and conflicting policies. Aim for ~10-15 tests per module.

## Risks / Trade-offs

- **[Two-cluster CI complexity]** → The test script will be ~300-400 lines of Python. Mitigated by reusing helper functions from existing federation VM tests.
- **[Verus HashMap limitation]** → Spec models sequences, not the actual HashMap implementation. Mitigated by proving set-theoretic properties that hold regardless of container type.
- **[Test flakiness from iroh socket timing]** → Federation wire tests need iroh to bind. Mitigated by existing retry config in the network profile (retries = 2).
