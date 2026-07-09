# Project Delta: CLI Imperative Shell

### Requirement: CLI modules stay thin shells
r[molten.modularity.cli_shell.thin_shell] CLI modules SHOULD limit themselves to argument parsing, path resolution, file IO, adapter orchestration, stdout/stderr rendering, process-exit behavior, and conversion between user-facing diagnostics and structured domain results.

#### Scenario: CLI handler delegates decision
- GIVEN a CLI command that performs domain validation, planning, or admission decisions
- WHEN the command is migrated for modularity
- THEN the handler converts parsed arguments into typed input, calls a library command core, and performs only the shell effects required by the structured result

#### Scenario: Domain decision in CLI is flagged
- GIVEN a CLI module contains deterministic domain decision logic that can be evaluated from in-memory inputs
- WHEN reviewers inspect a modularity change touching that module
- THEN the logic is moved to a library core or an explicit staged-migration exemption is recorded

### Requirement: Command cores are typed and testable
r[molten.modularity.cli_shell.typed_core] Extracted command cores MUST be callable without Clap parsing, filesystem state, stdout, stderr, process exits, network services, or live adapter execution.

#### Scenario: Valid command input succeeds in memory
- GIVEN a typed command-core input representing a valid command request
- WHEN a unit test calls the command core
- THEN it returns structured success, planned operations, receipts, or diagnostics without invoking the CLI binary

#### Scenario: Invalid command input fails in memory
- GIVEN malformed paths, missing evidence refs, stale refs, unsupported options, contradictory flags, or denied domain inputs represented in memory
- WHEN a unit test calls the command core
- THEN it returns a structured error or denial without writing files, printing output, or exiting the process

### Requirement: CLI modularity preserves UX contracts
r[molten.modularity.cli_shell.compatible_ux] CLI shell refactors MUST preserve existing command names, flags, canonical artifact outputs, and documented behavior unless a separate UX change owns the compatibility break.

#### Scenario: Existing command still works
- GIVEN a documented CLI command covered by the refactor
- WHEN the command is run with previously valid inputs
- THEN it accepts the same flags and emits equivalent canonical artifacts or documented diagnostics

### Requirement: CLI core extraction carries positive and negative tests
r[molten.modularity.cli_shell.tests] CLI core extraction SHOULD include positive tests for valid command inputs and negative tests for malformed, missing, stale, unsupported, or denied inputs.

#### Scenario: CLI core test matrix covers denial
- GIVEN a command core controls admission, artifact generation, or mutation planning
- WHEN reviewers inspect the tests
- THEN at least one positive path and at least one denial or malformed-input path are covered
