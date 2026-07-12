# Filesystem Materialization Specification

## Purpose

Defines the `filesystem-materialization` capability.

## Requirements

### Requirement: Filesystem materialization is planned in a pure core
r[molten.filesystem_materialization.plan] Molten MUST validate multi-file materialization as a deterministic in-memory plan before output mutation. The plan MUST bind a reviewed profile, bounded logical relative member paths, member kinds, expected BLAKE3 refs and sizes, duplicate and reserved-name checks, replacement policy, deterministic order, and a BLAKE3 plan identity.

#### Scenario: Valid member plan is deterministic
- GIVEN the same admitted logical members and materialization profile in different input order
- WHEN the pure planner runs
- THEN it MUST emit the same canonical member order, diagnostics, and plan identity.

#### Scenario: Unsafe logical member denies before output
- GIVEN a member is empty, absolute, parent-relative, platform-prefixed, separator-ambiguous, reserved, duplicated after normalization, or over a configured bound
- WHEN the pure planner runs
- THEN it MUST deny before any destination filesystem operation.

### Requirement: Materialization writes through an explicit capability root
r[molten.filesystem_materialization.root] Molten MUST open or create an explicitly requested destination in the outer shell and MUST perform all descendant directory creation, file creation, readback, rename, and cleanup through a typed materialization capability root. Semantic cores and member producers MUST NOT receive ambient descendant paths.

#### Scenario: Admitted output stays under destination
- GIVEN a passing materialization plan and an opened destination capability
- WHEN the shell writes and verifies its members
- THEN every descendant operation MUST be relative to that capability
- AND logical member names MUST NOT trigger ambient root reacquisition.

#### Scenario: Pre-existing symlink cannot redirect output
- GIVEN a destination contains a symlinked parent or leaf that would resolve outside the root
- WHEN materialization attempts to create, replace, or verify that member
- THEN the operation MUST deny before out-of-root data is read, written, renamed, or removed.

### Requirement: Publication is staged and fail-closed
r[molten.filesystem_materialization.commit] Molten MUST stage a multi-file plan beneath the destination capability, verify staged member refs and closure, and apply an explicit no-replace or reviewed-replace publication policy before emitting a passing materialization receipt. A failed, stale, or partial plan MUST NOT be represented as published success.

#### Scenario: Fully verified stage publishes
- GIVEN every staged member matches the active plan and publication policy permits the destination state
- WHEN commit runs
- THEN the shell MUST publish using in-root operations and emit a receipt bound to the verified plan and members.

#### Scenario: Mid-write failure does not mint success
- GIVEN a member write, hash verification, or publication step fails after some staging effects
- WHEN the operation terminates
- THEN no passing publication receipt MUST be emitted
- AND partial staging MUST be cleaned or explicitly quarantined through the destination capability.

### Requirement: Archive member names follow the materialization path policy
r[molten.filesystem_materialization.archive_members] Molten MUST normalize and validate archive member names with the same logical relative-path policy used for materialization. Archive readers and writers MUST reject absolute paths, parent components, platform prefixes, separator ambiguity, duplicate normalized names, links, devices, unsupported entry kinds, and configured count or byte bound violations; they MUST NOT use generic archive unpack for admitted output.

#### Scenario: Fixed regular archive members verify
- GIVEN an archive contains bounded regular files with unique admitted logical names
- WHEN archive verification runs
- THEN member refs and names MUST be evaluated deterministically without extracting through ambient paths.

#### Scenario: Link or traversal member denies
- GIVEN an archive contains a symlink, hard link, special entry, absolute path, parent traversal, or duplicate normalized name
- WHEN archive verification or materialization runs
- THEN the operation MUST deny before any member is written to a destination.

### Requirement: Materialization receipts are portable and bounded
r[molten.filesystem_materialization.receipt] Molten MUST emit canonical materialization evidence that binds schema, profile, plan identity, logical member paths, content refs, bounded count and byte summaries, replacement decision, diagnostics, and explicit non-claims. Absolute source, destination, staging, temporary, checkout, and store paths MUST remain display-only and MUST NOT determine canonical receipt identity.

#### Scenario: Equivalent roots produce equivalent evidence
- GIVEN the same logical plan and bytes are materialized beneath two different host roots
- WHEN passing receipts are constructed
- THEN their canonical semantic identity MUST be independent of the host roots.

#### Scenario: Containment is not release authority
- GIVEN a bundle was contained and its member hashes match
- WHEN evidence is rendered
- THEN it MUST NOT claim authenticity, signature validity, policy authority, confidentiality, release eligibility, or crash-atomic persistence from containment alone.

### Requirement: Converted materializers have a scoped regression gate
r[molten.filesystem_materialization.regression_gate] Molten MUST maintain a syntax-aware blocking gate for converted materialization modules that rejects ambient descendant writes, ambient directory scans, and generic archive unpack. The gate MUST permit reviewed top-level source or destination acquisition in outer shells and MUST include positive and negative fixtures.

#### Scenario: Ambient output join fails validation
- GIVEN a converted materializer joins an output root and member name before calling `std::fs`
- WHEN the structural gate runs
- THEN the gate MUST fail with a materialization-authority diagnostic.

#### Scenario: Explicit top-level destination bootstrap passes
- GIVEN a reviewed CLI shell opens the operator-selected destination and delegates a capability root
- WHEN the structural gate runs
- THEN the bootstrap fixture MUST pass without allowing ambient descendant operations.

### Requirement: Materialization validation covers success and failure
r[molten.filesystem_materialization.validation] Molten MUST include positive tests for repro, retention, release-directory, and archive workflows and negative tests for traversal, prefixes, duplicate names, symlink parents and leaves, wrong roots, special entries, stale plans, tampered sources, partial writes, and replacement-policy denial.

#### Scenario: Complete boundary suite passes
- GIVEN the shared planner and capability shell are integrated into all targeted materializers
- WHEN focused positive, negative, and structural tests run
- THEN valid bounded workflows MUST pass and every declared unsafe or stale workflow MUST deny before unauthorized output mutation.

#### Scenario: Failure class lacks coverage
- GIVEN happy-path round trips pass but a declared path, archive, or partial-commit failure class has no executable fixture
- WHEN the change is evaluated for archive
- THEN closeout MUST remain blocked with the uncovered class identified.
