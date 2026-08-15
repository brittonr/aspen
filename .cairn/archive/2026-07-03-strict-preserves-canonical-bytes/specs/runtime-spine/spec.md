## ADDED Requirements

### Requirement: Canonical Preserves byte decoding is strict
r[molten.preserves_canonical_bytes.strict_decode] Molten MUST reject packed Preserves byte streams at canonical byte boundaries unless parsing and re-encoding produce byte-identical canonical bytes.

#### Scenario: Canonical bytes pass strict decode
- GIVEN packed Preserves bytes produced by Molten canonical encoding
- WHEN a ledger, transport, storage, or executor boundary decodes those bytes
- THEN strict decode succeeds
- AND the decoded value ref matches the canonical ref of the original value.

### Requirement: Non-canonical Preserves bytes deny before side effects
r[molten.preserves_canonical_bytes.noncanonical_denial] Molten MUST fail closed when parseable packed Preserves bytes are not byte-identical to their canonical re-encoding.

#### Scenario: Alternate packed encoding is rejected
- GIVEN packed Preserves bytes that parse to a value but do not match that value's canonical encoding
- WHEN the bytes enter a trust boundary
- THEN the boundary decision is `deny` or returns a structured invalid-input error
- AND no import, enqueue, execution, dispatch, or persistence side effect is admitted.

### Requirement: Trust boundaries use strict canonical decoding
r[molten.preserves_canonical_bytes.trust_boundaries] Molten MUST use strict canonical decoding for externally supplied packed Preserves bytes in ledger, chunk store, typed storage, remote transport, node ingress, Iroh exchange, and Wasm executor paths.

#### Scenario: Boundary records strict decode evidence
- GIVEN invalid external packed Preserves bytes
- WHEN a boundary rejects the bytes
- THEN diagnostics identify the boundary and strict canonical decode failure
- AND rendered logs remain non-normative compared with the canonical denial evidence.
