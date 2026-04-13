## ADDED Requirements

### Requirement: Tiger Style audits produce committed audit reports

A Tiger Style audit SHALL leave a committed, reviewable report in the repo.

#### Scenario: Audit-only change remains audit-only

- **GIVEN** a contributor runs a Tiger Style audit and does not modify production source files
- **WHEN** the change is proposed or reviewed
- **THEN** the repo SHALL contain a committed audit report for that audit
- **AND** the report SHALL state that the work is audit-only
- **AND** the report SHALL list the scan scope, methodology, known limitations, and ranked hotspots
- **AND** no checked task or completion summary SHALL claim that remediation landed

#### Scenario: Audit refresh compares against the last committed baseline

- **GIVEN** a previous Tiger Style audit report already exists in the repo
- **WHEN** a contributor prepares a fresh Tiger Style audit change
- **THEN** the report SHALL compare current hotspot counts against the last committed baseline scan
- **AND** it SHALL call out newly added production hotspots separately from long-standing backlog
- **AND** it SHALL identify known scanner noise or parse errors that materially affect prioritization

#### Scenario: Remediation status is explicit

- **GIVEN** a Tiger Style change includes both audit artifacts and source refactors
- **WHEN** a reviewer reads the report
- **THEN** the report SHALL distinguish the backlog-input scan from the completed remediation slice
- **AND** it SHALL reference the production source files changed in that slice
- **AND** it SHALL identify the saved verification artifacts for the completed slice

### Requirement: Tiger Style audit tooling is reproducible

The repo SHALL provide a rerunnable Tiger Style scanner for the documented audit workflow.

#### Scenario: Reviewer reruns the scanner

- **GIVEN** a reviewer has the repo checkout
- **WHEN** they run the documented Tiger Style scanner command
- **THEN** the command SHALL emit a machine-readable hotspot inventory
- **AND** the inventory SHALL include enough detail to identify the file, function, and rule that triggered each hotspot

### Requirement: Audit parser regressions are captured as fixtures

Audit tooling rules that affect hotspot counts SHALL be backed by committed reproducer inputs and expected behavior.

#### Scenario: Trait declaration false positive is prevented

- **GIVEN** a Rust source fixture containing trait method declarations that end with `;` and later impl bodies that contain `{}`
- **WHEN** the Tiger Style scanner counts function bodies
- **THEN** it SHALL ignore the declaration-only trait methods
- **AND** it SHALL only count functions whose signatures reach a body `{` before a top-level `;`

#### Scenario: Lifetimes do not parse as char literals

- **GIVEN** a Rust source file containing lifetimes like `<'_>` and ordinary function bodies
- **WHEN** the Tiger Style scanner parses function bodies
- **THEN** it SHALL not treat the lifetime marker as the start of a char literal
- **AND** it SHALL not report a bogus unterminated-body parse error for that function

#### Scenario: Comments do not trigger ambient-time hotspots

- **GIVEN** a Rust source file whose doc comments mention ambient-time APIs such as `Instant::now()`
- **WHEN** the Tiger Style scanner evaluates `verified_ambient_time`
- **THEN** comment-only mentions SHALL not produce a hotspot
- **AND** fixture-backed tests SHALL cover that regression

#### Scenario: Engineering notes cite repo evidence

- **GIVEN** a contributor adds a repo-facing engineering note derived from an audit-tooling failure
- **WHEN** that note describes a scanner heuristic or bug
- **THEN** the change SHALL include a tracked reproducer or transcript supporting the note
- **AND** the audit report SHALL reference that evidence path

### Requirement: Audit findings produce an executable remediation backlog

A Tiger Style audit SHALL translate high-severity findings into bounded implementation tasks.

#### Scenario: Hotspots are turned into tasks

- **GIVEN** an audit identifies oversized functions, low-assertion hotspots, recursion, or raw-size API leaks
- **WHEN** the change is prepared for implementation
- **THEN** the task list SHALL group work into bounded phases
- **AND** each remediation task SHALL name the target files or functions
- **AND** each remediation task SHALL identify the expected verification command or saved evidence

#### Scenario: First remediation slice touches production code

- **GIVEN** a Tiger Style remediation change moves beyond audit-only status
- **WHEN** the first implementation phase is completed
- **THEN** at least one production hotspot SHALL be refactored in tracked source files
- **AND** the change SHALL include characterization or regression coverage for that hotspot
- **AND** the audit report SHALL be updated to show which backlog items remain open

#### Scenario: Hotspots are turned into completed remediation tasks

- **GIVEN** an audit identifies oversized functions, low-assertion hotspots, recursion, or raw-size API leaks
- **WHEN** the first remediation slice is completed
- **THEN** each checked task SHALL name the target files or functions
- **AND** each checked task SHALL cite saved evidence for its verification
- **AND** the completed slice SHALL remove the named hotspot targets from the saved targeted scan output
