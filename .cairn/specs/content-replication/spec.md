# Content Replication Specification

## Purpose

Defines the `content-replication` capability.

## Requirements

### Requirement: Content replication is an explicit system extension
r[molten.content_replication.manifest] Molten MUST require a canonical system-extension manifest before enabling content replication. The manifest MUST bind service identity, generation, content and transport bindings, membership and placement profiles, desired replica policy, repair policy, retention policy, resource envelope, evidence profile, and non-claims.

#### Scenario: Replication activates explicitly
- GIVEN a reviewed manifest has compatible ports, authority, policy, resources, provenance, and placement inputs
- WHEN system-extension activation runs
- THEN replication MAY start under a generation-bound service instance
- AND ordinary content-store operations MUST remain usable without activating it.

### Requirement: Replication planning is pure and deterministic
r[molten.content_replication.planner] Molten MUST compute inventory differences, target replicas, source candidates, repairs, handoffs, deferrals, required pins, and cleanup candidates through a pure deterministic planner over canonical content, membership, placement, health, policy, resource, time, and operation-history facts.

#### Scenario: Same inputs produce the same plan
- GIVEN identical canonical inventories, epochs, policies, resources, and prior operations
- WHEN reconciliation planning runs repeatedly
- THEN target assignments, transfer order, repair actions, deferrals, and plan ref MUST match.

#### Scenario: Missing placement evidence denies planning
- GIVEN a manifest requires fault-domain placement but current placement evidence is missing or stale
- WHEN the planner evaluates a replica change
- THEN it MUST deny or defer the change without selecting ambient peers.

### Requirement: Replication is receiver-driven and idempotent
r[molten.content_replication.receiver_driven] Molten MUST require receivers to select missing content refs and admit bounded fetch plans before transfers can update verified availability, replica state, or retention pins. Exact operation-id repeats MUST reuse prior semantic outcomes, while conflicting reuse MUST deny.

#### Scenario: Receiver fetches a missing replica
- GIVEN a receiver lacks a required manifest and has an admitted current placement plan
- WHEN it requests and verifies the missing content
- THEN replica state MAY advance and MUST bind the plan, operation, source, receiver, and verified manifest refs.

#### Scenario: Unsolicited push is not imported
- GIVEN a peer sends content without a matching receiver plan
- WHEN the replication extension observes the transfer
- THEN it MUST NOT mark the content as an admitted replica or create an authority-bearing pin.

### Requirement: Placement and generation epochs fence stale work
r[molten.content_replication.epoch_fencing] Molten MUST bind plans, transfers, repairs, handoffs, cleanup, and replica assertions to current service generation, membership view, and placement epoch. Superseded results MUST NOT satisfy current replica targets or authorize source cleanup.

#### Scenario: Current repair commits
- GIVEN repair completes under the active generation and placement epoch
- WHEN its verified terminal result is applied
- THEN the current replica state MAY advance.

#### Scenario: Delayed old-epoch repair is fenced
- GIVEN placement changes while a transfer is in flight
- WHEN the old transfer completes
- THEN its result MUST be diagnostic-only or revalidated under the new plan
- AND it MUST NOT authorize cleanup from the current replica set.

### Requirement: Replication preserves retention and confidentiality
r[molten.content_replication.retention_confidentiality] Molten MUST acquire and validate canonical retention pins for active required replicas and transfers and MUST preserve manifest transform, encryption, redaction, reveal, and visibility policy. Replication MUST NOT decrypt, expose, unpin, delete, or downgrade protected content without separate admitted authority and evidence.

#### Scenario: Required replica remains pinned
- GIVEN a replica is part of the current admitted placement plan
- WHEN GC or cleanup evaluates its content
- THEN active replication and retention evidence MUST prevent unsafe removal.

#### Scenario: Protected content is not revealed for repair
- GIVEN a replica uses protected commitments or encrypted chunks
- WHEN repair verifies and transfers it
- THEN the extension MUST preserve the protected form unless separate reveal authority is admitted.

### Requirement: Replication work is bounded and failure-aware
r[molten.content_replication.resources_failures] Molten MUST enforce per-service, per-peer, and per-content bounds for concurrent transfers, bytes, retries, repair attempts, queue depth, timers, and diagnostics and MUST classify unavailable peer, uncertain delivery, corruption, cancellation, timeout, resource exhaustion, and repair exhaustion explicitly.

#### Scenario: Repair stays within budget
- GIVEN repair work remains inside admitted limits
- WHEN reconciliation executes
- THEN work MAY continue and resource observations MUST bind the service generation.

#### Scenario: Repair storm is throttled
- GIVEN many replicas become unavailable simultaneously
- WHEN planned work exceeds concurrency or byte limits
- THEN excess work MUST be deferred or denied without an unbounded hidden queue.

### Requirement: Replication uses the same core in simulation and live profiles
r[molten.content_replication.same_core] Molten MUST run the same replication manifest, planner, reconciliation transitions, idempotency, fencing, retention logic, and status model with live and deterministic-simulation adapters. Simulation-only replica shortcuts or direct state mutation MUST NOT satisfy conformance.

#### Scenario: Live and simulated repair share semantics
- GIVEN equivalent no-fault adapter capabilities and one repair plan
- WHEN the plan runs in live loopback and simulation
- THEN canonical plan, verification, replica-state, and terminal outcome traces MUST fall within the declared equivalent set.

### Requirement: Replication validation covers convergence and denial
r[molten.content_replication.final_validation] Molten MUST include positive and negative tests for initial placement, repair, handoff, restart, partial resume, duplicate operations, epoch changes, retention, corruption, partitions, unavailable peers, overload, cancellation, unauthorized push, stale cleanup, and evidence non-claims.

#### Scenario: Bounded placement converges
- GIVEN a finite healthy peer set, valid content, sufficient resources, and a stable admitted placement epoch
- WHEN reconciliation runs to its configured bound
- THEN required verified replicas MUST be present or the run MUST report explicit unresolved causes.

#### Scenario: Replica receipt cannot prove permanent durability
- GIVEN a passing bounded replication run
- WHEN a release claim treats it as permanent availability or whole-system durability proof
- THEN claim validation MUST deny that overreach.
