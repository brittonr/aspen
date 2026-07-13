# Project Specification

## Purpose

Makes Aspen/Molten release dependencies reproducible, selects canonical standalone Valence, and records an AGPL-allowed distribution profile.

## Requirements

### Requirement: Release dependency source rows are typed
r[molten.project.reproducible_dependencies.contract] Molten MUST normalize release dependency declarations from repository-owned manifests, Cargo lock data, and Nix source inputs into typed rows containing package identity, source kind, source coordinate, immutable revision, transport policy, and release disposition.

#### Scenario: Complete source row validates
r[molten.project.reproducible_dependencies.fixtures.positive]
- GIVEN a release dependency has matching package, source, immutable revision, and transport policy across reviewed inputs
- WHEN dependency validation runs
- THEN the row MUST pass and preserve its exact source identity.

#### Scenario: Malformed source row fails
r[molten.project.reproducible_dependencies.fixtures.negative]
- GIVEN a row is missing a source coordinate or immutable revision, uses an unsupported source kind, or has conflicting package identity
- WHEN dependency validation runs
- THEN validation MUST fail with a deterministic source-row diagnostic.

### Requirement: Git dependencies use immutable release pins
r[molten.project.reproducible_dependencies.exact_pins] Every Git dependency in the Molten release closure MUST be bound to an immutable revision in the repository-owned dependency source of truth, and manifest, lockfile, and Nix source identities MUST agree.

#### Scenario: Floating Git source fails release validation
- GIVEN a release dependency is bound only to a branch, tag, moving reference, or unpinned SSH URL
- WHEN release dependency validation runs
- THEN validation MUST fail before the dependency contributes to release evidence.

#### Scenario: Lock drift fails release validation
- GIVEN the manifest or Nix source pin names one immutable revision and the Cargo lockfile resolves another
- WHEN release dependency validation runs
- THEN validation MUST fail with a revision-drift diagnostic naming both identities.

### Requirement: Dependency drift validation has a pure core
r[molten.project.reproducible_dependencies.drift_validation] Molten MUST compare normalized dependency rows in pure deterministic logic while filesystem reads, manifest parsing, lockfile loading, and Nix evaluation remain in shell or adapter code.

#### Scenario: Equivalent row ordering is stable
- GIVEN equivalent dependency rows arrive in different input order
- WHEN drift validation runs
- THEN diagnostics and receipt identity MUST remain deterministic.

### Requirement: Canonical standalone Valence is required
r[molten.project.reproducible_dependencies.canonical_valence] Molten MUST consume `valence-core` semantics from the exact standalone Valence revision accepted by the archived Valence integrity-hardening and Octet cutover receipts.

#### Scenario: Standalone canonical source passes
- GIVEN Aspen's dependency row matches the accepted standalone Valence source and revision
- WHEN dependency validation runs
- THEN the Valence source contribution MUST pass.

#### Scenario: Octet-hosted Valence is rejected
- GIVEN Aspen resolves `valence-core` from Octet's hosted or legacy compatibility package
- WHEN dependency validation runs
- THEN validation MUST fail with a non-canonical-source diagnostic.

### Requirement: Canonical Valence package identity is unique
r[molten.project.reproducible_dependencies.unique_valence_identity] Molten MUST reject a resolved dependency graph containing different source identities under the same canonical `valence-core` package name and version.

#### Scenario: Duplicate semantic providers fail
- GIVEN standalone and Octet-hosted Valence implementations both resolve under canonical package identity
- WHEN package-graph validation runs
- THEN validation MUST fail and identify each source.

### Requirement: Cross-repository cutover evidence is required
r[molten.project.reproducible_dependencies.cross_repo_dependencies] Molten MUST require archived receipts for Valence integrity hardening and Octet standalone cutover before accepting the canonical Valence migration as release evidence.

#### Scenario: Missing upstream receipt blocks migration
- GIVEN either required upstream archive receipt is absent or mismatched
- WHEN Aspen evaluates migration readiness
- THEN the canonical Valence migration MUST remain blocked.

### Requirement: AGPL is an allowed typed distribution profile
r[molten.project.agpl_distribution_profile.contract] Molten MAY use an AGPL distribution profile, and the profile MUST record the selected license identity, notice artifacts, source coordinate, immutable revision, and project-required corresponding-source or source-export evidence.

#### Scenario: Complete AGPL profile passes project policy
- GIVEN a release declares AGPL and supplies the configured notice and source evidence
- WHEN distribution-profile validation runs
- THEN the profile MUST pass without treating AGPL itself as a blocker.

#### Scenario: Missing configured evidence fails
- GIVEN an AGPL distribution profile omits a required notice, source coordinate, immutable revision, or configured source-export artifact
- WHEN distribution-profile validation runs
- THEN validation MUST fail with a deterministic missing-evidence diagnostic.

### Requirement: Distribution boundary is documented
r[molten.project.agpl_distribution_profile.docs] Molten documentation MUST state that AGPL is an accepted project choice and that distribution-profile evidence records project-policy facts rather than providing legal advice or proving compliance in every jurisdiction.

#### Scenario: License choice is not misclassified
- GIVEN a reviewer reads release documentation
- WHEN AGPL distribution is described
- THEN the documentation MUST distinguish accepted license choice from missing project-required release evidence.

### Requirement: Reproducible dependency verification rail
r[molten.project.reproducible_dependencies.final_validation] The change MUST include positive and negative evidence for exact source pins, lock agreement, canonical Valence selection, unique package identity, and the AGPL distribution profile.

#### Scenario: Drift and missing evidence fail closed
- GIVEN floating, drifting, duplicate, non-canonical, or incomplete fixtures
- WHEN focused validation runs
- THEN every invalid fixture MUST fail while complete exact-pin fixtures pass.

### Requirement: Flake-check CI is checked in
r[molten.project.reproducible_dependencies.flake_check_ci] Molten MUST include a checked-in CI workflow for this remediation whose verification command is `nix flake check`.

#### Scenario: CI uses the scoped verification rail
- GIVEN a change is evaluated by checked-in CI
- WHEN the remediation workflow runs
- THEN it MUST execute `nix flake check` without requiring a separate expanded CI command matrix in this change.
