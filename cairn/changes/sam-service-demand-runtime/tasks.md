## Phase 0: Prerequisite binding

- [x] [serial] r[molten.sam_service_demand_runtime.spec.admitted_demand_start] Require canonical service record parsing and ledger classification from `sam-service-records-ledger` before runtime startup is enabled.
- [x] [parallel] r[molten.sam_service_demand_runtime.spec.admitted_demand_start] Deny demand-driven startup when a caller supplies only text service names, local paths, or ambient process handles.

## Phase 1: Demand observation and dependency resolution

- [x] [serial] r[molten.sam_service_demand_runtime.spec.admitted_demand_start] Observe `service-demand-v1` assertions through the local dataspace kernel and bind demand assertion refs into lifecycle receipts.
- [x] [serial] r[molten.sam_service_demand_runtime.spec.dependency_resolution] Resolve required service readiness/status refs deterministically before startup.
- [x] [parallel] r[molten.sam_service_demand_runtime.spec.dependency_resolution] Enforce bounded dependency graph size and deny cycles or missing dependencies with diagnostics.
- [x] [parallel] r[molten.sam_service_demand_runtime.spec.dependency_resolution] Emit dependency-wait lifecycle receipts instead of silently ignoring unmet demand.

## Phase 2: Startup admission and owned assertions

- [x] [serial] r[molten.sam_service_demand_runtime.spec.admitted_demand_start] Gate service startup through authority, policy, resource, effect-handle, and strict source-gate evidence before actor execution.
- [x] [serial] r[molten.sam_service_demand_runtime.spec.owned_assertion_replay] Commit readiness/degraded/failure/stopped assertions as service-owned canonical dataspace facts.
- [x] [parallel] r[molten.sam_service_demand_runtime.spec.owned_assertion_replay] Bind service demand/startup/owned-assertion refs into actor-scoped turn-journal context refs.
- [x] [parallel] r[molten.sam_service_demand_runtime.spec.admitted_demand_start] Ensure failed admission or unmet dependencies emit denial/wait receipts and perform no actor start side effects.

## Phase 3: Replay and CLI fixture

- [x] [serial] r[molten.sam_service_demand_runtime.spec.owned_assertion_replay] Bind demand, manifest, dependency, authority, policy, resource, effect-handle, source-gate, scheduler, and effect-log refs into replay identity.
- [x] [serial] r[molten.sam_service_demand_runtime.spec.admitted_demand_start] Add a deterministic `test service run-two-service` CLI or equivalent harness fixture.
- [x] [parallel] r[molten.sam_service_demand_runtime.spec.owned_assertion_replay] Add safe rendered summaries for service lifecycle outputs while keeping Preserves receipts normative.
- [x] [parallel] r[molten.sam_service_demand_runtime.spec.owned_assertion_replay] Ensure service lifecycle pass evidence can be bound into harness/node dogfood gate receipts.

## Phase 4: Tests

- [x] [serial] r[molten.sam_service_demand_runtime.spec.admitted_demand_start] Test demand starts a dependency and readiness satisfies an observer.
- [x] [serial] r[molten.sam_service_demand_runtime.spec.admitted_demand_start] Test missing authority, unmet dependency, missing source gate, and malformed manifest denial before actor execution.
- [x] [parallel] r[molten.sam_service_demand_runtime.spec.owned_assertion_replay] Test replay divergence for changed dependency readiness, admission decision, or effect-log refs.
- [x] [parallel] r[molten.sam_service_demand_runtime.spec.dependency_resolution] Add Hegel properties for demand identity, dependency resolution determinism, and no-side-effects-on-deny.
