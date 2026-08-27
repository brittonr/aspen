# World distribution and retention

Molten distributes immutable world objects without making replication an authority source.

## Ownership

Molten owns world-commit meaning, typed root domains, local claim admission, and complete retention-root policy.

The generic DAG-sync core owns bounded traversal, resume fencing, and verify-before-progress behavior. The content-replication core owns replica placement, transfer actions, protected-form checks, and repair policy. Artifact Binding Core owns deterministic reachability and pin-path classification.

The existing Molten retention workflow still owns policy, remote clearance, dry-run planning, apply, execution, audit, tombstones, and destructive authority.

## Immutable closure

A world DAG contains one node for each canonical world commit and typed root object. Commit edges point to parent commits and required roots. The adapter verifies each canonical commit identity before it creates the graph.

The supported distribution cohort has these bounds:

- At most `MAX_WORLD_DISTRIBUTION_OBJECTS` objects.
- At most `MAX_WORLD_DISTRIBUTION_BYTES` encoded bytes.
- Generic DAG depth, edge, step, peer, and progress bounds remain active.
- The requested root, epoch, generation, policy, peers, and prior progress bind every resume.

A complete DAG receipt means that the receiver verified all requested objects. It does not grant activation, execution, merge, publication, or head-selection authority.

## Content transfer

`WorldReplicationBridge` maps only planned world objects to generic content-replication actions. It rejects an object outside the closure, a changed target, a changed operation, a changed encoded length, or a changed protected form.

The bridge gives the generic DAG shell a transfer port and a verification port. The shell records durable DAG progress only after content verification succeeds. It publishes the generic receipt and then the world receipt.

World replication disables cleanup and handoff cleanup in its manifest. A later retention workflow must make each cleanup decision.

## Detached head claims

Head claims use a separate exchange path. The shell obtains these facts for each claim:

1. The bounded transport carrier.
2. The authentication decision.
3. The current local authority observation.
4. The durable currentness observation.
5. The current local branch policy and history.

The pure head protocol evaluates each claim. Competing admitted successors remain an explicit conflict set. Arrival order never selects a claim. The exchange output never mutates a local head.

## Retention roots

The retention projection requires an explicit observation for each closed class:

- current and competing heads;
- active executions and task checkpoints;
- replay, simulation, and comparison pins;
- merge conflicts;
- promotion and reconciliation state;
- rollback, legal, evidence, and operator holds;
- remote leases; and
- incomplete transfers.

An observed empty class is different from a missing class. A missing class keeps the reference index incomplete.

Active, uncertain, contradictory, and unavailable remote lease observations retain their named roots. Uncertain remote observations also create blockers. A cleared lease does not retain roots.

The adapter maps complete facts to Artifact Binding reachability. The result is observation-only. `handoff_world_retention` adds the report to the existing retention evidence and runs only the existing dry-run plan gate. The report does not add policy or deletion authority.

## Operator commands

Use `molten world-distribution` for bounded operator views:

- `sync-plan` writes a canonical plan without transfer effects.
- `closure-inspect` lists the typed local closure.
- `claims-inspect` lists stored conflicts without selecting a claim.
- `pins-inspect` lists known retention pins.
- `retention-explain` reads the existing retention evidence graph.
- `sync` and `resume` fail closed in the standalone CLI. A composition root must supply current authority, resources, peers, content, progress, and receipt ports.

## Failure behavior

Molten denies or keeps work incomplete when it finds any of these conditions:

- a wrong commit identity or root domain;
- a missing descriptor, parent, or root;
- a cycle or exceeded bound;
- unsolicited inventory or transfer content;
- corrupt or unauthorized content;
- stale resume context;
- stale or denied claim authority;
- competing claims;
- a missing retention-class observation;
- an uncertain remote lease; or
- a reachability report presented as deletion authority.

## Non-claims

World distribution does not prove global convergence, permanent durability, peer trust, application correctness, merge eligibility, activation authority, or safe deletion. Evidence remains an input to the owner of each later decision.
