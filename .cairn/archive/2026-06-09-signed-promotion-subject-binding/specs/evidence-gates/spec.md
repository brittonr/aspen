## ADDED Requirements

### Requirement: Signed promotion subject binding
r[molten.evidence.release_promotion.signed_subject_binding] Molten's dogfood release evidence MUST verify the signed release promotion receipt against the exact subject ref emitted by `molten dogfood release-promote`.

#### Scenario: Signed promotion subject matches emitted promotion receipt
- GIVEN a release promotion gate receipt emitted by dogfood release promotion
- AND a signed receipt envelope for release promotion
- WHEN signed promotion verification runs
- THEN verification requires the signed envelope subject ref to equal the emitted promotion receipt ref
- AND verification fails when the signed envelope subject ref differs

### Requirement: Signed promotion subject binding remains evidence only
r[molten.evidence.release_promotion.signed_subject_evidence_only] Signed promotion subject-ref binding MUST NOT grant authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation trust, release publication authority, or permission to bypass subsystem gates.

#### Scenario: Subject binding does not grant publication authority
- GIVEN a signed promotion receipt whose subject ref matches the emitted promotion receipt
- WHEN release publication or a subsystem operation requires authority
- THEN the matching subject binding MUST NOT be treated as sufficient authority
- AND the subsystem MUST still require its own gate evidence
