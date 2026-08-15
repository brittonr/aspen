# Node Runtime Delta: Production profile schema metadata

### Requirement: Production profile exports carry schema metadata
r[molten.prod_ops.profile_schema_metadata.root_identity] Production deployment profile exports MUST include explicit schema identity, schema version, source language, and stable profile identity metadata.

#### Scenario: Metadata identifies reviewed profile export
- GIVEN a production profile exported from the reviewed Nickel contract boundary
- WHEN the exported JSON is inspected or bound into evidence
- THEN it includes metadata naming the production profile schema, schema version, source language, and profile identity

#### Scenario: Missing metadata fails validation
- GIVEN an exported profile JSON document without required schema or source-language metadata
- WHEN deployment-profile or startup validation evaluates it
- THEN validation fails before accepting the profile as production evidence

### Requirement: Profile metadata is bound into receipts
r[molten.prod_ops.profile_schema_metadata.receipt_binding] Deployment-profile and startup receipts MUST bind production profile metadata together with the profile content ref and MUST reject stale, unsupported, or tampered metadata bindings.

#### Scenario: Receipt binds matching metadata
- GIVEN a production profile export with supported metadata and a matching content ref
- WHEN deployment-profile evidence is generated
- THEN the receipt records the schema, version, source language, profile identity, and profile ref consistently

#### Scenario: Tampered metadata denies
- GIVEN a profile receipt whose schema, version, source language, profile identity, or profile ref no longer matches the exported profile under review
- WHEN validation runs
- THEN validation denies the profile evidence before startup can rely on it

### Requirement: Profile metadata is evidence-only
r[molten.prod_ops.profile_schema_metadata.evidence_only] Production profile metadata MUST NOT grant authority, source-gate acceptance, adapter readiness, provenance trust, resource sufficiency, retention clearance, or live transport correctness.

#### Scenario: Metadata does not replace subsystem gates
- GIVEN a profile export with valid metadata
- WHEN a subsystem requires authority, source-gate, adapter, resource, retention, or transport evidence
- THEN that subsystem still requires its own matching gate receipts and MUST NOT rely on metadata alone
