## ADDED Requirements

### Requirement: Parse flake references with FlakeRef

All flake reference handling in CI payloads SHALL use `nix_compat::flakeref::FlakeRef` for parsing and validation instead of raw string manipulation.

#### Scenario: Parse GitHub flake reference

- **WHEN** a CI job payload contains `flake_url = "github:owner/repo"`
- **THEN** the parser SHALL produce a `FlakeRef::GitHub { owner: "owner", repo: "repo" }` with no rev or ref

#### Scenario: Parse flake ref with attribute

- **WHEN** a CI job specifies `flake_url = "github:owner/repo"` and `attribute = "packages.x86_64-linux.default"`
- **THEN** the system SHALL produce the canonical flake reference `github:owner/repo#packages.x86_64-linux.default`

#### Scenario: Parse local path flake reference

- **WHEN** a CI job payload contains `flake_url = "."`
- **THEN** the parser SHALL produce a `FlakeRef::Path` variant

#### Scenario: Reject invalid flake reference

- **WHEN** a CI job payload contains an unparseable flake URL
- **THEN** the parser SHALL return a validation error before any build is attempted

### Requirement: Normalize flake references for caching

The CI executor SHALL normalize flake references via `FlakeRef` to produce consistent cache keys regardless of input format variations.

#### Scenario: Equivalent refs produce same cache key

- **WHEN** two CI jobs reference `github:owner/repo` and `github:owner/repo?ref=main`
- **AND** both resolve to the same rev
- **THEN** the cache key derivation SHALL produce the same key for both

### Requirement: Extract rev/ref for build deduplication

The CI executor SHALL extract `rev` and `ref` fields from parsed `FlakeRef` to detect duplicate build requests.

#### Scenario: Deduplicate builds with same rev

- **WHEN** two CI jobs target the same flake with the same `rev`
- **THEN** the system SHALL detect the duplication and reuse the first build's result
