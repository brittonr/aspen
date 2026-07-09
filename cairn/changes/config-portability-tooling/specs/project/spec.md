# Project

## ADDED Requirements

### Requirement: Repository config paths are relocatable

r[molten.project.config_portability.relocatable_paths] Repository-owned development, hook, Nix, and validation configuration SHOULD avoid user-specific absolute paths and MUST allow required sibling repository paths to be supplied by reviewed workspace-relative defaults, flake inputs, or explicit environment variables.

#### Scenario: Common workspace checkout works without user-specific paths

- GIVEN Molten is checked out under a normal OnixResearch sibling workspace
- WHEN development hooks, Nix checks, or config validation resolve Cairn and private dependency source paths
- THEN they resolve through reviewed defaults or explicit environment variables
- AND reviewed config does not require a `/home/<user>/...` literal to run.

#### Scenario: User-specific path is rejected by config lint

- GIVEN a repo-owned config file introduces a hard-coded user home path for a required tool or sibling repository
- WHEN the config lint check runs in release-review mode
- THEN the check fails with a diagnostic naming the file and portability rule.

### Requirement: Release toolchains are pinned

r[molten.project.config_portability.toolchain_pin] Rust toolchain configuration used for release, CI, Nix checks, or canonical evidence SHOULD pin an exact toolchain identity and MUST NOT rely on a floating channel unless that channel is explicitly scoped to local exploratory use and excluded from release evidence.

#### Scenario: Pinned release toolchain passes

- GIVEN the release and Nix check toolchain is a dated Rust channel or exact toolchain identity
- WHEN config validation inspects the toolchain source
- THEN validation records the pinned identity as release-review evidence.

#### Scenario: Floating release nightly fails

- GIVEN release-scoped config uses a floating `nightly` Rust channel with no exact date or toolchain identity
- WHEN config validation runs in release-review mode
- THEN validation fails before formatter, Clippy, unit2nix, or test evidence can be treated as reproducible release evidence.

### Requirement: Cargo and Nix private source pins stay aligned

r[molten.project.config_portability.git_source_pin_drift] Molten SHOULD provide a deterministic check that compares private OnixResearch git dependency revisions in `Cargo.lock` with the Nix local-source map used for hermetic unit2nix builds.

#### Scenario: Matching source pins pass

- GIVEN Cargo.lock names private dependency revisions that match the Nix local-source map
- WHEN the source-pin drift check runs
- THEN it passes and reports the dependency names and revisions that were compared.

#### Scenario: Mismatched source pin fails

- GIVEN Cargo.lock names a private dependency revision that differs from the Nix local-source map
- WHEN the source-pin drift check runs
- THEN it fails closed with diagnostics naming the dependency, Cargo revision, and Nix revision.

### Requirement: Config lint is pure-core and shell-owned

r[molten.project.config_portability.config_lint] Config lint decisions SHOULD be computed by a deterministic pure core over explicit file records, while the shell owns filesystem discovery, environment lookup, command execution, and rendered diagnostics.

#### Scenario: Pure config lint accepts explicit inputs

- GIVEN in-memory config records with paths, toolchain channels, source pins, and profile refs
- WHEN the config lint core evaluates them
- THEN it returns pass or denial diagnostics without reading files, executing commands, consulting environment variables, or rendering stdout.

#### Scenario: Shell reports denied config

- GIVEN the shell reads repo config files and the pure core returns a denial
- WHEN the config lint command renders the result
- THEN it names the denied rule and source file while keeping runtime authority, policy, provenance, and source-gate decisions out of scope.

### Requirement: Repeated config values are named

r[molten.project.config_portability.named_config_constants] Long-lived Nix and test configuration SHOULD express VM addresses, attempt bounds, event limits, timeout values, profile names, and evidence-output paths through named constants or small modules when those values are part of reviewed behavior.

#### Scenario: Named config constant is review-visible

- GIVEN a VM address, retry bound, event limit, timeout, or evidence profile changes
- WHEN reviewers inspect the diff
- THEN the changed value is associated with a name describing its role rather than appearing only as an unexplained numeric or string literal.

#### Scenario: Refactor preserves check behavior

- GIVEN a Nix check is refactored to use named constants
- WHEN the check runs with the same semantic values
- THEN canonical evidence outputs and pass/deny behavior remain unchanged or the change records an explicit evidence migration note.
