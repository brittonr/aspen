# Artifact Auth Adoption Specification

## MODIFIED Requirements

### Requirement: Molten adopts one immutable reviewed source

r[molten.artifact_auth_adoption.source] Molten MUST consume its authentication and binding packages from one immutable reviewed `OnixResearch/onix-artifact` revision. Cargo, Nix, policy, plan, workspace, license, and consumer identities MUST agree.

#### Scenario: Unified source identity is admissible

- GIVEN Cargo and Nix resolve revision `5f3962d7471d1547e044f57daeda72f56621fd52`, including Valence's transitive authentication dependencies
- WHEN Molten evaluates dependency admission
- THEN it SHALL accept the two authentication packages and binding package while rejecting floating, duplicate, mixed, local, widened, or incompatible sources.

#### Scenario: A transitive dependency introduces another cohort

- GIVEN a direct or transitive authentication package resolves a different source revision
- WHEN Molten evaluates dependency admission
- THEN it SHALL deny the mixed graph without ignoring duplicate identities or weakening exact lock admission.
