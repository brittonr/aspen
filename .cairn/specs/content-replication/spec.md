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

### Requirement: Historical success does not establish current availability
r[molten.audit_f04.current_availability] Molten MUST derive current replica availability from current admitted observations, not historical success alone.

#### Scenario: Current verified inventory counts
- GIVEN a replica matches the current generation, epochs, manifest, and protected form
- WHEN admitted inventory marks it present and verified
- THEN status counts that replica once.

#### Scenario: Same-epoch loss remains visible
- GIVEN only A remains present after a successful transfer to B in the same generation and epochs
- WHEN reconciliation consumes the fresh inventory and old success
- THEN B does not count as currently verified from history alone.

### Requirement: Loss requires bounded repair or unresolved demand
r[molten.audit_f04.fresh_repair] Molten MUST plan fresh bounded work or explicit unresolved demand after current observations contradict prior success.

#### Scenario: Repair verifies the lost target
- GIVEN A can supply content and B is eligible within the repair budget
- WHEN a new reconciliation observes B absent
- THEN a distinct admitted repair operation requires fresh content verification before success.

#### Scenario: Repair cannot proceed
- GIVEN repair attempts are exhausted or verification fails
- WHEN reconciliation handles the lost replica
- THEN under-replicated demand remains explicit and no historical success fills the deficit.

### Requirement: Replay and rejection preserve state
r[molten.audit_f04.replay_preservation] Molten MUST preserve exact-operation idempotency and MUST reject conflicting evidence without protected effects or durable state mutation.

#### Scenario: Exact replay retains historical meaning
- GIVEN an exact prior operation and its saved result
- WHEN the operation repeats without a new repair intent
- THEN replay returns the prior outcome without a second transfer or a new availability claim.

#### Scenario: Conflicting history cannot commit
- GIVEN prior history conflicts with the operation binding
- WHEN the core evaluates reuse
- THEN the operation rejects and preserves inventory, history, and retention state.

### Requirement: Evidence stays scoped to executed checks
r[molten.audit_f04.validation_claims] Molten MUST retain repository-owned positive and negative tests and MUST distinguish observed repair from historical outcome evidence.

#### Scenario: Regression runs through normal tests
- GIVEN the same-epoch loss fixture from F04
- WHEN normal core and controlled-adapter tests run after the change
- THEN they cover fresh repair and unresolved repair without ignored audit artifacts.

#### Scenario: Historical receipt cannot prove durability
- GIVEN only a historical verified transfer receipt
- WHEN a consumer requests current availability or permanent durability evidence
- THEN the receipt does not establish either claim.

### Requirement: Cleanup preserves the residual policy
r[molten.audit_f05.residual_policy] Molten MUST preserve desired replicas, minimum verified replicas, and minimum fault domains in the cumulative remainder after each cleanup selection.

#### Scenario: Redundant domain permits cleanup
- GIVEN A and B occupy zone A and C occupies zone B under a two-replica, two-domain policy
- WHEN all replicas have valid cleanup clearance and no pins
- THEN cleanup selects at most one of A and B and retains C.

#### Scenario: Unique domain blocks removal
- GIVEN a candidate is the only replica in a required domain
- WHEN its removal violates the minimum domain count
- THEN the planner does not select that candidate.

### Requirement: Cleanup selection is deterministic and conservative
r[molten.audit_f05.selection] Molten MUST evaluate safe alternatives deterministically and MUST retain replicas when no policy-preserving candidate exists.

#### Scenario: Input order does not change selection
- GIVEN equivalent inventories in different input orders
- WHEN the planner selects cleanup actions
- THEN the selected actions and plan reference match.

#### Scenario: Combined removals are unsafe
- GIVEN individual candidates appear safe against the original inventory but their combined removal violates policy
- WHEN the planner evaluates later candidates
- THEN it uses the cumulative remainder and blocks the unsafe removal.

### Requirement: Placement safety does not grant cleanup authority
r[molten.audit_f05.authority_preservation] Molten MUST retain separate cleanup admission and MUST preserve content and retention state after rejection.

#### Scenario: Admitted cleanup executes
- GIVEN a policy-safe action with current authority, valid clearance, and no active pin
- WHEN the shell executes cleanup
- THEN retention admission precedes the content cleanup call.

#### Scenario: Rejected cleanup has no deletion effect
- GIVEN missing authority, an active pin, stale clearance, or adapter denial
- WHEN cleanup admission rejects
- THEN no content deletion occurs and no successful deletion observation is emitted.

### Requirement: Cleanup evidence distinguishes plans from effects
r[molten.audit_f05.validation_claims] Molten MUST retain positive and negative repository tests and MUST distinguish safe-plan evidence from executed deletion evidence.

#### Scenario: Normal tests preserve domains
- GIVEN the F05 three-peer fixture
- WHEN normal planner and controlled-adapter tests run
- THEN they cover safe selection and rejected execution without ignored scratch files.

#### Scenario: Unsafe-plan evidence is not deletion evidence
- GIVEN the executed F05 pure-planner counterexample
- WHEN an audit or release summary describes its result
- THEN the summary identifies an unsafe plan and does not claim executed deletion or production data loss.

### Requirement: Deficits do not depend on actions
r[molten.audit_f07.deficit_status] Molten MUST preserve each unmet replica demand in status independently from the presence of executable actions.

#### Scenario: Satisfied inventory clears demand
- GIVEN current admitted inventory satisfies every replica rule
- WHEN status is projected without transfer actions
- THEN no under-replicated content is reported.

#### Scenario: No target retains demand
- GIVEN two replicas are required, one is verified, and no eligible target exists
- WHEN status is projected from an empty action list
- THEN the content remains under-replicated with an explicit insufficient-peer cause.

### Requirement: Partial success cannot hide residual demand
r[molten.audit_f07.per_content_accounting] Molten MUST resolve deficits only from sufficient current verified evidence for the same content reference.

#### Scenario: Enough verified results resolve demand
- GIVEN admitted results supply every missing replica for one content reference
- WHEN status applies those results
- THEN the deficit for that content clears.

#### Scenario: A successful subset remains partial
- GIVEN selected targets cover only part of the missing replicas
- WHEN all selected transfers succeed
- THEN residual demand remains under-replicated and another content result cannot fill it.

### Requirement: Unresolved demand prevents completion
r[molten.audit_f07.receipt_decision] Molten MUST prevent complete decisions for unresolved demand and MUST preserve durable state on rejection.

#### Scenario: Convergence permits completion
- GIVEN all demand is satisfied and no active operations or failures remain
- WHEN the shell classifies the receipt
- THEN completion is permitted under the existing admission rules.

#### Scenario: No-work deficit remains partial
- GIVEN unresolved demand with no target and no action
- WHEN the shell classifies the receipt
- THEN the receipt is not complete and no transfer effect is fabricated.

#### Scenario: Denial preserves state
- GIVEN invalid planning input or rejected verification
- WHEN the core or adapter rejects the work
- THEN prior durable state remains intact and the receipt does not claim completion.

### Requirement: Evidence retains its execution scope
r[molten.audit_f07.validation_claims] Molten MUST retain repository-owned positive and negative tests and MUST distinguish core observations from shell receipt evidence.

#### Scenario: Normal regression covers the shell
- GIVEN the F07 no-target fixture
- WHEN normal core and controlled-adapter tests run after implementation
- THEN status retains the deficit and the shell emits a non-complete receipt.

#### Scenario: Audit evidence cannot imply live execution
- GIVEN only the original F07 planner and status counterexample
- WHEN a report describes receipt behavior
- THEN it labels receipt classification as static evidence, not an executed live receipt.
