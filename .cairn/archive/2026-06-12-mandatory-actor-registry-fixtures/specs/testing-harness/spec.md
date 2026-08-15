## ADDED Requirements

### Requirement: Evidence-bearing suites require explicit actor registries
r[molten.testing.mandatory_actor_registry.explicit_fixture] Evidence-bearing harness suites MUST include an explicit actor registry fixture or equivalent actor/executor proof refs. Omitted actor registries MUST NOT be inferred from steps, capability grants, policy rules, observations, or runner defaults for execution, validation, or pass-evidence gates.

#### Scenario: Omitted registry fails execution
r[molten.testing.mandatory_actor_registry.explicit_fixture.omitted]
- GIVEN a harness suite with actor-referencing steps, explicit capabilities, and no `<actor-registry-v1 ...>` fixture
- WHEN the evidence-bearing local runner attempts to execute the suite
- THEN the runner rejects it before actor executor setup, runtime turns, admission decisions, or ambient effect requests occur

#### Scenario: Explicit empty registry is valid only for empty actor use
r[molten.testing.mandatory_actor_registry.explicit_fixture.empty]
- GIVEN a harness suite with `<actor-registry-v1 "molten.harness.actor-registry.v1" []>`
- WHEN the suite contains no actor-referencing steps or evidence
- THEN the registry fixture is explicit and may satisfy the registry preflight
- BUT WHEN any step references an actor absent from the explicit registry
- THEN normal unknown-actor preflight rejects the suite

### Requirement: Report validation rejects inferred actor registries
r[molten.testing.mandatory_actor_registry.validation] Report validation MUST reject embedded suites that omitted explicit actor registry evidence, even if the report contains actor-registry evidence inferred by an older runner.

#### Scenario: Legacy report with inferred actors fails validation
r[molten.testing.mandatory_actor_registry.validation.legacy]
- GIVEN a report produced by an older runner whose embedded suite omitted the actor registry fixture
- WHEN `molten test report validate` evaluates the report
- THEN validation fails closed with missing explicit actor registry diagnostics

#### Scenario: Report actors must match explicit registry
r[molten.testing.mandatory_actor_registry.validation.mismatch]
- GIVEN a report whose embedded suite declares an explicit actor registry
- WHEN observations, effect records, admission requests, or final state mention an actor not present in that registry
- THEN validation rejects the report rather than accepting an inferred actor

### Requirement: Executor selection is a fail-closed boundary
r[molten.testing.mandatory_actor_registry.executor_boundary] Actor registry entries MUST bind actor ids to executor kinds, and evidence-bearing execution MUST NOT silently coerce unsupported or unreviewed kinds to native execution. Unsupported Steel, Wasm, adapter, and remote-proxy actors MUST fail until their executor boundary evidence is implemented and reviewed.

#### Scenario: Unsupported kind cannot fall back to native
r[molten.testing.mandatory_actor_registry.executor_boundary.unsupported]
- GIVEN an explicit actor registry containing `<actor "a" "wasm">` before Wasm executor boundary evidence is supported
- WHEN the evidence-bearing local runner attempts to execute a step for actor `a`
- THEN the runner rejects the suite before the step executes and does not run actor `a` as native

#### Scenario: Future executor evidence is explicit
r[molten.testing.mandatory_actor_registry.executor_boundary.future]
- GIVEN a future Steel, Wasm, adapter, or remote-proxy actor kind
- WHEN it participates in deterministic pass evidence
- THEN its registry entry is bound to explicit executor manifest, policy, replay, or exclusion evidence rather than runner defaults

### Requirement: Pass-evidence receipts prove no inferred actors
r[molten.testing.mandatory_actor_registry.gate_checks] Successful pass-evidence gate receipts MUST include checks proving the accepted report used an explicit actor registry, no inferred actors, and a reviewed executor boundary.

#### Scenario: Receipt includes explicit registry checks
r[molten.testing.mandatory_actor_registry.gate_checks.receipt]
- GIVEN a deterministic report with an explicit actor registry that validates and replays successfully
- WHEN `molten test gate check` emits a pass receipt
- THEN the receipt includes `explicit-actor-registry`, `no-inferred-actors`, and `executor-boundary` checks in addition to actor-registry, capability, policy, admission, budget, effect-log, and replay checks

### Requirement: Examples declare actor registries
r[molten.testing.mandatory_actor_registry.examples] Repository examples and positive harness tests MUST declare explicit actor registries for every actor they expect to use. Negative actor tests MUST use explicit registries with missing actors or unsupported kinds, not omitted registries, unless the test specifically targets omitted-registry failure.

#### Scenario: Two-actor example declares registry
r[molten.testing.mandatory_actor_registry.examples.two_actor]
- GIVEN the repository two-actor example suite
- WHEN it is run through the harness and gated as pass evidence
- THEN the suite includes explicit native actor entries for `consumer` and `producer`

### Requirement: Future executor evidence remains explicit
r[molten.testing.mandatory_actor_registry.future_executor_evidence] Future Steel, Wasm, adapter, and remote-proxy executor integration MUST preserve the invariant that missing executor-boundary evidence fails closed. Executor manifests, hostcall capabilities, adapter contracts, remote identity refs, non-replayable exclusions, and simulation receipts MAY replace the first local native-only executor check, but they MUST be explicit and bound to run identity.

#### Scenario: Missing future executor proof fails closed
r[molten.testing.mandatory_actor_registry.future_executor_evidence.missing]
- GIVEN a future evidence-bearing report whose actor registry includes a non-native executor kind
- WHEN the required executor manifest or boundary receipt is omitted
- THEN validation rejects the report rather than treating missing executor evidence as native execution authority
