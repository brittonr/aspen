# Tasks: Type authority and artifact references

## Phase 1: Inventory and foundation

- [ ] [serial] I1 Inventory raw authority, capability, artifact, policy, resource, evidence, operation, and receipt references across core and wire boundaries. r[molten.authority.nominal_references.inventory]
- [ ] [serial] V1 Capture baseline canonical Preserves bytes, receipt refs, replay outputs, and focused authority decisions for each migration cohort. r[molten.authority.nominal_references.compatibility]
- [ ] [serial] I2 Add private domain-marked entity and canonical-reference families with checked constructors, explicit accessors, and typed diagnostics. r[molten.authority.nominal_references.types]
- [ ] [parallel] V2 Add positive and negative construction tests for bounds, spelling, domain tags, malformed refs, and unsupported algorithms. r[molten.authority.nominal_references.validation]
- [ ] [serial] I3 Add explicit Preserves wire-to-core adapters and closed heterogeneous reference enums where needed. r[molten.authority.nominal_references.wire_boundary]

## Phase 2: Authority and execution admission

- [ ] [serial] I4 Migrate authority contexts, delegation and revocation state, key refs, and capability proofsets to exact reference aliases. r[molten.authority.nominal_references.authority_core]
- [ ] [serial] I5 Migrate effect and handler admission, node-control requests, sessions, and operation/resource refs. r[molten.authority.nominal_references.execution_core]
- [ ] [parallel] V3 Add wrong-holder, wrong-session, wrong-policy, wrong-resource, expired, revoked, and possession-is-authority negative tests after each cohort. r[molten.authority.nominal_references.authority_tests]

## Phase 3: Artifact and evidence linkage

- [ ] [serial] I6 Migrate artifact binding, provenance, evidence, operation, and receipt linkage without absorbing binding or semantic-effect ownership. r[molten.authority.nominal_references.artifact_core]
- [ ] [serial] I7 Migrate retention, replay, cache, and historical receipt references while preserving evidence-only boundaries. r[molten.authority.nominal_references.evidence_core]
- [ ] [parallel] V4 Add compile-pass fixtures for same-domain flows and compile-fail fixtures for each maintained cross-domain substitution pair. r[molten.authority.nominal_references.compile_time]
- [ ] [parallel] V5 Prove canonical Preserves bytes, receipt refs, ledger import, and historical replay remain stable for accepted fixtures. r[molten.authority.nominal_references.compatibility]

## Phase 4: Enforcement and closeout

- [ ] [serial] I8 Add Molten domain declarations for future Octet nominal enforcement in migrated pure-core scopes. r[molten.authority.nominal_references.octet]
- [ ] [serial] I9 Document wire/core admission, domain aliases, concurrent change ownership, migration guidance, and authority non-claims. r[molten.authority.nominal_references.docs]
- [ ] [parallel] V6 Run focused authority, capability, node, effect, artifact, retention, replay, and provenance suites. r[molten.authority.nominal_references.final_checks]
- [ ] [serial] V7 Run nextest or focused Cargo fallback, Clippy, Octet, lifecycle validation, Cairn gates, and relevant Nix checks. r[molten.authority.nominal_references.final_checks]

## Verification coverage

- `Scenario: Reference inventory classifies every selected raw value` -> I1
- `Scenario: Same-domain reference is accepted` -> I2, V2, V4
- `Scenario: Cross-domain reference does not compile` -> I2, V4
- `Scenario: Preserves record admits typed core refs` -> I3, V2
- `Scenario: Wire domain mismatch fails` -> I3, V2
- `Scenario: Authority admission keeps exact reference roles` -> I4, V3
- `Scenario: Node and effect admission keep exact roles` -> I5, V3
- `Scenario: Artifact and evidence linkage stay distinct` -> I6, V4
- `Scenario: Historical replay remains evidence only` -> I7, V5
- `Scenario: Canonical Preserves bytes remain stable` -> V1, V5
- `Scenario: Octet blocks raw-domain regression` -> I8, V7
- `Scenario: Nominal references do not grant authority` -> I9, V6, V7
