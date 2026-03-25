## ADDED Requirements

### Requirement: Spec markers in snix documentation

Normative requirements in snix-related documentation SHALL be marked with `r[snix.area.detail]` standalone paragraphs.

#### Scenario: Verus invariants have spec markers

- **WHEN** a Verus invariant is documented in `crates/aspen-snix/verus/lib.rs`
- **THEN** a corresponding `r[snix.verified.invariant-name]` marker SHALL exist in the spec docs

#### Scenario: Storage behavior has spec markers

- **WHEN** a normative storage behavior is documented (blob size limits, key format, digest length)
- **THEN** a corresponding `r[snix.store.detail]` marker SHALL exist

### Requirement: Implementation annotations in Rust source

Rust source files implementing a spec requirement SHALL carry `// r[impl snix.area.detail]` annotations.

#### Scenario: Blob service implementation annotated

- **WHEN** `blob_service.rs` implements blob size bounds
- **THEN** the enforcing code SHALL carry `// r[impl snix.store.blob-size-bound]`

#### Scenario: Directory service implementation annotated

- **WHEN** `directory_service.rs` enforces entry count limits
- **THEN** the enforcing code SHALL carry `// r[impl snix.store.directory-entry-bound]`

### Requirement: Verification annotations in tests

Test files verifying a spec requirement SHALL carry `// r[verify snix.area.detail]` annotations.

#### Scenario: Blob test annotated

- **WHEN** a test in `blob_service_test.rs` verifies blob size bounds
- **THEN** the test SHALL carry `// r[verify snix.store.blob-size-bound]`

### Requirement: CI enforcement

The CI pipeline SHALL run `tracey query validate` and fail on broken references or duplicate IDs.

#### Scenario: Broken reference fails CI

- **WHEN** a `// r[impl snix.foo]` annotation references a non-existent spec marker
- **THEN** `tracey query validate` SHALL report an error
- **AND** the CI check SHALL fail

### Requirement: Tracey configuration

A tracey config file SHALL define the `snix` spec with source includes for snix docs and implementation includes for snix Rust crates.

#### Scenario: Config covers snix crates

- **WHEN** tracey is configured
- **THEN** the config SHALL include `crates/aspen-snix/src/**/*.rs` in implementation sources
- **AND** the config SHALL include `crates/aspen-castore/src/**/*.rs` in implementation sources
- **AND** test files SHALL be listed in `test_include`
