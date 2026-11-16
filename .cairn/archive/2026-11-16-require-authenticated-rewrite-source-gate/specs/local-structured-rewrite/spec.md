# Local Structured Rewrite Delta

## ADDED Requirements

### Requirement: Rewrite upgrade plans require explicit authority and authenticated evidence
r[molten.local_rewrite.upgrade_hook_authority] The rewrite upgrade-session hook MUST require explicit operator-supplied initiator, capability and policy refs and authenticated source-gate evidence whose strict current-workspace validation passes. It MUST NOT derive authority refs from labels or session ids. Missing, unauthenticated, stale or unavailable evidence and invalid refs MUST fail before applying the rewrite or writing an upgrade plan.

#### Scenario: Missing evidence or refs leaves no plan
- GIVEN `molten rewrite apply --upgrade-plan-out` without an authenticated source state root, with a forged gate or trust file, or without explicit initiator, capability or policy refs
- WHEN the command runs
- THEN it fails, writes no upgrade plan and installs no rewritten artifact

#### Scenario: Explicit refs and authenticated evidence produce a plan
- GIVEN a valid rewrite request, explicit canonical refs and an authenticated current source gate
- WHEN the combined upgrade-plan entrypoint admits and applies the rewrite
- THEN the plan binds those refs, install tasks and the authenticated gate

#### Scenario: Valid produced gate is stale for the workspace
- GIVEN an authenticated produced gate whose receipt structure passes but whose workspace config hash is stale, or no current workspace is available
- WHEN the combined upgrade-plan entrypoint runs
- THEN admission fails before any replacement installation or upgrade-plan publication
