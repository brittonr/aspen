## ADDED Requirements

### Requirement: Production threat model maps to gates
r[molten.prod_security.threat_model] Molten MUST maintain a production threat model that names protected assets, principals, trust boundaries, attack scenarios, existing gates, required drills, and unresolved risks for node identity, peer tickets, authority delegation, source-gate evidence, release evidence, remote artifacts, plugin/hostcall boundaries, secrets, retention, and operator workflows.

#### Scenario: Threat has a gate or unresolved-risk record
- GIVEN a production threat-model entry
- WHEN the security review is evaluated
- THEN the entry maps to a concrete gate, drill, negative test, or explicit unresolved-risk record with pilot-scope consequences.

### Requirement: Key and revocation drills
r[molten.prod_security.key_and_revocation_drills] Molten MUST provide security drills for key revocation, delegation expiry, authority attenuation, live-ref cleanup, stale ticket denial, and compromised peer evidence, and MUST emit canonical receipts for pass and denial paths.

#### Scenario: Revoked authority cannot operate
- GIVEN an authority context or live ticket whose key or delegation has been revoked
- WHEN a privileged node-control, artifact install, job execution, or secret reveal operation is attempted
- THEN Molten denies before side effects and records revocation diagnostics.

### Requirement: Boundary fuzzing and negative suites
r[molten.prod_security.boundary_fuzzing] Molten SHOULD run bounded fuzzing or generated negative suites for Preserves parsers, canonical receipt validators, source-gate receipts, repro bundle verification and unpack, node-control ingress/workflow bundles, provenance/build records, plugin hostcalls, and live transport envelopes.

#### Scenario: Malformed receipt does not pass as missing evidence
- GIVEN malformed or adversarial receipt bytes at a production trust boundary
- WHEN the validator evaluates the bytes
- THEN it emits a structured denial or failure artifact and MUST NOT treat parser failure as absent clean evidence.

### Requirement: Secrets and redaction audit
r[molten.prod_security.secrets_redaction_audit] Molten MUST audit logs, CLI summaries, catalog views, MCP responses, repro bundles, release exports, failure diagnostics, transcript rendering, and reveal workflows to ensure secret material and private refs are redacted, encrypted, or gated by matching reveal evidence.

#### Scenario: Secret marker in export is denied or redacted
- GIVEN an export path encounters a private secret, credential, confidential marker, or encrypted ref
- WHEN production security review evaluates the export
- THEN the review requires a redaction/encryption/reveal receipt and denies unredacted plaintext exposure.

### Requirement: Supply-chain security review
r[molten.prod_security.supply_chain_review] Molten MUST bind source-gate evidence, reproducible build verification, provenance admission receipts, signed release evidence, keyring/revocation state, and release bundle verification into production security review for sensitive artifacts.

#### Scenario: Sensitive artifact has stale provenance
- GIVEN a sensitive production artifact with stale, missing, or mismatched provenance or build verification evidence
- WHEN security review or production admission evaluates the artifact
- THEN the decision denies before install or execution side effects.

### Requirement: Incident-response drill evidence
r[molten.prod_security.incident_response_drill] Molten SHOULD provide incident-response drills for compromised key, leaked peer ticket, stale source gate, bad release evidence, secret exposure, and emergency node stop, with canonical receipts and operator next-step diagnostics.

#### Scenario: Leaked ticket is contained
- GIVEN an operator marks a live peer ticket or authority grant as compromised
- WHEN the incident-response drill runs
- THEN Molten records revocation or deny evidence, cleanup actions, affected workflow refs, and recovery next steps without exposing private material.
