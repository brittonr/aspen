## ADDED Requirements

### Requirement: All VM tests have known pass/fail status

Every NixOS VM test in `nix/tests/` SHALL be categorized as passing, failing (with root cause), or skipped (with justification).

#### Scenario: Test audit produces a status report

- **WHEN** all 63 NixOS VM tests are built via `nix build .#checks.x86_64-linux.<name>`
- **THEN** a status report lists each test with its result (pass/fail/skip) and any failure details

### Requirement: Regressions from hardening sprint are fixed

Any VM test that previously passed but now fails due to changes from the hardening sprint (cluster discovery, write forwarding, tiger style, health gates) SHALL be fixed.

#### Scenario: Regression identified and fixed

- **WHEN** a VM test fails due to a change made in the last 30 days
- **THEN** the root cause is identified, the test is fixed, and the fix is committed

### Requirement: Pre-existing failures are documented

VM tests that were already broken before this sprint SHALL be documented with their failure mode but not fixed in this change.

#### Scenario: Pre-existing failure cataloged

- **WHEN** a VM test fails for reasons unrelated to recent changes
- **THEN** the failure is recorded in the audit report with root cause and the test is marked as a known issue
