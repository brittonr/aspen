## ADDED Requirements

### Requirement: Operator gateway readback core
r[molten.operator_gateway.readback_core] Molten MUST define a pure operator-gateway readback decision core that normalizes requested object refs, optional collection members, byte ranges, requester context, visibility policy refs, and supporting evidence refs before any HTTP, Iroh, filesystem, or response-streaming shell performs I/O.

#### Scenario: Readback request normalizes before I/O
- GIVEN an operator gateway request for a canonical artifact, receipt, bundle member, or chunk manifest range
- WHEN the readback decision core evaluates the request
- THEN it returns a pass, deny, or degraded decision with normalized refs, range, required checks, and diagnostics
- AND the imperative shell performs no response I/O until the decision is available.

#### Scenario: Malformed ref denies before lookup
- GIVEN an operator gateway request with a malformed or non-canonical object ref
- WHEN the readback decision core evaluates the request
- THEN it returns a deny decision with malformed-ref diagnostics
- AND catalog, chunk-store, Iroh, or filesystem lookup is skipped.

### Requirement: Read-only operator gateway index
r[molten.operator_gateway.readonly_index] Molten SHOULD provide read-only operator gateway indexes for visible artifact bundles, chunk collections, release evidence bundles, retention review bundles, and receipt sets without granting mutation authority.

#### Scenario: Visible bundle index is rendered
- GIVEN an operator gateway index request with policy-admitted visibility over a bundle
- WHEN Molten renders the index
- THEN it includes only visible member names, refs, sizes, and optional MIME hints
- AND the index receipt binds the request, visibility policy refs, response ref, and read-only checks.

#### Scenario: Hidden member is redacted or omitted
- GIVEN a bundle with a member hidden by confidentiality, retention, redaction, or visibility policy
- WHEN Molten renders the gateway index
- THEN the hidden member ref and sensitive name are omitted or redacted
- AND diagnostics record the omission without leaking the hidden ref.

### Requirement: Operator gateway receipts are evidence-only
r[molten.operator_gateway.receipts] Molten MUST emit canonical readback receipts for gateway read, range, and index operations, and MUST NOT treat those receipts as authority, policy admission, provenance trust, source-gate acceptance, retention clearance, execution permission, or mutation rights.

#### Scenario: Gateway receipt cannot authorize mutation
- GIVEN a passing gateway readback receipt
- WHEN a caller attempts to use it as evidence for delete, pin, unpin, install, execute, or policy mutation
- THEN the downstream gate denies unless the normal authority, policy, retention, provenance, source-gate, and resource evidence is supplied independently.
