## ADDED Requirements

### Requirement: Compute input closure via PathInfoService references

The system SHALL compute the transitive input closure of a derivation by recursively walking the `references` field in PathInfoService entries, replacing `nix-store -qR` subprocess calls.

#### Scenario: Complete closure from PathInfoService

- **WHEN** all transitive references for a derivation's inputs exist in PathInfoService
- **THEN** the system returns the full closure as a deduplicated list of store path names, matching the output of `nix-store -qR`

#### Scenario: Self-referencing paths

- **WHEN** a store path lists itself in its references
- **THEN** the system includes the path once and does not loop

#### Scenario: Empty references

- **WHEN** a store path has no references (leaf dependency)
- **THEN** the system includes only the path itself

### Requirement: Closure walk resource bounds

The system SHALL enforce Tiger Style resource bounds during closure traversal.

#### Scenario: Closure exceeds path limit

- **WHEN** the BFS traversal discovers more than `MAX_CLOSURE_PATHS` (50,000) unique paths
- **THEN** the system stops traversal and returns an error with the count of paths discovered so far

#### Scenario: Cycle detection

- **WHEN** the reference graph contains cycles (pathological data)
- **THEN** the BFS visited set prevents infinite traversal and the system returns normally

### Requirement: Fallback to nix-store -qR

The system SHALL fall back to `nix-store -qR` subprocess when PathInfoService cannot resolve all transitive references, but only when the `nix-cli-fallback` feature is enabled.

#### Scenario: Partial resolution triggers fallback

- **WHEN** PathInfoService resolves some but not all references AND `nix-cli-fallback` is enabled
- **THEN** the system falls back to `nix-store -qR` for the original input paths and returns the subprocess result

#### Scenario: Partial resolution without fallback

- **WHEN** PathInfoService cannot resolve all references AND `nix-cli-fallback` is disabled
- **THEN** the system returns the partially-resolved closure with a warning listing unresolved paths, and the caller decides whether to proceed or error
