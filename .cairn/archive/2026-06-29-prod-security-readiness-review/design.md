## Context

The repo already has authority identities/contexts/revocations, Basalt/UCAN capability gates, provenance and build verification receipts, secret/encrypted-ref/redaction machinery, Octet source gates, plugin host deny-by-default hostcalls, Wasmtime/Steel executor preflights, and node-control live ticket workflows. Production readiness requires a cross-cutting security review that proves those rails are composed correctly.

## Design

### Threat model

Write a production threat model covering:

- node identity and key material;
- peer tickets, delegation, and authority grants;
- source-gate and release-evidence freshness;
- remote artifact install and job execution;
- plugin host/Wasmtime/Steel boundaries;
- secrets, encrypted refs, redaction, repro bundles, catalog/MCP views;
- retention/destructive operations;
- operator workflows and emergency stop.

Each threat should map to an existing gate or a new required test/receipt.

### Security drills

Security drills should emit receipts for:

- key revocation and live-ref cleanup;
- stale/compromised ticket denial;
- authority attenuation and expiry;
- secret redaction and reveal denial;
- provenance/build verification mismatch;
- source-gate tamper/stale denial;
- plugin/hostcall ambient authority denial;
- incident-response recovery steps.

### Fuzzing and negative suites

Add bounded fuzz or generated negative fixtures for Preserves parser boundaries, receipt validators, source-gate receipts, repro bundle verification/unpack, node-control ingress/workflow bundle validation, provenance/build records, and plugin hostcall requests.

### Review output

Emit a canonical `prod-security-review-receipt-v1` or equivalent report that binds threat-model refs, drill refs, fuzz/negative-suite refs, unresolved risk refs, and pilot-scope recommendations.

### Non-goals

- Do not claim a formal security proof.
- Do not bypass authority/provenance/source-gate decisions because a review receipt passed.
- Do not include private key material or plaintext secrets in review artifacts.
