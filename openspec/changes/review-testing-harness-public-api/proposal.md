## Why

The testing harness family can accelerate future extraction work, but reusable defaults must be clearly separated from madsim/network/patchbay/runtime adapters before it becomes a stable public surface. Existing smoke and negative fixture patterns needed an explicit public API/readiness decision and fresh evidence.

## What Changes

- **Review `aspen-testing-core` as the reusable default root**: Document `DeterministicClusterController`, `DeterministicKeyValueStore`, wait/assertion utilities, and generic mock state as the canonical reusable testing surface.
- **Classify adapter features for madsim, network, patchbay, VM, and runtime fixtures**: Document `aspen-testing-madsim`, `aspen-testing-network`, and `aspen-testing-patchbay` as explicit adapter crates; keep `aspen-testing` as the compatibility facade.
- **Prove positive reusable smoke coverage and negative adapter-boundary checks**: Add active-change fixtures and evidence for portable default usage, adapter import rejection, dependency graph exclusions, and adapter compatibility.
- **Raise workspace readiness only**: Promote the testing harness family to `extraction-ready-in-workspace`; publication/repo-split remains blocked on human license/publication policy.

## Capabilities

### New Capabilities
- `testing-harness-extraction`: Review testing harness public API readiness/evidence requirements.

### Modified Capabilities
- Extraction inventory and policy record the testing harness readiness decision and evidence rail.

## Impact

- **Files**: `docs/crate-extraction.md`, `docs/crate-extraction/testing-harness.md`, `docs/crate-extraction/policy.ncl`, and OpenSpec artifacts under `openspec/changes/review-testing-harness-public-api/`.
- **APIs**: No production code API change; canonical testing-core surface and adapter ownership are documented.
- **Dependencies**: No workspace dependency change; active-change fixtures reuse local path dependencies.
- **Testing**: Positive downstream fixture, negative adapter-boundary fixture, adapter package checks, readiness checker, `openspec validate review-testing-harness-public-api --strict`, `scripts/openspec-preflight.sh review-testing-harness-public-api`, and `git diff --check`.

## Verification Expectations

- Requirement `testing-harness-extraction.testing-core-default-reusable` / scenario `testing-harness-extraction.testing-core-default-reusable.evidence`: `verification.md` MUST include changed files and evidence showing the positive downstream fixture, metadata, and canonical reusable API owner/import decision.
- Requirement `testing-harness-extraction.adapters-explicit-negative-checked` / scenario `testing-harness-extraction.adapters-explicit-negative-checked.evidence`: the portable fixture dependency graph MUST exclude app/cluster/runtime/patchbay/madsim adapter leaks, and the negative fixture MUST fail without explicit adapter dependencies.
- Requirement `testing-harness-extraction.workspace-readiness-evidenced` / scenario `testing-harness-extraction.workspace-readiness-evidenced.evidence`: readiness checker evidence MUST pass for `--candidate-family testing-harness` before archive while preserving the license/publication blocker.
- `verification.md` MUST include a `## Verification Commands` section listing exact commands and artifacts.
