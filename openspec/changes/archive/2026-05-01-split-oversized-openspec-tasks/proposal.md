## Why

Task gates repeatedly flag oversized or incoherently ordered tasks that bundle multiple requirements. Oversized tasks make completion ambiguous and failure isolation poor.

## What Changes

- Add tasks-stage guidance and checks for task size, dependency order, and requirement fan-out.
- Require tasks that cover many scenario IDs to be split or explicitly marked as an integration verification task.
- Warn when implementation tasks depend on later foundation tasks.

## Capabilities

### New Capabilities
- `openspec-governance.task-size-and-ordering`: Task lists are small enough and ordered enough to verify cleanly.

## Verification Expectations

- `openspec-governance.task-size-and-ordering`: positive task-gate fixture covers bounded and dependency-aware task lists.
- `openspec-governance.task-size-and-ordering.bounded-implementation-task-passes`: bounded implementation fixture with small `[covers=...]` fan-out must pass.
- `openspec-governance.task-size-and-ordering.oversized-implementation-task-flagged`: oversized implementation fixture must fail with split guidance.
- `openspec-governance.task-size-and-ordering.integration-verification-may-be-broad`: broad verification fixture labeled as an integration proof with evidence expectations must pass.
- `openspec-governance.task-size-and-ordering.dependency-order-explicit`: out-of-order and ambiguous dependency fixtures must fail unless an explicit prerequisite note is present.

## Impact

- **Files**: tasks template/instructions, tasks gate logic, fixtures.
- **Testing**: Positive small task fixture; negative oversized, out-of-order, and ambiguous dependency fixtures.
