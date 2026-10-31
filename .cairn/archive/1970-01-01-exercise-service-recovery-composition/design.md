# Design: Exercise a service recovery composition end to end

## Context

The deterministic harness composes pure services over canonical envelopes; the real-process track launches actual node processes. The composition deliberately stays small: three services with the minimum relationships that expose every property — one durable stateful service (the dependency), one dependent worker in the same recovery group, one independent sibling outside it.

## Approach

1. **Shared definitions.** Define the three services once, through the typed facades, with a recovery-group declaration ordering the dependent worker after the stateful service. Both harness tracks consume the same definitions.
2. **Scenario matrix.** One deterministic scenario per property: member failure with dependent restart; delayed pre-restart event delivery; durable work readmission; uncertain effect surfacing; CPU saturation under time slicing with the sibling making progress; queue overload under admitted backpressure; restart storm to escalation; generation upgrade with stale-work rejection. Each scenario asserts all five properties that apply to it, so a regression in one mechanism fails multiple scenarios.
3. **Negative variants.** For fencing, storm, saturation and upgrade properties, include deliberately broken composition variants (fencing check removed; group budget reset on replacement; callback yields ignored until completion; admitted state transformation skipped). The harness must fail these; a passing broken variant means the property assertion is vacuous. Both modeled work and a real Wasmtime callback exercise saturation. A stale-generation denial alone is insufficient upgrade evidence: drain, checkpoint, transformed activation, durable readmission and sibling progress must also hold.
4. **Live twin.** The actual public node owns native supervisors and real capability-rooted journals under the separately governed `host-node-recovery-groups` prerequisite. The test launches an independently source-admitted real Nix node with genuine exact UCAN operation credentials, waits for that current serving child's canonical hosted-ready observation, kills that same process and reopens through public operations. It covers crash/restart fencing, durable readmission, persisted storm termination and drain/checkpoint/materialized recovery/activation with a second activated-generation loss and migrated work. Observations use the deterministic property vocabulary; admitted logical ticks drive judgments and wall time bounds supervision only. Concrete fault-build source mutations are built/admitted/signed in separate source cohorts, never injected as fabricated observations or test-owned supervisors. Without actual independent good/fault source trusts and native admission cohorts, the explicit external tests fail and lifecycle closure stays open; no model fallback is allowed.
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
