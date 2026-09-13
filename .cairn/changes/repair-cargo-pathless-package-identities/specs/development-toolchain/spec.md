## ADDED Requirements

### Requirement: Explicit source identities round-trip without fabricated paths
r[molten.toolchain.pathless_identity]

Molten's selected Cargo MUST format pathless source URLs with explicit package names without a panic.
Its package-ID parser MUST accept valid explicit name/version fragments without requiring an unrelated URL path.
It MUST preserve the source URL, Git reference, package name, and version.

#### Scenario: Locked Radicle dependency identity
- GIVEN a pathless Radicle source URL and an explicit package name and version
- WHEN Cargo formats and parses the package identity
- THEN the identity remains equal and the operation does not panic

#### Scenario: Missing inferred package name
- GIVEN a pathless source URL without an explicit package name
- WHEN Cargo parses the package identity
- THEN it rejects the missing name rather than inventing one

### Requirement: Existing validation and toolchain boundaries remain intact
r[molten.toolchain.pathless_boundary]

The repair MUST preserve ordinary URL round-trips and rejection of malformed package names, versions, protocols, and queries.
Molten MUST consume a pinned Cargo source and a reviewable patch through Nix.
The repair MUST NOT change producer revisions, transports, global tools, or the selected Rust compiler.

#### Scenario: Malformed explicit identity
- GIVEN an invalid version or forbidden query in a package identity
- WHEN the repaired parser evaluates the identity
- THEN it rejects the input under the applicable validation rule

#### Scenario: Repository toolchain composition
- GIVEN the existing compiler cohort and the pinned Cargo patch
- WHEN the repository Nix environment selects its tools
- THEN it selects the repaired Cargo and retains the original compiler and target libraries

### Requirement: Actual consumers provide completion evidence
r[molten.toolchain.pathless_acceptance]

The repair MUST provide current locked Molten metadata and nextest results through the selected repository toolchain.
An isolated library probe or rewritten metadata MUST NOT substitute for those results.

#### Scenario: Real metadata and test discovery
- GIVEN the unchanged producer revisions and the repaired toolchain
- WHEN Cargo produces all-feature metadata and nextest discovers and runs the selected tests
- THEN metadata preserves the dependency identities and nextest does not fail on their serialization
