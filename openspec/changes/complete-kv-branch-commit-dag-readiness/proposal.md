## Why

KV branch and commit DAG are reusable primitives for sagas, speculative writes, and federation, but the manifest still requires proof that normal graphs do not depend on Raft/runtime shells.

## What Changes

- **Localize commit hash/helper surfaces in `aspen-commit-dag`**: Localize commit hash/helper surfaces in `aspen-commit-dag`.
- **Prove branch/DAG feature boundaries and no normal `aspen-raft` dependency**: Prove branch/DAG feature boundaries and no normal `aspen-raft` dependency.
- **Capture downstream and representative consumer compatibility evidence**: Capture downstream and representative consumer compatibility evidence.

## Capabilities

### New Capabilities
- `kv-branch-commit-dag-extraction`: Complete KV branch and commit DAG readiness evidence readiness/evidence requirements.

### Modified Capabilities
- Existing extraction and dogfood evidence inventories gain an active implementation target with explicit verification rails.

## Impact

- **Files**: OpenSpec artifacts under `openspec/changes/complete-kv-branch-commit-dag-readiness/`.
- **APIs**: No immediate code API change; implementation tasks will decide stable public API or evidence surfaces.
- **Dependencies**: No dependency change in this spec-only slice.
- **Testing**: `openspec validate complete-kv-branch-commit-dag-readiness --strict`, helper verification, `git diff --check`, and the change-specific verification tasks.
