# Runtime Spine Delta: Delivery Idempotency UX

### Requirement: CLI emits canonical delivery scope and operation records
r[molten.delivery_idempotency_ux.spec.cli_scope_operation] The delivery idempotency CLI MUST emit canonical Preserves scope profile and operation id records using the same hashing and validation rules as the runtime delivery idempotency module.

#### Scenario: Operation id is materialized
- GIVEN a scope profile/name, producer, consumer, sequence, intent, payload ref, and policy refs
- WHEN an operator runs `molten test delivery operation-id`
- THEN Molten emits an `operation-id-v1` artifact
- AND the printed operation ref is the canonical ref of that artifact

### Requirement: CLI checks produce replay-bound idempotency receipts
r[molten.delivery_idempotency_ux.spec.cli_check_receipts] The delivery idempotency CLI MUST run checks against an explicit local store root and MUST emit the same idempotency receipt records used by runtime ingress paths.

#### Scenario: Duplicate delivery is suppressed
- GIVEN a delivery check for a scoped sequence has already emitted a `first` receipt
- WHEN the same scope, producer, consumer, sequence, payload, policy, and evidence are checked again
- THEN Molten emits a `duplicate` idempotency receipt
- AND the duplicate receipt binds the prior first receipt
- AND the side effect is marked `suppress`

### Requirement: Delivery CLI receipts are evidence only
r[molten.delivery_idempotency_ux.spec.evidence_only] Delivery idempotency CLI receipts MUST NOT grant transport, authority, provenance, policy, resource, or execution trust.

#### Scenario: Evidence-only receipt
- GIVEN a pass delivery idempotency receipt
- WHEN another subsystem needs authority or provenance admission
- THEN it must still resolve explicit authority, policy, resource, provenance, and transport evidence
- AND it must not treat the delivery idempotency receipt as a grant
