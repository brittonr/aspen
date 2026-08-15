## ADDED Requirements

### Requirement: Replay evidence binds deterministic run identity
r[molten.determinism.replay_freshness.identity_binding] Replay verification receipts and replay indexes SHOULD bind the deterministic run identity they verify, including artifact ref, dependency closure ref, initial state ref, schema refs, policy refs, capability refs, handler profile ref, seed or effect-log ref, runtime/tool refs, and replay profile.

#### Scenario: Matching identity is accepted
- GIVEN replay verification evidence whose run identity matches the expected subsystem or release subject identity
- WHEN replay freshness validation runs
- THEN the freshness decision is `pass`
- AND the receipt records the matching run identity ref.

#### Scenario: Changed policy ref denies freshness
- GIVEN replay evidence recorded with a different policy ref than the expected subject identity
- WHEN freshness validation runs
- THEN the freshness decision is `deny`
- AND diagnostics identify the stale policy component.

### Requirement: Replay indexes preserve member identity bindings
r[molten.determinism.replay_freshness.index_binding] Replay indexes SHOULD preserve and summarize run identity refs from their member replay verification receipts, and MUST deny when a member's declared identity ref is malformed or stale for an expected subject.

#### Scenario: Index lists identity refs
- GIVEN a replay index built from identity-bound replay verification receipts
- WHEN the index is emitted
- THEN it records the unique run identity refs represented by the member receipts
- AND each identity ref is content-ref validated.

#### Scenario: Stale member denies index freshness
- GIVEN a replay index with one member receipt whose run identity differs from the expected subject identity
- WHEN index freshness validation runs
- THEN validation denies
- AND diagnostics identify the stale member receipt ref and mismatched identity component.

### Requirement: Replay freshness behavior is tested
r[molten.determinism.replay_freshness.tests] Molten SHOULD test matching identity acceptance and stale artifact, dependency closure, initial state, schema, policy, capability, handler profile, seed/effect-log, runtime, tool, and replay profile denial cases.

#### Scenario: Identity denial matrix identifies components
- GIVEN replay fixtures that each alter one deterministic identity component
- WHEN freshness validation evaluates them
- THEN each case denies with the expected stale component diagnostic
- AND none of the stale receipts can satisfy release-bound replay evidence.
