## ADDED Requirements

### Requirement: Octet CLI shell dispatch is split without semantic drift
r[molten.octet_source_remediated_zero.cli_octet_shell_split] Molten SHOULD split Octet CLI command parsing and dispatch out of the monolithic `src/main.rs` shell into a focused CLI module while preserving command syntax, receipt labels, stdout/stderr behavior, denial behavior, and canonical Preserves output values.

#### Scenario: Octet command module preserves operator commands
- GIVEN an operator runs an existing `molten test octet` subcommand
- WHEN the command is dispatched through the split CLI module
- THEN the command accepts the same flags and delegates to the same Octet gate, baseline, artifact import, review, source-gate, or remediation core helper as before.

### Requirement: CLI split validation prevents semantic drift
r[molten.octet_source_remediated_zero.no_cli_semantic_drift] Molten MUST validate CLI shell splits with focused Rust checks that compile the Clap command graph and command dispatch before the split can be counted as source-remediated-zero progress.

#### Scenario: Split command graph still compiles
- GIVEN an Octet CLI command group has moved to a focused module
- WHEN Rust validation runs
- THEN the top-level `TestCommand::Octet` variant, nested subcommands, and dispatch function compile without changing public command names.

### Requirement: Source-remediated-zero evidence is refreshed
r[molten.octet_source_remediated_zero.evidence_refresh] Molten MUST refresh Octet workspace/lib artifacts, object-corpus or fingerprint evidence for changed critical paths, strict gate receipts, remediation-plan receipts, and release dogfood evidence before claiming a source-remediated-zero checkpoint.

#### Scenario: Changed CLI path refreshes source-gate evidence
- GIVEN a CLI source file involved in Octet evidence changes
- WHEN Molten claims a source-gate checkpoint for the change
- THEN the Octet artifact import, strict gate, remediation plan, and release dogfood evidence reflect the changed source scope.

### Requirement: Disabled lint family burn-down remains explicit
r[molten.octet_source_remediated_zero.disabled_lint_burndown] Molten MUST keep disabled Octet lint families visible as configuration-clean caveats until source splits and code-shape changes allow those families to be removed or narrowed without hiding findings.

#### Scenario: Configuration-clean is not source-remediated zero
- GIVEN a broad Octet lint family remains disabled in `dylint.toml`
- WHEN remediation evidence is reported
- THEN Molten labels the result as configuration-clean with a remaining burn-down item rather than source-remediated zero for that family.

### Requirement: Remaining Octet burn-down is delegated by category
r[molten.octet_source_remediated_zero.categorized_followups] Molten SHOULD split unfinished no-disabled Octet burn-down work into focused Cairn follow-up packages so completed source-shape foundation work can sync and archive without claiming source-remediated zero for unfinished lint families.

#### Scenario: Categorized follow-ups preserve caveats
- GIVEN the foundation Octet source-shape change has completed its scoped refactors
- AND no-disabled Octet evidence still reports warning-only categories
- WHEN the foundation change is prepared for archive
- THEN active follow-up Cairn packages track import hygiene, path shape, size shape, source-scope/tooling, and safety polish separately
- AND remediation docs continue to label the overall state as configuration-clean rather than source-remediated zero.
