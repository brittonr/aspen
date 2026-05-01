## Why

Proposal reviews repeatedly find that verification sections omit required paths later introduced in delta specs. Missing proposal-level verification expectations lead to under-scoped designs and task lists.

## What Changes

- Require proposals to list verification expectations for each new or modified capability.
- Require proposal verification to include at least one negative path when specs or impact mention rejection/failure behavior.
- Add a proposal-gate check that flags unreferenced added requirement IDs.

## Capabilities

### New Capabilities
- `openspec-governance.proposal-verification-traceability`: Proposals trace verification expectations to changed requirements.

## Verification Expectations

- `openspec-governance.proposal-verification-traceability`: positive proposal gate fixture covers requirement-ID verification mapping.
- `openspec-governance.proposal-verification-traceability.cites-requirement-ids`: positive fixture cites all changed IDs; valid deferral fixture uses `defer to design` with requirement ID and rationale.
- `openspec-governance.proposal-verification-traceability.missing-positive-verification-fails`: negative fixture omits an added requirement ID and must fail proposal gate checking.
- `openspec-governance.proposal-verification-traceability.negative-behavior-needs-verification`: negative fixture mentions rejection/failure behavior without negative-path verification and must fail.

## Impact

- **Files**: proposal template/instructions, proposal gate logic, fixtures.
- **Testing**: Positive proposal with full traceability and negative proposals with missing positive/negative coverage.
