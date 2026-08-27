## Context

The world-commit core forms a typed immutable DAG. Head claims form a separate mutable and potentially conflicting statement set.

Molten already has content refs, Iroh transport, retention indexes, remote-clearance workflows, and planned generic DAG-sync and replication extensions. This change must adapt those mechanisms instead of creating a second transport or garbage collector.

## Decisions

### Decision: Reuse generic DAG and content extensions

**Choice:** Project world commits and typed root edges into the planned DAG-sync contract. Fetch immutable objects through the content-replication extension.

World semantics remain in the adapter. Generic traversal and transfer cores do not gain restore, merge, activation, or authority meaning.

**Rationale:** Traversal and transfer are reusable mechanisms. World closure and branch policy are product semantics.

### Decision: Separate immutable closure from mutable claims

**Choice:** Sync world commits and roots by immutable identity. Exchange signed head claims through a distinct bounded claim protocol.

A peer can request one commit closure without accepting any remote branch claim.

**Rationale:** Content availability must not imply branch selection or trust.

### Decision: Preserve claim conflicts locally

**Choice:** The receiver authenticates claim statements, then applies current local policy and authority. Multiple admissible successors remain a conflict set.

No transport arrival order, peer priority, or last-writer rule selects a semantic winner.

**Rationale:** Replication cannot manufacture merge or authority decisions.

### Decision: Require complete admitted closure before activation

**Choice:** A received head remains unavailable for activation until every required typed root is present, identity-valid, schema-admitted, and within profile bounds.

Optional unavailable roots remain explicit only when the snapshot profile permits them.

**Rationale:** A head identity without its causal closure cannot support reliable restore or replay.

### Decision: Define complete retention roots

**Choice:** Retention projection includes:

- current local heads and bounded competing claims;
- active or recoverable executions and task checkpoints;
- replay, simulation, comparison, and merge pins;
- unresolved merge conflicts;
- promotion reservations, attempts, observations, and reconciliation state;
- rollback, legal, evidence, and operator holds;
- admitted remote leases and incomplete-transfer recovery pins.

Every root has a typed owner and completeness state.

**Rationale:** Collecting only current heads can delete state required for recovery, reconciliation, or review.

### Decision: Keep reachability separate from deletion authority

**Choice:** Artifact Binding Core computes deterministic reachability and pin paths from supplied complete facts. Existing Molten retention gates decide eligibility and deletion.

An incomplete inventory cannot produce a safe-delete decision.

**Rationale:** Reachability is evidence. It does not grant destructive authority.

### Decision: Treat remote observations conservatively

**Choice:** Remote leases and clearance observations bind peer, scope, generation, root set, validity basis, and observation identity.

Missing, stale, contradictory, or unreachable peer state retains affected content or reports an explicit unresolved blocker.

**Rationale:** Local absence of evidence does not prove remote disuse.

### Decision: Make sync resumable and bounded

**Choice:** Sync plans have explicit node, depth, byte, retry, peer, and elapsed-policy bounds. Partial progress is durable under one operation identity.

Resume revalidates the requested root, peer policy, prior received objects, and remaining closure.

**Rationale:** Large worlds require bounded transfer and crash recovery without accepting unsolicited closure expansion.

## Rollout

1. Add local world-DAG projection and closure reports.
2. Pilot one loopback immutable closure transfer.
3. Add detached claim exchange with no automatic head selection.
4. Integrate retention-root observation in report-only mode.
5. Enable retention handoff only after incomplete and remote-uncertainty fixtures pass.

## Risks / Trade-offs

- Complete worlds can be large. Content deduplication and resumable bounded transfer reduce repeated work.
- Conservative remote handling retains extra content. This is safer than deleting recovery state.
- Signed remote claims can be valid but unauthorized locally. Preserve authentication and authorization separation.
- Replication and retention evidence do not prove permanent availability or safe application behavior.
