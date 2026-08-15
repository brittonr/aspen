## ADDED Requirements

### Requirement: Current clippy gate stays warning-free
r[molten.project.clippy_gate.current_warning_free] The repository SHOULD keep `cargo clippy --all-targets -- -D warnings` passing before proof-affecting replay, release, or production-readiness evidence is promoted.

#### Scenario: Clippy gate blocks evidence refresh
- GIVEN a candidate source tree with clippy diagnostics denied by the configured command
- WHEN replay or release evidence is being prepared for promotion
- THEN the quality gate must be fixed or explicitly recorded as blocking before refreshed evidence is treated as current.
