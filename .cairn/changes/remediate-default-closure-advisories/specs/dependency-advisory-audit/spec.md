# Dependency Advisory Remediation Delta

## ADDED Requirements

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
