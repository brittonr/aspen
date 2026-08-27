# Molten World Commit Specification Delta

## Purpose

Replicate immutable world closure and detached head claims while preserving local authority and conservative retention.

## ADDED Requirements

### Requirement: World closure uses typed bounded DAG synchronization

r[molten.world_distribution.closure] Molten MUST project world commits and typed root edges into the generic DAG-sync boundary. Received objects MUST match requested identities, domains, bounds, and closure before activation.

#### Scenario: Complete closure is received

- GIVEN a peer supplies every requested commit and typed root object within declared bounds
- WHEN closure validation runs
- THEN Molten MUST report a complete immutable closure for local admission

#### Scenario: Peer substitutes one domain

- GIVEN bytes match a digest but the object domain differs from the requested root type
- WHEN closure validation runs
- THEN Molten MUST reject the object and keep the closure incomplete

### Requirement: Head claims replicate separately from immutable content

r[molten.world_distribution.head_claims] Molten MUST exchange detached signed head claims separately from world objects. Local authentication, current policy, authority, ancestry, and conflict rules MUST decide whether any claim can affect a local head.

#### Scenario: Remote claim is locally authorized

- GIVEN the statement authenticates and current local branch policy admits its transition
- WHEN local claim admission runs
- THEN the claim MAY enter local head-transition planning

#### Scenario: Two remote claims compete

- GIVEN two admissible claims name different successors for the same expected state
- WHEN local claim admission runs
- THEN Molten MUST preserve a conflict set and MUST NOT select by arrival order

### Requirement: Retention roots cover recovery and unresolved work

r[molten.world_distribution.retention_roots] Molten MUST include current and competing heads, executions, task checkpoints, replay and simulation pins, merge conflicts, promotion and reconciliation state, rollback and evidence holds, remote leases, and incomplete transfers in world retention projection.

#### Scenario: Promotion outcome is unresolved

- GIVEN a promoted effect has an uncertain external observation
- WHEN retention roots are projected
- THEN the related commit, intent, reservation, attempt, and observation objects MUST remain reachable

#### Scenario: Active execution root is missing

- GIVEN an execution profile may retain a world root but provides no current observation
- WHEN retention projection runs
- THEN the inventory MUST remain incomplete

### Requirement: Reachability does not grant deletion authority

r[molten.world_distribution.gc_boundary] World reachability and retirement reports MUST remain evidence inputs. Existing retention policy, remote clearance, tombstone, apply, execute, audit, and destructive authority gates MUST run before deletion.

#### Scenario: Commit is unreachable but legally held

- GIVEN complete reachability finds no active runtime path but retention policy records a legal hold
- WHEN garbage collection is requested
- THEN Molten MUST retain the content

#### Scenario: Remote state is unknown

- GIVEN a required peer observation is stale or unavailable
- WHEN deletion planning runs
- THEN Molten MUST retain affected content or report an unresolved blocker

### Requirement: Partial synchronization is resumable and non-authoritative

r[molten.world_distribution.partial] Molten MUST record partial sync progress under one operation identity. Resume MUST revalidate prior objects, requested root, peer policy, and remaining closure. Partial receipt MUST NOT grant activation or availability claims.

#### Scenario: Transfer stops after some roots

- GIVEN a bounded transfer ends before closure completion
- WHEN the operation is inspected
- THEN Molten MUST report exact durable progress and block activation

#### Scenario: Resume observes a changed request

- GIVEN the requested root or peer policy differs from the saved operation
- WHEN resume planning runs
- THEN Molten MUST reject reuse of the stale operation

### Requirement: Distribution verification covers authority and retention denial

r[molten.world_distribution.verification] Molten MUST test complete and partial transfer, corruption, domain confusion, bounds, claim conflicts, stale authority, retention omissions, remote uncertainty, and destructive overclaims.

#### Scenario: Focused distribution rail runs

- GIVEN positive and negative fixtures use reviewed DAG, replication, binding, and retention cohorts
- WHEN the focused verification rail runs
- THEN it MUST report closure and retention facts without claiming convergence or permanent durability
