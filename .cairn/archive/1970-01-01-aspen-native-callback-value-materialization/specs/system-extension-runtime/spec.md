# System Extension Runtime Specification Delta

## ADDED Requirements

### Requirement: Native callbacks receive exact materialized values

r[molten.system_extension.native_host.value_materialization] The native host MUST supply bounded, identity-checked bytes for every callback payload, prior state, effect completion, and recovery checkpoint required by the selected callback.

#### Scenario: Materialized callback values match their references

- GIVEN the value port returns bytes for every required callback reference
- WHEN callback framing runs
- THEN the host MUST verify each BLAKE3 identity and byte bound
- AND the child MUST receive the verified bytes in the canonical v2 envelope.

#### Scenario: A required value is missing or corrupt

- GIVEN a payload, state, completion, or checkpoint reference has no exact bounded bytes
- WHEN callback preparation runs
- THEN the callback MUST fail before process start
- AND the host MUST NOT use a reference-only fallback.

### Requirement: Native callback results publish exact values

r[molten.system_extension.native_host.value_publication] The native host MUST admit and publish bounded output, effect-request, next-state, and checkpoint bytes before their references become visible to host or provider semantics.

#### Scenario: Returned values publish successfully

- GIVEN the child returns canonical reference-and-byte values
- WHEN callback result admission runs
- THEN every reference MUST match its bytes
- AND publication MUST complete before state replacement or provider routing.

#### Scenario: Returned bytes are absent or substituted

- GIVEN the child returns a reference without bytes or bytes with another identity
- WHEN result admission runs
- THEN the callback MUST fail closed
- AND no state, checkpoint, output, or provider effect may become visible.

### Requirement: Value effects have durable intent and uncertainty

r[molten.system_extension.native_host.value_intent] The native host MUST persist callback intent before materialization and publication intent before value publication.

#### Scenario: Publication fails before acceptance

- GIVEN publication intent committed and the value port rejects before accepting bytes
- WHEN the host records the result
- THEN the publication operation MUST become terminal
- AND semantic state MUST remain unchanged.

#### Scenario: Publication acceptance is uncertain

- GIVEN publication may have accepted bytes but no definitive result is available
- WHEN recovery inventory runs
- THEN the publication operation MUST remain unknown
- AND the host MUST NOT republish or route dependent effects automatically.

### Requirement: Semantic state survives restart as materialized content

r[molten.system_extension.native_host.semantic_state] The durable native instance MUST track latest semantic state separately from lifecycle checkpoint state and MUST recover both by exact content identity.

#### Scenario: Request updates semantic state

- GIVEN a callback publishes valid next-state bytes
- WHEN callback completion commits
- THEN the instance MUST store the new state reference
- AND the next callback MUST receive those exact bytes.

#### Scenario: Restart observes unresolved value work

- GIVEN a restart finds unresolved materialization or publication operations
- WHEN native recovery classification runs
- THEN it MUST preserve their exact identity and uncertainty
- AND normal ingress MUST remain blocked until explicit reconciliation.

### Requirement: Native protocol v2 is exact and non-fallback

r[molten.system_extension.native_host.value_protocol] A materializing native host profile MUST use only the exact v2 envelope, outcome, ALPN, and framing cohort.

#### Scenario: Version two is selected

- GIVEN the executable and host profile select the same v2 cohort
- WHEN install and callback admission run
- THEN the host MUST use the v2 value protocol for every callback.

#### Scenario: Legacy or mixed protocol is supplied

- GIVEN any schema, ALPN, framing, executable cohort, or value requirement selects v1 or a mixed version
- WHEN admission runs
- THEN installation or callback admission MUST fail without fallback.

### Requirement: Materialization conformance includes negative paths

r[molten.system_extension.native_host.value_validation] Conformance MUST test exact bytes, process separation, restart, missing values, corrupt values, bounds, legacy framing, publication rejection, publication uncertainty, and blocked dependent effects.

#### Scenario: Separate-process materialization passes

- GIVEN a conforming external executable and exact v2 profile
- WHEN ingress, callback, state publication, effect publication, checkpoint, restart, and recovery run
- THEN parent-observed identities and durable operation ordering MUST pass.

#### Scenario: Negative evidence is absent

- GIVEN required identity, bound, legacy, rejection, uncertainty, or restart tests are absent
- WHEN closeout runs
- THEN the change MUST remain incomplete.
