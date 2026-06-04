## ADDED Requirements

### Requirement: Capability context evidence is canonical
r[molten.testing.capability_context.fixture_schema] Evidence-bearing harness suites MUST represent local authority inputs as canonical capability context evidence or refs. The first deterministic fixture MAY be a static Preserves `<capabilities-v1 "molten.harness.capabilities.v1" [...]>` record, but report and gate validation MUST treat it as capability evidence rather than ambient test setup.

#### Scenario: Suite declares local grants
r[molten.testing.capability_context.fixture_schema.grants]
- GIVEN a harness suite that grants `producer` authority to send to `consumer` and request logical time
- WHEN the suite is parsed for an evidence-bearing run
- THEN the capability fixture is canonicalized, hashed, and included in run identity and report evidence

#### Scenario: Missing capability fixture is explicit
r[molten.testing.capability_context.fixture_schema.default]
- GIVEN a harness suite with no explicit capability fixture
- WHEN the harness normalizes the suite for an evidence-bearing profile
- THEN the default capability context is explicit, deterministic, and validated rather than inferred from actor names or prior ambient state

### Requirement: Admission requests are bound to authority context
r[molten.testing.capability_context.request_binding] Each admission request MUST be authorized against the embedded capability context before side effects. Admission evidence MUST be bound to the capability context ref and to the matching grant or denial reason used for authorization.

#### Scenario: Send request uses matching grant
r[molten.testing.capability_context.request_binding.send]
- GIVEN a suite step `<send "producer" "consumer" "hello">`
- AND a capability context granting `producer` `send` authority to `consumer`
- WHEN the step is admitted
- THEN the admission evidence identifies the capability context and the decision is reproducible from the matching grant

#### Scenario: Tampered authority binding fails validation
r[molten.testing.capability_context.request_binding.tampered]
- GIVEN a report whose recorded admission decision references an authority context or grant not matching the embedded suite capability fixture
- WHEN the report is validated or gated
- THEN validation rejects the report before accepting any committed trace or effect records

### Requirement: Authorization denies by default
r[molten.testing.capability_context.deny_by_default] The harness runtime MUST deny admission when no matching capability grant or proof authorizes the request. Missing capability evidence MUST NOT be treated as implicit authority.

#### Scenario: Send without grant is denied
r[molten.testing.capability_context.deny_by_default.send]
- GIVEN a suite step where `producer` sends to `consumer`
- AND the capability context contains no matching send grant
- WHEN the step executes
- THEN the runtime records an admission denial, rolls the turn back, and emits no message-delivered evidence

#### Scenario: Observe without grant is denied
r[molten.testing.capability_context.deny_by_default.observe]
- GIVEN a suite step where an actor observes a dataspace pattern
- AND the capability context contains no matching observe grant
- WHEN the step executes
- THEN the runtime records an admission denial and does not register the observer

### Requirement: Ambient effects require capability grants
r[molten.testing.capability_context.effect_authority] Effect-producing steps, including clock, random, storage, network, filesystem, process, blob, or adapter effects, MUST require explicit capability grants or proof evidence before any effect request or response record is emitted.

#### Scenario: Clock without grant has no effect request
r[molten.testing.capability_context.effect_authority.clock]
- GIVEN a suite step `<clock "producer">`
- AND no matching clock capability grant
- WHEN the step executes
- THEN admission denies the request, rollback evidence is recorded, and no effect request or effect response appears in the observation

#### Scenario: Tampered denied effect response fails validation
r[molten.testing.capability_context.effect_authority.tampered]
- GIVEN a report with a denied clock step due to missing authority
- WHEN the report also contains an effect request or effect response for that step
- THEN report validation rejects the report as crossing the effect boundary after denial

### Requirement: Capability authorization composes with static policy
r[molten.testing.capability_context.policy_composition] Capability authorization and static policy admission MUST compose fail-closed. A step is allowed only when both the authority context authorizes the request and the static policy layer allows it. A denial from either layer MUST prevent side effects and be represented in canonical admission evidence.

#### Scenario: Granted but policy-denied assertion is denied
r[molten.testing.capability_context.policy_composition.policy_denies]
- GIVEN a capability context granting `producer` authority to assert `service.ready`
- AND a static policy denying that same assertion
- WHEN the assertion step executes
- THEN the final admission decision is deny and validation can recompute that denial from both embedded authority and policy evidence

#### Scenario: Policy allows but capability missing is denied
r[molten.testing.capability_context.policy_composition.capability_denies]
- GIVEN no static deny rule for a send step
- AND no matching send capability grant
- WHEN the send step executes
- THEN the final admission decision is deny because authority is missing

### Requirement: Capability evidence is recomputed during validation
r[molten.testing.capability_context.validation] Report validation MUST recompute capability authorization from the embedded capability context and MUST reject missing, malformed, stale, unsupported, or tampered capability evidence.

#### Scenario: Stale capability ref fails validation
r[molten.testing.capability_context.validation.stale]
- GIVEN a report whose embedded capability fixture changed after execution without updating recorded authority evidence
- WHEN the report is validated or accepted by a pass-evidence gate
- THEN validation fails closed because capability refs no longer bind to the embedded suite

#### Scenario: Tampered grant fails validation
r[molten.testing.capability_context.validation.tampered_grant]
- GIVEN a report that records an allowed send decision
- WHEN the embedded capability context is tampered to remove the matching send grant
- THEN validation rejects the report rather than trusting the recorded allow decision

### Requirement: Capability divergence is first-class replay evidence
r[molten.testing.capability_context.capability_divergence] Deterministic replay SHOULD classify mismatches at the authority boundary as `capability-decision` divergence, or MUST include equivalent authority-mismatch diagnostics before downstream trace, effect, output, or state-hash differences are reported.

#### Scenario: Replay stops at changed capability decision
r[molten.testing.capability_context.capability_divergence.changed]
- GIVEN a recorded report with an admission decision allowed by capability evidence
- WHEN replay recomputes a missing-grant denial for the same step
- THEN replay fails at the authority boundary with capability-decision or authority-mismatch diagnostics for the first divergent step

### Requirement: Gate receipts include capability checks
r[molten.testing.capability_context.gate_receipts] Successful pass-evidence gate receipts MUST include checks for capability context presence, grant/proof validation, deny-without-capability behavior, and authority-ref binding. Receipts MUST include artifact refs for the capability context and any future Basalt/UCAN proof bundle used for authorization.

#### Scenario: Gate receipt lists capability checks
r[molten.testing.capability_context.gate_receipts.checks]
- GIVEN a deterministic harness report that validates and replays successfully with capability evidence
- WHEN `molten test gate check` emits a pass receipt
- THEN the receipt includes passed checks named `capability-context`, `capability-grants`, `deny-without-capability`, and `authority-ref-binding`

### Requirement: Basalt/UCAN replacement seam is preserved
r[molten.testing.capability_context.basalt_ucan_path] The local static grant fixture MUST be structured so Basalt/UCAN proof refs, caveats, attenuation chains, revocation evidence, and authority receipts can replace or augment it without removing fail-closed capability validation.

#### Scenario: Future UCAN proof cannot bypass evidence
r[molten.testing.capability_context.basalt_ucan_path.future_ucan]
- GIVEN a future report whose admission decision depends on UCAN proof evidence
- WHEN the proof ref, caveat evidence, revocation check, or authority receipt is missing or stale
- THEN validation fails closed rather than treating the action as authorized
