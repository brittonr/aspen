## ADDED Requirements

### Requirement: Octet source-gate validation binds consumer scope coverage
r[molten.octet_gates.source_gate_scope_coverage] Molten MUST accept an Octet source-gate validation only when the bound strict Octet gate receipt includes object-corpus/fingerprint evidence for every configured source path in the consumer source scope.

#### Scenario: Missing source-scope coverage denies validation
- GIVEN a pass-shaped Octet gate receipt that lacks source-scope object-corpus coverage evidence
- WHEN source-gate validation runs for a consumer with a configured source scope
- THEN validation emits a deny receipt
- AND diagnostics identify missing object-corpus/fingerprint coverage for the required source scope

#### Scenario: Unsupported custom source scope denies validation
- GIVEN a source-gate validation request with a source path outside the configured source-gate inventory
- WHEN validation normalizes the requested source scope
- THEN validation denies before downstream side effects
- AND diagnostics identify that the source scope is outside configured source-gate coverage

### Requirement: Octet gate refs use canonical content-ref grammar
r[molten.octet_gates.canonical_gate_refs] Molten MUST reject Octet gate receipts and object-corpus fingerprints whose content refs do not match the canonical BLAKE3 hex grammar for the declared ref scheme.

#### Scenario: Malformed gate artifact ref denies validation
- GIVEN a pass-shaped Octet gate receipt whose fingerprint, object-corpus, command, status, summary, findings, or policy ref has a non-hex or wrong-length hash suffix
- WHEN source-gate validation runs
- THEN validation emits a deny receipt
- AND diagnostics identify malformed canonical artifact refs before accepting pass evidence

#### Scenario: Malformed object-set hash denies strict gate
- GIVEN clean Octet status and summary artifacts
- AND an object-corpus receipt with a malformed `object_set_hash`
- WHEN strict gate evaluation runs
- THEN the gate emits a deny receipt
- AND no downstream source-gate consumer may treat the object corpus as fingerprint-bound evidence
