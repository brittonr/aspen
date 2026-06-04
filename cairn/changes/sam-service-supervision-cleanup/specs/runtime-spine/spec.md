## ADDED Requirements

### Requirement: Supervision is logical and receipt-backed
r[molten.sam_service_supervision_cleanup.spec.logical_supervision] Service links, monitors, failure propagation, and restart decisions MUST be represented as canonical logical records and receipts independent from OS process parentage.

#### Scenario: Failure notifies monitors deterministically
- GIVEN a running service with two monitor records
- WHEN the service commits a failure transition
- THEN Molten emits monitor notification refs in deterministic order
- AND the failure lifecycle receipt binds the monitor refs and notification refs

#### Scenario: OS parentage is not supervision evidence
- GIVEN an OS process tree or ambient parent pid without canonical service link records
- WHEN Molten evaluates service supervision evidence
- THEN the OS parentage data is rejected as pass evidence
- AND no restart or monitor decision is admitted from it

### Requirement: Restart policy is bounded and replayable
r[molten.sam_service_supervision_cleanup.spec.bounded_restart] Restart decisions MUST be bounded by explicit restart policy, authority state, logical resource budgets, and recorded lifecycle refs; unbounded restart loops MUST deny.

#### Scenario: Restart attempt passes within bounds
- GIVEN a failed service with a restart policy that allows another attempt
- AND authority/resource evidence remains valid
- WHEN restart evaluation runs
- THEN Molten emits a restart decision receipt with decision `pass`
- AND schedules startup through the demand runtime path

#### Scenario: Restart budget exhausted denies
- GIVEN a failed service whose restart attempts exceed the policy window
- WHEN restart evaluation runs
- THEN Molten emits a restart denial receipt
- AND publishes a final failed or stopped status instead of restarting indefinitely

### Requirement: Cleanup retracts only proven service-owned state
r[molten.sam_service_supervision_cleanup.spec.owned_cleanup] Service stop, failure, shutdown, or authority revocation MUST retract service-owned assertions, observers, live refs, exposed refs, and pending effect intents, and MUST NOT delete state whose ownership cannot be proven.

#### Scenario: Revocation cleans owned readiness
- GIVEN a running service with owned readiness and exposed-reference assertions
- AND its owner authority is revoked
- WHEN cleanup runs
- THEN Molten retracts the owned assertions
- AND emits a cleanup receipt binding the revoked authority and retraction refs

#### Scenario: Foreign state is not deleted
- GIVEN cleanup input that includes an assertion owned by another service
- WHEN cleanup validates ownership
- THEN cleanup denies deletion of the foreign assertion
- AND records deterministic diagnostics in the cleanup receipt

### Requirement: Cleanup evidence is replay and retention input
r[molten.sam_service_supervision_cleanup.spec.cleanup_replay_retention] Cleanup receipts MUST bind prior lifecycle, ownership, revocation, resource, and retraction refs so replay can detect cleanup divergence and retention/GC can consume cleanup evidence without bypassing retention policy.

#### Scenario: Replay detects missing retraction
- GIVEN a recorded cleanup receipt with three owned assertion retractions
- WHEN replay observes only two matching retractions
- THEN replay fails at cleanup verification
- AND reports the missing retraction ref

#### Scenario: Retention still gates deletion
- GIVEN a cleanup receipt proving service-owned assertion retraction
- WHEN retention/GC evaluates physical deletion eligibility
- THEN the cleanup receipt is treated as input evidence
- AND retention policy still decides whether deletion is admitted
