## ADDED Requirements

### Requirement: derivation_to_build_request is fully tested without subprocess

The `derivation_to_build_request` function SHALL be tested with zero subprocess calls for all code paths: basic derivation, derivation with inputs, environment variable merging, fixed-output derivation network access, placeholder replacement, output path construction, refscan needle generation, and builder path injection.

#### Scenario: Empty inputs produces valid BuildRequest

- **WHEN** `derivation_to_build_request` is called with a Derivation containing zero input_derivations and zero input_sources
- **THEN** the returned `BuildRequest` has correct `command_args`, empty `inputs` map, and valid `outputs`/`scratch_paths`/`working_dir`

#### Scenario: Placeholder replacement in arguments and environment

- **WHEN** a Derivation's arguments or environment contain `hashPlaceholder("out")` strings
- **THEN** `derivation_to_build_request` replaces them with the actual output store path

#### Scenario: FOD gets NetworkAccess constraint

- **WHEN** a Derivation has exactly one output named "out" with `ca_hash` set (fixed-output derivation)
- **THEN** the returned `BuildRequest` contains `BuildConstraints::NetworkAccess`

#### Scenario: Non-FOD does not get NetworkAccess

- **WHEN** a Derivation has a normal (non-fixed) output
- **THEN** the returned `BuildRequest` does NOT contain `BuildConstraints::NetworkAccess`

### Requirement: parse_closure_output handles all edge cases

The `parse_closure_output` function SHALL correctly parse `nix-store -qR` output and handle degenerate inputs without panicking.

#### Scenario: Standard multi-line output

- **WHEN** input is multiple lines of `/nix/store/<hash>-<name>` paths
- **THEN** returns deduplicated basenames in order, prefix stripped

#### Scenario: Empty input

- **WHEN** input is an empty string
- **THEN** returns an empty Vec

#### Scenario: Duplicate entries

- **WHEN** input contains the same store path on multiple lines
- **THEN** returns only one copy (first occurrence preserved)

#### Scenario: Lines without /nix/store/ prefix

- **WHEN** input contains lines that don't start with `/nix/store/`
- **THEN** those lines are silently skipped

### Requirement: output_sandbox_path_to_store_path extracts store paths correctly

The `output_sandbox_path_to_store_path` function SHALL extract the `/nix/store/<hash>-<name>` target from sandbox output paths.

#### Scenario: Typical sandbox path

- **WHEN** input is `/tmp/builds/<uuid>/scratches/nix/store/<hash>-<name>`
- **THEN** returns `Some(PathBuf::from("/nix/store/<hash>-<name>"))`

#### Scenario: No nix/store marker

- **WHEN** input path contains no `nix/store/` segment
- **THEN** returns `None`

#### Scenario: Empty basename after marker

- **WHEN** input path ends with `nix/store/` (trailing slash, no basename)
- **THEN** returns `None`

### Requirement: parse_derivation validates .drv ATerm format

The `parse_derivation` function SHALL parse valid ATerm-encoded derivation bytes and reject invalid ones.

#### Scenario: Valid minimal derivation

- **WHEN** valid ATerm bytes for a single-output derivation are provided
- **THEN** returns `Ok((Derivation, Vec<String>))` with correct output paths

#### Scenario: Invalid bytes

- **WHEN** arbitrary non-ATerm bytes are provided
- **THEN** returns `Err` with a descriptive error message

### Requirement: extract_drv_path_string validates format

The `extract_drv_path_string` function SHALL accept only strings starting with `/nix/store/` and ending with `.drv`.

#### Scenario: Valid drv path string in NixEvalOutput

- **WHEN** `NixEvalOutput.value` is `Some(Value::String("/nix/store/...-foo.drv"))`
- **THEN** returns `Ok("/nix/store/...-foo.drv")`

#### Scenario: Non-drv path

- **WHEN** value is a string that doesn't end with `.drv`
- **THEN** returns `Err` with "unexpected drvPath" message

#### Scenario: Non-string value

- **WHEN** value is `Some(Value::Integer(42))`
- **THEN** returns `Err` with message about expected string type

#### Scenario: No value

- **WHEN** value is `None`
- **THEN** returns `Err` with "no value" message

### Requirement: NixEvaluator pure eval works without subprocess

`NixEvaluator::evaluate_pure` SHALL evaluate Nix expressions using only snix-eval in-process, with no subprocess calls.

#### Scenario: Arithmetic expression

- **WHEN** `evaluate_pure("1 + 2")` is called
- **THEN** returns a value of `3` with no errors and no subprocess spawned

#### Scenario: Source too large

- **WHEN** source exceeds `MAX_EVAL_SOURCE_SIZE` (1MB)
- **THEN** returns an error immediately without attempting evaluation

#### Scenario: Syntax error

- **WHEN** source has a Nix syntax error
- **THEN** returns structured `NixEvalError` with non-empty message

### Requirement: Subprocess call sites documented with tracking comments

Every `Command::new("nix")` and `Command::new("nix-store")` call in `aspen-ci-executor-nix/src/` SHALL have a `// SUBPROCESS-ESCAPE: <description>` comment documenting what it does and what the pure-snix replacement path is.

#### Scenario: All subprocess calls annotated

- **WHEN** `rg 'Command::new\("nix' crates/aspen-ci-executor-nix/src/` is run
- **THEN** every match has a `SUBPROCESS-ESCAPE` comment within the preceding 3 lines
