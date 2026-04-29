## Why

Proposal reviews repeatedly find that verification sections omit required paths later introduced in delta specs. Missing proposal-level verification expectations lead to under-scoped designs and task lists.

## What Changes

- Require proposals to list verification expectations for each new or modified capability.
- Require proposal verification to include at least one negative path when specs or impact mention rejection/failure behavior.
- Add a proposal-gate check that flags unreferenced added requirement IDs.

## Capabilities

### New Capabilities
- `openspec-governance.proposal-verification-traceability`: Proposals trace verification expectations to changed requirements.

## Impact

- **Files**: proposal template/instructions, proposal gate logic, fixtures.
- **Testing**: Positive proposal with full traceability and negative proposals with missing positive/negative coverage.
