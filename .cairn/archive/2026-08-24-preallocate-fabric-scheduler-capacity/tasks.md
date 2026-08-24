## Phase 1: Baseline and plan

- [x] [serial] `[scheduler-capacity-baseline]` Run current scheduler ordering, overload, fairness, replay, generation, and fixture checks before core changes. r[molten.fabric_time.scheduler_capacity.verification]
- [x] [depends:scheduler-capacity-baseline] `[scheduler-capacity-plan]` Derive a checked scheduler-capacity plan from the admitted profile and active generation. r[molten.fabric_time.scheduler_capacity.plan]
- [x] [parallel] `[scheduler-capacity-plan-tests]` Add valid, zero, one-past-cap, relation, conversion, and arithmetic-overflow fixtures. r[molten.fabric_time.scheduler_capacity.verification]

## Phase 2: Activation and steady state

- [x] [depends:scheduler-capacity-plan] `[scheduler-capacity-activation]` Allocate planned runnable and queue capacity before scheduler activation and deny without fallback on failure. r[molten.fabric_time.scheduler_capacity.activation]
- [x] [depends:scheduler-capacity-activation] `[scheduler-capacity-transitions]` Fence capacity by profile and generation and prevent steady-state transition growth. r[molten.fabric_time.scheduler_capacity.steady_state]
- [x] [parallel] `[scheduler-capacity-parity]` Preserve FIFO, priority, fairness, overload, recorded-choice replay, and cleanup semantics. r[molten.fabric_time.scheduler_capacity.boundary]

## Phase 3: Observation and closeout

- [x] [depends:scheduler-capacity-transitions] `[scheduler-capacity-observation]` Add plan, activation, usage, high-water, exhaustion, release, generation, and profile observations. r[molten.fabric_time.scheduler_capacity.observation]
- [x] [parallel] `[scheduler-capacity-negative-tests]` Add allocation-failure, stale-generation, wrong-profile, exhaustion, hidden-growth, restart, and overclaim tests. r[molten.fabric_time.scheduler_capacity.verification]
- [x] [serial] `[scheduler-capacity-validation]` Run focused fabric-time fixtures, workspace tests, Clippy, Octet, Cairn validation, and relevant Nix checks. r[molten.fabric_time.scheduler_capacity.verification]
