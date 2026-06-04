## ADDED Requirements

### Requirement: Demand startup is admitted before side effects
r[molten.sam_service_demand_runtime.spec.admitted_demand_start] A service MUST start from a canonical demand assertion only after dependency readiness and explicit authority, policy, resource, effect-handle, and source-gate evidence pass.

#### Scenario: Demand starts dependency after gates pass
- GIVEN a `service-demand-v1` assertion for service `svc:frontend`
- AND a canonical manifest for `svc:frontend` requiring `svc:backend`
- AND `svc:backend` has a ready status assertion
- AND startup authority, policy, resource, effect-handle, and strict source-gate evidence pass
- WHEN the service demand runtime evaluates demand
- THEN it commits a service lifecycle receipt with decision `pass`
- AND it publishes service-owned readiness/status assertions for `svc:frontend`

#### Scenario: Missing source gate denies before actor execution
- GIVEN a valid demand assertion and manifest
- AND all dependencies are ready
- BUT strict source-gate evidence is missing or denied
- WHEN the service demand runtime evaluates demand
- THEN startup denies before actor execution
- AND no readiness assertion is committed

### Requirement: Dependency readiness is deterministic and bounded
r[molten.sam_service_demand_runtime.spec.dependency_resolution] Service dependency readiness MUST be resolved from canonical service status/readiness refs within bounded graph limits, and unmet, stale, cyclic, or missing dependencies MUST produce deterministic wait or denial receipts.

#### Scenario: Unmet dependency waits
- GIVEN a demand assertion for a service whose required dependency has no ready status assertion
- WHEN demand evaluation runs
- THEN the runtime emits a dependency-wait lifecycle receipt
- AND it performs no actor start side effects

#### Scenario: Dependency cycle denies
- GIVEN service manifests whose `requires` relations form a cycle outside supported bounds
- WHEN dependency resolution runs
- THEN the runtime emits deterministic denial diagnostics
- AND no service in the cycle is started by that demand evaluation

### Requirement: Service-owned assertions are replay-bound
r[molten.sam_service_demand_runtime.spec.owned_assertion_replay] Readiness, degraded, failure, and stopped assertions emitted by service startup MUST be owned by the service and bound into replay identity with demand, dependency, authority, resource, scheduler, and effect-log refs.

#### Scenario: Readiness owner is bound
- GIVEN an admitted service startup emits a readiness assertion
- WHEN the lifecycle receipt is generated
- THEN the receipt binds the service id, manifest ref, demand ref, authority/resource/effect refs, and readiness assertion ref
- AND later cleanup can identify the assertion as service-owned

#### Scenario: Replay detects changed dependency status
- GIVEN a recorded service startup replay identity
- AND the dependency readiness ref changes before replay
- WHEN replay validates the service lifecycle
- THEN replay fails at the dependency decision
- AND reports deterministic first-divergence diagnostics
