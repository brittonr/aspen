# Dev Feature Review Delta

## ADDED Requirements

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
