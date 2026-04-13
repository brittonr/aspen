## MODIFIED Requirements

### Requirement: Tiger Style audits produce committed audit reports

A Tiger Style audit SHALL leave a committed, reviewable report in the repo.

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

### Requirement: Audit findings produce an executable remediation backlog

A Tiger Style audit SHALL translate high-severity findings into bounded implementation tasks.

#### Scenario: Hotspots are turned into completed remediation tasks

- **GIVEN** an audit identifies oversized functions, low-assertion hotspots, recursion, or raw-size API leaks
- **WHEN** the first remediation slice is completed
- **THEN** each checked task SHALL name the target files or functions
- **AND** each checked task SHALL cite saved evidence for its verification
- **AND** the completed slice SHALL remove the named hotspot targets from the saved targeted scan output

### Requirement: Audit parser regressions are captured as fixtures

Audit tooling rules that affect hotspot counts SHALL be backed by committed reproducer inputs and expected behavior.

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
