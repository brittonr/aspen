## Why

The testing harness family can accelerate future extraction work, but reusable defaults must be clearly separated from madsim/network/patchbay/runtime adapters before it becomes a stable public surface.

## What Changes

- **Review `aspen-testing-core` as the reusable default root**: Review `aspen-testing-core` as the reusable default root.
- **Classify adapter features for madsim, network, patchbay, VM, and runtime fixtures**: Classify adapter features for madsim, network, patchbay, VM, and runtime fixtures.
- **Prove positive reusable smoke coverage and negative adapter-boundary checks**: Prove positive reusable smoke coverage and negative adapter-boundary checks.

## Capabilities

### New Capabilities
- `testing-harness-extraction`: Review testing harness public API readiness/evidence requirements.

### Modified Capabilities
- Existing extraction and dogfood evidence inventories gain an active implementation target with explicit verification rails.

## Impact

- **Files**: OpenSpec artifacts under `openspec/changes/review-testing-harness-public-api/`.
- **APIs**: No immediate code API change; implementation tasks will decide stable public API or evidence surfaces.
- **Dependencies**: No dependency change in this spec-only slice.
- **Testing**: `openspec validate review-testing-harness-public-api --strict`, helper verification, `git diff --check`, and the change-specific verification tasks.
