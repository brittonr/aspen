# Testing Harness Delta: Harness Schema and Gate Modularity

### Requirement: Harness responsibilities are layered
r[molten.testing.modularity.harness_layers] Harness implementation SHOULD separate schema models, pure gate decisions, fixture builders, canonical receipt construction, and IO or CLI shells.

#### Scenario: Harness module ownership is clear
- GIVEN harness schema or gate code is reorganized
- WHEN reviewers inspect the module layout
- THEN each module has an identifiable responsibility such as schema, decision, fixtures, receipts, or shell

### Requirement: Gate decisions are pure
r[molten.testing.modularity.pure_gate_decisions] Harness gate decisions SHOULD be deterministic functions over typed suite, report, policy, and evidence inputs, without filesystem reads, CLI rendering, process execution, or adapter IO.

#### Scenario: Valid report passes in memory
- GIVEN a valid suite/report input represented in memory
- WHEN the gate decision core evaluates it
- THEN it returns a pass decision and structured receipt input without reading files or running commands

#### Scenario: Malformed report denies in memory
- GIVEN a malformed, stale, unsupported, or contradictory suite/report input represented in memory
- WHEN the gate decision core evaluates it
- THEN it returns a deny or diagnostic result without writing evidence or invoking the CLI shell

### Requirement: Runtime code consumes harness evidence, not harness orchestration
r[molten.testing.modularity.runtime_boundary] Runtime modules MUST NOT depend on harness runners or release-test orchestration to make normal runtime decisions; they MAY consume canonical gate receipts or evidence summaries as explicit inputs.

#### Scenario: Runtime consumes receipt summary
- GIVEN runtime admission depends on prior harness evidence
- WHEN the runtime core evaluates admission
- THEN it consumes a canonical receipt or typed evidence summary rather than invoking harness suite execution

### Requirement: Harness modularity has positive and negative fixtures
r[molten.testing.modularity.fixtures] Harness schema or gate refactors SHOULD include positive fixtures for valid inputs and negative fixtures for malformed, stale, unsupported, or contradictory inputs.

#### Scenario: Fixture matrix covers gate behavior
- GIVEN a harness gate boundary is extracted
- WHEN focused validation runs
- THEN valid fixtures pass and negative fixtures fail for the expected invariant class
