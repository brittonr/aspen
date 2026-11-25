# Testing harness deltas

## ADDED Requirements

### Requirement: Traceability coverage requires real code and test markers rather than prose or uncompiled stubs
r[molten.testing.traceability_code_backed_coverage] `tools/tracey/inherited_debt_guard.rs` MUST count requirement coverage only when real compiled source and test files under `src/`, `crates/`, or `tests/` supply `impl` and `verify` requirement markers. Non-code files (`docs/`, `flake.nix`, `tools/`), `depends`/`related` markers, and uncompiled comment-only stub files MUST NOT discharge requirement coverage.

#### Scenario: Markdown or flake comment cannot discharge requirement debt
- GIVEN an accepted requirement referenced only via a `related` marker in `docs/` or a shell comment in `flake.nix`
- WHEN `inherited_debt_guard` evaluates coverage
- THEN the requirement MUST remain classified as uncovered rather than covered.

#### Scenario: Uncompiled comment-only stub files are prohibited
- GIVEN no compiled test or implementation backs a requirement
- WHEN the repository traceability audit runs
- THEN uncompiled `tools/tracey/*_proof.rs` stub files MUST NOT exist to satisfy coverage.
