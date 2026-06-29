## Phase 1: Threat model and review receipt

- [x] [serial] r[molten.prod_security.threat_model] Write the production threat model and map every named threat to an existing gate, required drill, or explicit unresolved risk.
- [x] [serial] r[molten.prod_security.supply_chain_review] Bind release/source/provenance/build verification evidence into the security review and deny stale or mismatched sensitive-artifact evidence.

## Phase 2: Drills

- [x] [parallel] r[molten.prod_security.key_and_revocation_drills] Add key, delegation, revocation, live-ref cleanup, stale ticket, and authority attenuation drills with canonical receipts.
- [x] [parallel] r[molten.prod_security.secrets_redaction_audit] Audit redaction across logs, summaries, catalogs, MCP, repro bundles, exports, failure diagnostics, and reveal workflows.
- [x] [parallel] r[molten.prod_security.incident_response_drill] Add incident-response drills for compromised key, leaked ticket, stale source gate, bad release evidence, and emergency stop.

## Phase 3: Negative and fuzz suites

- [x] [serial] r[molten.prod_security.boundary_fuzzing] Add bounded fuzz/generated negative coverage for Preserves parsers, receipt validators, source-gate receipts, repro bundles, node-control ingress, provenance records, and plugin hostcalls.
- [x] [serial] r[molten.prod_security.threat_model] Emit a canonical security-readiness report summarizing pass/deny status, unresolved risks, and pilot-scope recommendations.
