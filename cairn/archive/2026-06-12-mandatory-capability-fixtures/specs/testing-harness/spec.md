## ADDED Requirements

### Requirement: Evidence-bearing suites require explicit capability fixtures
r[molten.testing.mandatory_capabilities.explicit_fixture] Evidence-bearing harness suites MUST include an explicit capability fixture or equivalent authority proof refs. Omitted capability fixtures MUST NOT be normalized to implicit authority for execution, validation, or pass-evidence gates.

#### Scenario: Omitted fixture fails execution
r[molten.testing.mandatory_capabilities.explicit_fixture.omitted]
- GIVEN a harness suite with actor steps but no `<capabilities-v1 ...>` fixture
- WHEN the evidence-bearing local runner attempts to execute the suite
- THEN the runner rejects it before any runtime turn or ambient effect request occurs

#### Scenario: Explicit empty fixture is valid authority context
r[molten.testing.mandatory_capabilities.explicit_fixture.empty]
- GIVEN a harness suite with `<capabilities-v1 "molten.harness.capabilities.v1" []>`
- WHEN a step requests a send, assertion, observation, or effect
- THEN the request is denied through normal admission evidence rather than rejected as malformed suite evidence

### Requirement: Report validation rejects implicit authority
r[molten.testing.mandatory_capabilities.validation] Report validation MUST reject embedded suites that omitted explicit capability evidence, even if the report contains a capability gate record over a compatibility/default context.

#### Scenario: Legacy report with default authority fails validation
r[molten.testing.mandatory_capabilities.validation.legacy]
- GIVEN a report produced by an older runner whose embedded suite omitted capability fixtures
- WHEN `molten test report validate` evaluates the report
- THEN validation fails closed with missing explicit capability fixture diagnostics

### Requirement: Pass-evidence receipts prove no implicit authority
r[molten.testing.mandatory_capabilities.gate_checks] Successful pass-evidence gate receipts MUST include checks proving the accepted report used explicit capability evidence and no implicit authority default.

#### Scenario: Receipt includes explicit authority checks
r[molten.testing.mandatory_capabilities.gate_checks.receipt]
- GIVEN a deterministic report with explicit capability grants that validates and replays successfully
- WHEN `molten test gate check` emits a pass receipt
- THEN the receipt includes `explicit-capability-fixture` and `no-implicit-authority` checks in addition to capability context, grant, denial, authority binding, policy, admission, budget, actor-registry, effect-log, and replay checks

### Requirement: Examples use least-privilege grants
r[molten.testing.mandatory_capabilities.examples] Repository examples and positive harness tests MUST declare explicit least-privilege grants for the actions they expect to allow. Negative authority tests MUST use explicit empty fixtures or missing grants, not omitted fixtures.

#### Scenario: Two-actor example declares grants
r[molten.testing.mandatory_capabilities.examples.two_actor]
- GIVEN the repository two-actor example suite
- WHEN it is run through the harness and gated as pass evidence
- THEN the suite includes explicit grants for observe, assert, send, clock, random, and retract actions used by the test

### Requirement: Future Basalt/UCAN keeps explicit authority invariant
r[molten.testing.mandatory_capabilities.basalt_ucan_invariant] Future Basalt/UCAN authority proof integration MUST preserve the invariant that missing authority evidence fails closed. Proof bundles, caveats, revocation evidence, and authority receipts MAY replace local static grants, but they MUST be explicit and bound to run identity.

#### Scenario: Missing future proof fails closed
r[molten.testing.mandatory_capabilities.basalt_ucan_invariant.missing]
- GIVEN a future evidence-bearing report whose admission decision depends on a UCAN proof bundle
- WHEN the proof bundle or Basalt receipt ref is omitted
- THEN validation rejects the report rather than treating missing proof evidence as ambient authority
