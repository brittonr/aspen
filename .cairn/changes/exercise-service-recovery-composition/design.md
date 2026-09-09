# Design: Exercise a service recovery composition end to end

## Context

The deterministic harness composes pure services over canonical envelopes; the real-process track launches actual node processes. The composition deliberately stays small: three services with the minimum relationships that expose every property — one durable stateful service (the dependency), one dependent worker in the same recovery group, one independent sibling outside it.

## Approach

1. **Shared definitions.** Define the three services once, through the typed facades, with a recovery-group declaration ordering the dependent worker after the stateful service. Both harness tracks consume the same definitions.
2. **Scenario matrix.** One deterministic scenario per property: member failure with dependent restart; delayed pre-restart event delivery; durable work readmission; uncertain effect surfacing; CPU saturation under time slicing with the sibling making progress; queue overload under admitted backpressure; restart storm to escalation; generation upgrade with stale-work rejection. Each scenario asserts all five properties that apply to it, so a regression in one mechanism fails multiple scenarios.
3. **Negative variants.** For fencing and storm properties, include deliberately broken composition variants (fencing check removed; group budget reset on replacement). The harness must fail these; a passing broken variant means the property assertion is vacuous.
4. **Live twin.** The real-process case replays the failure classes that survive process boundaries: crash-and-restart fencing, durable readmission, storm termination, and the upgrade. Observations map back to the deterministic trace's assertion vocabulary so results are comparable; timing-dependent checks use admitted observations, not wall-clock assertions.
5. **Failure routing.** When a property fails because an owning change's mechanism is defective, the fix belongs to that change. This change records the failing property as evidence and does not patch mechanism code.

## Alternatives considered

- Per-change integration tests only. Rejected: the review's core point is that the guarantee lives in the interactions; isolated tests cannot see a stale timer slipping past a restart into a restarted dependent.
- A larger composition with a mesh of dependencies. Rejected: more services make failure attribution harder without adding a property; ordered groups already cover the interesting shape.

## Non-claims

- A passing composition does not prove whole-runtime correctness, distributed failure detection, or production workload performance.
- Live-track agreement with the deterministic track does not prove the live adapters are deterministic.

## Risks

- The composition depends on four in-flight changes; scenario work starts only after the owning change lands, and this change's tasks order accordingly.
- Real-process fault injection flakiness is bounded by the existing outside-in fault track conventions.
