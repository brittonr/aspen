# Evidence Gates Delta: Contract export drift gates

### Requirement: Contract export drift gates compare source, export, schema, and Rust admission
r[molten.evidence.contract_export_drift.source_export_rust_alignment] Contract export drift gates MUST verify that reviewed Nickel source exports match checked-in generated JSON or Preserves artifacts, that Preserves boundary schema identity and arity remain compatible, and that Rust admission accepts valid exports while rejecting negative exports.

#### Scenario: Checked export matches source and parser
- GIVEN a reviewed Nickel contract source, its checked-in generated artifact, the relevant Preserves boundary schema, and Rust admission parser coverage
- WHEN the drift gate runs
- THEN it confirms the generated artifact matches the source export and the Rust parser admits it as valid evidence

#### Scenario: Stale generated artifact fails
- GIVEN a Nickel contract source changed without refreshing its checked-in generated JSON or Preserves artifact
- WHEN the drift gate runs
- THEN validation fails before the stale artifact can be promoted as current evidence

### Requirement: Contract export drift gates are deterministic and local
r[molten.evidence.contract_export_drift.local_deterministic_gate] Contract export drift gates MUST run deterministically from source-controlled fixtures without live network access, production credentials, mutable runtime state, or runtime Nickel authority.

#### Scenario: Local drift check runs in CI
- GIVEN a checkout with contract sources, fixtures, generated artifacts, and Rust tests
- WHEN the CI or release-review drift gate runs
- THEN it produces deterministic pass or fail evidence using only source-controlled inputs and local toolchains
