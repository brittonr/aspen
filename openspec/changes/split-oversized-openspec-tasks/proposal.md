## Why

Task gates repeatedly flag oversized or incoherently ordered tasks that bundle multiple requirements. Oversized tasks make completion ambiguous and failure isolation poor.

## What Changes

- Add tasks-stage guidance and checks for task size, dependency order, and requirement fan-out.
- Require tasks that cover many scenario IDs to be split or explicitly marked as an integration verification task.
- Warn when implementation tasks depend on later foundation tasks.

## Capabilities

### New Capabilities
- `openspec-governance.task-size-and-ordering`: Task lists are small enough and ordered enough to verify cleanly.

## Impact

- **Files**: tasks template/instructions, tasks gate logic, fixtures.
- **Testing**: Positive small task fixture; negative oversized, out-of-order, and ambiguous dependency fixtures.
