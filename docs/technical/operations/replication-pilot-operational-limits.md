# Replication pilot operational limits

The verified node-replication pilot evaluates a possible local multicore and NUMA data-structure building block. It is not a deployment of network replication. This article assumes familiarity with pinned source inputs, verifier toolchains, and the [pilot decision document](../../verified-node-replication-pilot.md). It explains what an operator or reviewer can conclude from the probe and why a reproducible check can pass while dependency admission remains denied. See the [Technical companion](../README.md) for adjacent material.

## The subject of verification is deliberately narrow

The [typed pilot profile](../../../verification/verified-node-replication-pilot/profile.ncl) sets scope to local multicore/NUMA data structures and runtime dependency status to denied. It pins upstream revision `eb8e27244a4f1b7714defe55aa495c6173a1c330`, the historical Verus revision, source hashes, historical Rust context, current Octet verifier identity, solver identity, Rust version, probe arguments, bounds, trusted-boundary counts, and promotion criteria.

These identities distinguish three questions. First, which upstream source is being examined? Second, which exact verifier and solver are examining it now? Third, would that result justify introducing an adapter into Molten? A hash can answer an identity question without answering a correctness or integration question. Likewise, reproducing a historical toolchain context is not equivalent to current-toolchain compatibility.

The local data-structure meaning of “node replication” must remain separate from fabric membership, transport, distributed consistency, and consensus. No successful outcome of this pilot would by itself establish agreement between machines or persistence across a network partition.

## The reproducible check expects a blocked result

The [Nix probe](../../../nix/verified-node-replication-pilot.nix) exports Nickel and compares the generated JSON, checks a positive fixture and negative fixtures, validates the supplied Octet profile, and checks source sentinels and historical source context. It also searches workspace Cargo manifests and lockfile inputs to reject unintended runtime dependency admission. The toolchain is obtained from Octet package attributes; there is no local reconstruction fallback in this expression.

The probe then runs the current verifier twice under the profile's 120-second timeout. Without `-V new-mut-ref`, it expects seven unsupported-feature diagnostics. With that flag, it expects an internal error containing `Verus Internal Error: var_local_id failed` and the upstream `exec/rwlock.rs` location. Expected failure exit codes and non-timeout status are checked explicitly.

Consequently, success of this Nix check means that the bounded, pinned experiment reproduced its specified blocked outcome and evidence shape. It does not mean that the upstream verification succeeded. If a future verifier unexpectedly succeeds, the existing check is not thereby a promotion gate that automatically admits the dependency: its expected-result assertions and reviewed profile would need reconsideration.

Saved diagnostics have a declared bound of 65536 bytes. The expression normalizes the source path and process identifier in captured stderr and writes concise diagnostic summaries. Normalized diagnostic text is useful for comparison, but it is not a complete execution trace or a proof certificate.

## Trusted boundaries survive tool success

The profile records 33 trusted markers, nine external-body markers, and one assume site. Required locations include the top-level refinement theorems and the public `Dispatch` and `NodeReplicatedT` traits. The probe inventories marker sites and checks the expected counts and named locations.

Counting these sites is an inventory operation, not acceptance of their assumptions. A verifier may establish obligations only relative to its explicit trusted boundaries. Even if the current internal error disappeared, accepting a local API would still require examining what those boundaries assume about operations, state, scheduling, and callers. The profile does not convert upstream theorem names into a proof of Molten integration.

The promotion criteria therefore include more than compatibility: accepted trusted boundaries, a bounded local NUMA adapter design, positive and negative concurrency tests, bounded NUMA benchmarks, removal and rollback testing, and scoped Octet, provenance, Valence, and Cairn evidence. These are existing pilot criteria, not new requirements introduced by this article.

## Worked review error: “green check” becomes “safe dependency”

Consider an illustrative review in which a reproducible Nix build succeeds and a dashboard displays green. A proposed change then claims that the dependency is verified and ready for runtime use. The claim reverses the experiment's meaning. Its expected decision is `blocked-verifier-internal-error`; the generated decision also sets runtime dependency status to denied and promotion eligibility to false.

The decision representation matters. The probe creates a JSON payload, renders sorted compact JSON, hashes those bytes with BLAKE3, and stores a wrapper identifying that hash scope. It recomputes the payload digest for comparison. This is a content-bound pilot decision, not one of the canonical Preserves operator receipts described in the [production runbooks](../../production-operator-runbooks.md). Treating all files called “evidence” as interchangeable would erase their different serialization and claim boundaries.

The corrected review conclusion is narrow: the experiment reproduced the blocked compatibility result for specified inputs, subject to its trusted and diagnostic boundaries. Runtime dependency admission remains denied.

## Verification guidance and evidence availability

The governing pilot document supplies the Nix command for reproducing the check. Suggested review examines exact profile/export agreement, Octet package identities, expected diagnostics, trusted-marker inventories, decision payload hash scope, denied dependency status, and every promotion criterion independently. The operator should retain failure and timeout distinctions rather than treating all unsuccessful verifier runs as the same result.

No pilot build or verifier command was executed for this article. There is also a checkout limitation: the governing document names a refreshed archive under `cairn/archive/2026-07-11-consume-octet-verus-toolchain/evidence/`, but that directory was unavailable during source inspection. The reported historical outcome here is attributed to the governing document and the inspected reproducibility assertions, not to an independently inspected archive. No missing archive artifact is linked or reconstructed.

Neither a future verifier pass nor this bounded probe establishes production latency, throughput, recovery safety, distributed correctness, or release readiness. Those conclusions require their own explicitly scoped evidence.

## Sources

- [Verified node-replication pilot](../../verified-node-replication-pilot.md)
- [Production operator runbooks](../../production-operator-runbooks.md)
- [Typed pilot profile and promotion criteria](../../../verification/verified-node-replication-pilot/profile.ncl)
- [Reproducible probe and decision serialization](../../../nix/verified-node-replication-pilot.nix)
- [Technical companion](../README.md)
