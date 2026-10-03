# Distribution, retention, and reachability

Moving immutable objects, learning remote head claims, and deciding what may be deleted are different operations. This article assumes familiarity with typed world roots and explains why their evidence cannot be collapsed into a single “synchronized” state. The [distribution contract](../../world-distribution.md) governs these boundaries; the [Technical companion](../README.md) links related world mechanisms.

## Closure transfer does not select history

A world distribution graph contains canonical commits and typed root objects. Commit edges reference required roots and parent commits. The adapter checks canonical commit identity before constructing the graph, while generic DAG synchronization supplies bounded traversal, resume fencing, and verify-before-progress behavior. World object and byte bounds supplement, rather than replace, the generic depth, edge, peer, and progress bounds.

The requested root, epoch, generation, policy, peers, and prior progress bind a resumed synchronization. A cursor alone is not sufficient authority to continue after context changes. Likewise, transferring every requested object establishes verified possession of that requested closure, not permission to activate it or choose its head.

Detached head claims travel through a different exchange path. Authentication, local authority, durable currentness, and local branch policy inform the head protocol. Competing admitted successors remain conflicts rather than allowing arrival order to choose a winner. This separation prevents a fast content source from acquiring head-selection power merely because it finished transferring first. The claim boundary is specified in the [governing distribution document](../../world-distribution.md).

## The replication bridge preserves the planned binding

`WorldReplicationBridge` adapts generic content replication to DAG fetch and verification ports. The [implementation](../../../src/world_distribution/bridge.rs) indexes transferable actions by content reference and rejects requests outside that index. Peer assignment must match an action. Returned envelopes must preserve operation identity, content reference, target peer, encoded byte length, and protected form.

Verification is another step, not an implication of receiving bytes. The bridge requires a matching transfer envelope and checks that the returned verification names the planned operation, content, and peer. It exposes identity and authorization observations to the DAG layer. Uncertain, unavailable, and timed-out transfers become deferred observations rather than verified completion.

The governing shell sequence records durable DAG progress only after content verification, then publishes generic and world receipts. World replication disables transfer cleanup and handoff cleanup: transferring a replica is deliberately not a garbage-collection decision. Even a correctly verified second copy says nothing by itself about legal holds or ongoing executions that still reference the first.

## Completeness is a property of observations

Retention projects explicit observations for a closed collection of root classes: heads and competing heads, execution and task state, replay and simulation pins, conflicts, promotion and reconciliation state, holds, remote leases, and incomplete transfers. An observed empty class means “this class was inspected and no roots were found.” An omitted class means “this part of the root inventory is unknown.” Replacing the latter with the former would turn missing knowledge into apparent permission.

The [pure retention projection](../../../crates/molten-core/src/world_distribution/retention.rs) makes that distinction visible. Missing or unobserved classes populate `missing_classes`; missing-class issues can yield an incomplete report rather than a fatal validation error. Other malformed observations can reject projection entirely. `reference_index_complete` also depends on unresolved remote observations and complete edge and attribution inventories. All classes being present is therefore necessary but not sufficient.

Active, uncertain, contradictory, and unavailable remote leases retain their named roots under the governing contract. Uncertain observations also block completeness. A cleared lease no longer contributes its roots, but clearing one lease cannot erase a local execution pin or another lease's reference.

## Illustrative reachability failure

Suppose object R is reachable from an old world commit W. The current head no longer references W, so a head-only inventory would classify R as apparently unused. These labels are illustrative.

A replay capsule still pins W, and a remote lease naming R is unavailable. The complete projection retains the paths contributed by the replay pin and retains the remote lease's named roots. If the replay class was omitted entirely, the report would remain incomplete even if no visible path to R existed. Absence of a discovered path is not evidence that every possible retaining class was observed.

The [retention handoff](../../../src/world_distribution/retention.rs) appends retained, remote, and evidence references to existing destructive evidence. It combines existing index completeness with report completeness using conjunction. Thus a complete world report cannot repair an already-incomplete wider retention index. The handoff invokes the existing dry-run plan gate and explicitly returns `report_granted_deletion_authority: false`.

## Verification and limits

Suggested review, not executed test evidence: contrast an observed-empty retention class with a missing one; add an unresolved lease; remove edge completeness; and inspect how each changes the report. At the transfer boundary, substitute a target, encoded length, or protected-form flag and verify rejection before durable progress. Keep transport diagnostics distinct from canonical verification receipts.

Reachability is deterministic classification over supplied facts. It does not observe remote reality by itself, create policy, or authorize deletion. Actual destructive authority remains with the existing retention workflow, including its clearance, apply, execution, audit, and tombstone boundaries. Distribution receipts likewise do not establish global convergence, permanent durability, peer trust, semantic merge eligibility, or production readiness.

## Sources

- [World distribution and retention](../../world-distribution.md)
- [World replay capsules](../../world-replay-capsules.md)
- [Replication bridge](../../../src/world_distribution/bridge.rs)
- [Pure retention projection](../../../crates/molten-core/src/world_distribution/retention.rs)
- [Retention evidence handoff](../../../src/world_distribution/retention.rs)
- [Technical companion](../README.md)
