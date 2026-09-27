# Logical and opaque restore

Logical and opaque snapshots are distinct reconstruction contracts, not two encodings of an interchangeable heap. This article assumes knowledge of typed world roots and follows the [execution snapshot contract](../../world-execution-snapshots.md). It focuses on completeness, cohort compatibility, and activation ordering. See the [Technical companion](../README.md) for adjacent discussions of replay and distribution.

## Two kinds of completeness

A logical profile contains eleven Molten-owned components: artifact, schema, durable state, tasks, history, effects, scheduler, virtual time, entropy, runtime profile, and policy. Its cohort binds the runtime build and ABI together with the schema, handler, task, scheduler, time, entropy, and effect profiles. Capturing only application records would therefore omit execution-relevant facts even when those records look internally consistent.

An opaque profile retains Molten-owned artifact, schema, runtime-profile, and policy roots, while ChaosControl owns the exact machine descriptor and CPU, memory, device, disk, and backend state. Architecture, CPU features, vCPU topology, runtime ABI, and the remaining cohort facts all participate in compatibility. A machine image that is readable is not necessarily reconstructible under the destination cohort.

The [restore service](../../../src/world_snapshot/parts/service/p000/body.rs) uses separate `restore_logical_snapshot` and `restore_opaque_snapshot` entry points. Each rejects the other profile class. The opaque path checks compatibility and a ChaosControl descriptor observation before restoring. There is no error branch that silently invokes logical restoration when the opaque cohort fails.

## Inventory verification precedes state restoration

Both paths obtain an initial current-admission observation, construct a restore plan, and materialize the complete component inventory. The [validation helpers](../../../src/world_snapshot/parts/service/p001/body.rs) sort components by kind and reject observations with a substituted identity, unavailable content, or unverified identity. Duplicate component kinds also fail. State restoration begins only after the inventory call has completed successfully.

This matters because incremental availability is not equivalent to a complete restore basis. If durable state is present but entropy state is unavailable, restoring the former and improvising the latter would change the execution contract. The implementation instead stops before state restoration when a required component cannot be verified.

Logical state restoration follows the core-produced step plan. The shell maps state steps to durable state, history, tasks, scheduler, time, entropy, and effects, while handling artifact materialization, host-handle recreation, admission, and activation separately. The opaque path calls `restore_exact` through its runtime port and checks that the returned observations are nonempty, bounded, and labeled as opaque-machine restoration.

## Restored state is not yet an active runtime

The shell recreates host handles rather than transferring live file descriptors, sockets, timers, credentials, or sessions through the snapshot. The governing contract forbids those handles in descriptors. Fresh handle construction and current admission are separate operations: successful reconstruction does not refresh revoked authority by itself.

Immediately before activation, the shell obtains another admission observation. Validation binds it to the descriptor, profile, and destination cohort, requires permission, and rejects a generation lower than the earlier observation. The check is non-regression, not a claim that the two observations must have identical generations. Only after this succeeds does the shell invoke activation and publish the success receipt. The [logical restore tests](../../../src/world_snapshot/tests/logical.rs) include a successful restore whose final admission generation is newer than its initial generation.

This sequencing creates a useful local safety boundary, but not automatic rollback. A final admission failure can occur after components and host handles were restored. The inspected service returns an error before activation; it does not establish transactional undo of all earlier shell effects. Operational cleanup must not be inferred from the absence of a success receipt.

## Illustrative admission-drift scenario

Consider a logical snapshot with valid scheduler and entropy roots. At the first admission observation, generation 8 permits restoration. During materialization, authority changes. The final observation reports generation 9 with permission denied. These numbers are illustrative.

Inventory completeness and compatibility may still be true. Neither fact overrules the current denial. The shell stops before activation and does not publish the restored success receipt. A reviewer should therefore ask two separate questions: whether reconstruction work occurred, and whether the runtime became active. Treating both as one Boolean “restore succeeded” discards the boundary that protects current authority.

For an opaque variant, suppose CPU-feature facts differ at the destination. The compatibility check fails earlier. An operator cannot reinterpret that failure as permission to perform a logical restore; choosing a different profile would be a different admitted operation, not recovery within the exact opaque request.

## Verification and limits

Suggested verification, not a claim of executed tests: exercise missing inventory, substituted component identity, wrong-profile entry, opaque descriptor drift, stale final admission, and activation failure through the existing ports. Observe which calls occurred, especially activation and receipt publication, rather than checking only the returned error text. Inspect canonical descriptor identities separately from diagnostic rendering.

Snapshot evidence does not prove workload correctness, future portability, logical/opaque equivalence, current release eligibility, or clone isolation beyond recorded observations. Copy-on-write clone planning has its own overlay and realization boundaries; successful restoration does not discharge them. Replay additionally needs the exact transition closure and current execution admission described in the [capsule contract](../../world-replay-capsules.md).

## Sources

- [World execution snapshot profiles](../../world-execution-snapshots.md)
- [World replay capsules](../../world-replay-capsules.md)
- [Restore orchestration](../../../src/world_snapshot/parts/service/p000/body.rs)
- [Inventory and admission validation](../../../src/world_snapshot/parts/service/p001/body.rs)
- [Logical snapshot tests](../../../src/world_snapshot/tests/logical.rs)
- [Technical companion](../README.md)
