# Testing Harness Delta: repro reveal encrypted-ref binding

### Requirement: Reveal receipts bind encrypted repro refs directly
r[molten.testing.repro_reveal_encrypted_ref_binding.receipt_field] Reveal receipts used for encrypted-private repro unpack MUST carry an explicit encrypted-ref binding and a corresponding binding check.

#### Scenario: Bound reveal receipt is accepted
- GIVEN an encrypted-private repro bundle with an encrypted ref
- WHEN unpack receives a passing reveal receipt bound to that exact encrypted ref
- THEN unpack may materialize the authorized private repro evidence

#### Scenario: Legacy generic reveal receipt is not enough for repro unpack
- GIVEN an encrypted-private repro bundle and a passing legacy reveal receipt with no encrypted-ref binding
- WHEN unpack runs with that reveal receipt
- THEN unpack fails closed before materializing private content

### Requirement: Unpack matches exact bundle encrypted refs
r[molten.testing.repro_reveal_encrypted_ref_binding.unpack_match] Repro unpack MUST authorize encrypted-private material only by exact encrypted-ref ids present in the bundle.

#### Scenario: Stale reveal binding is rejected
- GIVEN a reveal receipt whose secret or commitment ref matches a bundle encrypted ref but whose encrypted-ref field names another ref
- WHEN encrypted-private repro unpack runs
- THEN unpack fails closed with a stale or unrelated reveal diagnostic

### Requirement: Reveal coverage remains complete and evidence-only
r[molten.testing.repro_reveal_encrypted_ref_binding.partial_coverage_denial] Repro unpack MUST fail closed unless every encrypted ref in the bundle has a passing exact-bound reveal receipt.

#### Scenario: Partial reveal coverage is rejected
- GIVEN an encrypted-private repro bundle with one or more encrypted refs
- WHEN any encrypted ref lacks a passing exact-bound reveal receipt
- THEN unpack fails closed before writing private material

r[molten.testing.repro_reveal_encrypted_ref_binding.evidence_only] Reveal receipt bindings MUST NOT make encrypted-private repro bundles gate-preserving pass evidence.

#### Scenario: Reveal does not grant pass-gate evidence
- GIVEN an encrypted-private repro bundle with complete reveal receipts
- WHEN pass gate verification evaluates the bundle
- THEN the bundle remains requires-reveal private evidence and is not accepted as gate-preserving pass evidence
