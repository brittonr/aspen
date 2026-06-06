# Runtime Spine Delta: SAM Service Supervision

### Requirement: Services start from demand and admitted authority
r[molten.sam_service_supervision.spec.demand_start] A service MUST start only when a matching demand assertion exists and startup is admitted by explicit authority, policy, resource, and effect-handle evidence.

#### Scenario: Demand starts service
- GIVEN a service manifest and a demand assertion for that service
- AND authority/resource evidence admits startup
- WHEN the service runtime evaluates demand
- THEN it emits a service lifecycle receipt with decision `pass`
- AND publishes owned readiness or startup assertions through the dataspace

#### Scenario: Missing authority denies startup
- GIVEN a demand assertion for a service
- AND no authority context admitting service startup
- WHEN the runtime evaluates demand
- THEN startup is denied before actor execution
- AND no readiness assertion is committed

### Requirement: Supervision is logical and deterministic
r[molten.sam_service_supervision.spec.supervision] Service links, monitors, restart decisions, and failure propagation MUST be expressed as canonical dataspace/evidence records and MUST replay deterministically.

#### Scenario: Failure notifies monitor
- GIVEN a monitored service fails during a turn
- WHEN the failure commits
- THEN monitor assertions/events are emitted in deterministic order
- AND the lifecycle receipt binds the failure and monitor refs

#### Scenario: Restart budget is exhausted
- GIVEN a service restart policy with a bounded restart rate
- WHEN failures exceed the bound
- THEN the runtime emits a deny receipt
- AND publishes a final failed/stopped assertion instead of restarting indefinitely

#### Scenario: Supervision gate receipt is evidence only
- GIVEN a canonical service supervision report
- WHEN an operator gates the report
- THEN the runtime emits a service supervision gate receipt binding report, suite, restart, monitor, status, and cleanup evidence
- AND the receipt decision is derived by deterministic replay
- AND the receipt is not authority, provenance, resource, policy, or transport trust evidence

### Requirement: Cleanup retracts owned state
r[molten.sam_service_supervision.spec.cleanup] Service termination, failure, shutdown, or authority revocation MUST retract service-owned assertions, observers, live refs, and pending effect intents.

#### Scenario: Revocation cleans assertions
- GIVEN a running service with owned readiness and exposed-reference assertions
- WHEN its authority context is revoked
- THEN cleanup retracts those assertions
- AND emits a cleanup receipt binding the revoked authority and retraction refs
