## ADDED Requirements

### Requirement: derivation_to_build_request never panics on arbitrary input

Property-based tests SHALL verify that `derivation_to_build_request` does not panic for any combination of valid `Derivation` and `BTreeMap<StorePath, Node>` inputs.

#### Scenario: Arbitrary derivation with random outputs

- **WHEN** proptest generates a `Derivation` with 1-5 outputs, 0-10 input sources, arbitrary environment variables, and a random system string
- **THEN** `derivation_to_build_request` returns `Ok(BuildRequest)` or `Err` — never panics

#### Scenario: Output count bounded

- **WHEN** a `Derivation` has N outputs
- **THEN** the returned `BuildRequest::outputs` has exactly N entries

#### Scenario: Refscan needles include all inputs and outputs

- **WHEN** a `Derivation` has K outputs and M resolved inputs
- **THEN** `BuildRequest::refscan_needles` contains at least K + M entries (one nixbase32 hash per path)

### Requirement: parse_closure_output never panics on arbitrary input

Property-based tests SHALL verify that `parse_closure_output` does not panic for any `String` input.

#### Scenario: Arbitrary string input

- **WHEN** proptest generates an arbitrary `String` (including empty, whitespace-only, binary-like, multi-megabyte)
- **THEN** `parse_closure_output` returns a `Vec<String>` without panicking

#### Scenario: Output is always deduplicated

- **WHEN** proptest generates input with repeated lines
- **THEN** the output Vec contains no duplicates

### Requirement: NixBuildPayload validation covers edge cases

Property-based tests SHALL verify that `NixBuildPayload::validate()` returns `Err` for invalid inputs and `Ok` for well-formed inputs without panicking.

#### Scenario: Empty flake_url always rejected

- **WHEN** proptest generates a payload with `flake_url = ""`
- **THEN** `validate()` returns `Err`

#### Scenario: Excessive timeout always rejected

- **WHEN** proptest generates a payload with `timeout_secs > 86400`
- **THEN** `validate()` returns `Err`

#### Scenario: Valid payloads accepted

- **WHEN** proptest generates a payload with non-empty `flake_url` and `timeout_secs` in `[1, 86400]`
- **THEN** `validate()` returns `Ok(())`
