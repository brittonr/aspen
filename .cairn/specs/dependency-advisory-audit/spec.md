# Dependency Advisory Audit Specification

## Purpose

Defines the `dependency-advisory-audit` capability.

## Requirements

### Requirement: Release advisory observations bind exact selected closure

r[molten.dependency_audit.resolved] The default release audit MUST run a checksum-pinned maintained scanner against an explicit immutable RustSec database and MUST retain bounded raw machine output, package/version/source findings, paths, SPDX expressions and notice bytes, exact source/build/lock/target/default-feature bindings and a compact summary. Missing metadata, unknown source, stale scanner/data/binding and unavailable notices MUST deny or be explicitly unavailable. Existing release source and owned-license guards MUST remain prerequisites; audit evidence MUST NOT grant release, legal or runtime authority.

#### Scenario: Real scanner and immutable observation

- GIVEN an explicit maintained RustSec revision and pinned scanner
- WHEN the default target profile is captured offline
- THEN the report binds actual raw scan and input bytes and does not accept a supplied empty advisory list as a scan.

### Requirement: Negative controls and narrowly reviewed exceptions

r[molten.dependency_audit.controls] The audit MUST detect affected packages, source-policy violations, unavailable licenses, expired exceptions and stale bindings. An advisory exception MUST name exact package/version/source/advisory, owner, rationale, review identity and expiry; expired or invalid review MUST deny.

#### Scenario: Missing evidence never becomes clean

- GIVEN missing license metadata or stale required input
- WHEN the audit is evaluated
- THEN the machine report explicitly denies and identifies the unavailable or stale input.

### Requirement: Available advisory fixes are refreshed before any exception

r[molten.dependency_audit.registry_refresh] A dependency audit remediation MUST refresh every resolved registry crate whose
advisory names a patched version the current requirement already allows, MUST
regenerate `Cargo.lock` and the unit2nix plans together, and MUST name each
advisory that still remains together with the upstream move it needs. It MUST
NOT add an advisory exception in place of an available registry fix.

#### Scenario: Registry fix is available

- GIVEN an advisory whose patched range intersects the pinned requirement
- WHEN remediation runs
- THEN the crate is updated, both plans are regenerated, and the audit is re-run
  against the same pinned database revision

#### Scenario: Fix requires an upstream move

- GIVEN an advisory whose patched version is semver-incompatible with a pinned
  dependency
- WHEN the remediation record is written
- THEN the advisory, its dependency chain and the upstream owner are recorded,
  and no exception is added

### Requirement: A transitive fix is adopted with its whole pin surface [r[molten.dependency_audit.pin_surface]]

When a fix for a transitive advisory requires moving a pinned Git source, the
change MUST move every manifest, Nix input, release-profile revision, focused
workspace, notice and document that carries the revision in one change, MUST
re-resolve the lock without drifting unrelated package versions, and MUST re-run
the pinned audit to record the before and after.

#### Scenario: Partial repin

- GIVEN only some of the manifests or the flake input carry the new revision
- WHEN the audit runs
- THEN the release guard reports duplicate identity or Nix drift and denies

#### Scenario: Complete repin

- GIVEN every carrier of the revision moved together and the lock re-resolved
- WHEN the audit runs against the same pinned database revision
- THEN every prerequisite passes and the advisory set shrinks by the fixed entry

### Requirement: Root features are reviewed as an exact dev-only set

r[molten.dependency_audit.dev_feature_review] The audit MUST review the root
feature set that `cargo metadata` reports for the workspace root as an exact
allowlist covering features enabled only by the crate's own dev-dependencies,
and MUST deny any other root feature set.

#### Scenario: Reviewed dev-only features

- GIVEN the resolve reports exactly the features the crate's own dev-dependency enables
- WHEN the audit evaluates the closure
- THEN the root feature set is accepted and no `profile-binding` finding is raised

#### Scenario: Any other root feature

- GIVEN the resolve reports a changed default, an optional union, or any feature beyond the reviewed dev-only set
- WHEN the audit evaluates the closure
- THEN the closure is denied with `profile-binding`
