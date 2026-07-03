# Node Runtime Delta: Production profile domain contracts

### Requirement: Production profile scalar fields use domain contracts
r[molten.prod_ops.profile_domain_contracts.scalar_types] The production node deployment profile MUST validate evidence refs, profile names, state roots, and state layout directory fields with domain-specific Nickel contracts before exporting profile JSON.

#### Scenario: Valid scalar domains export
- GIVEN a production node profile whose refs use the supported BLAKE3 content-ref syntax, whose profile name is non-empty, whose state root is absolute, and whose layout directories are safe relative directory names
- WHEN the operator exports the profile through Nickel
- THEN the export succeeds and preserves the reviewed profile field names and values

#### Scenario: Malformed evidence ref fails early
- GIVEN a production node profile containing a malformed, uppercase, empty, or non-BLAKE3 source-gate ref
- WHEN the operator exports the profile through Nickel
- THEN the export fails before any production readiness receipt can bind that profile

#### Scenario: Unsafe state path fails early
- GIVEN a production node profile whose state root is relative or whose layout directory is absolute, empty, current-directory, parent-directory, or path-traversal shaped
- WHEN the operator exports the profile through Nickel
- THEN the export fails with a contract diagnostic for the path field

### Requirement: Production resource limits are positive integers
r[molten.prod_ops.profile_domain_contracts.positive_limits] Production profile resource limits MUST be positive integer values at the Nickel contract boundary.

#### Scenario: Positive integer limits export
- GIVEN a production profile whose queue, receipt, store, delivery-latency, and recovery-time limits are positive integers
- WHEN the operator exports the profile
- THEN the exported JSON contains numeric limit values accepted by production readiness evidence generation

#### Scenario: Non-positive or fractional limit fails
- GIVEN a production profile with a zero, negative, fractional, or non-numeric resource limit
- WHEN the operator exports the profile through Nickel
- THEN the export fails before startup or production-readiness evidence can treat the limit as reviewed
