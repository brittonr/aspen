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

## Verification Expectations

For `kv-branch-commit-dag-extraction.commit-dag-avoids-raft` and
`kv-branch-commit-dag-extraction.commit-dag-avoids-raft.evidence`, the
implementation drain MUST capture source-ownership evidence, leaf crate
`cargo check` transcripts, negative dependency graph scans, and representative
consumer compile checks.

For `kv-branch-commit-dag-extraction.kv-branch-boundaries-feature-gated` and
`kv-branch-commit-dag-extraction.kv-branch-boundaries-feature-gated.evidence`,
the implementation drain MUST capture downstream fixture metadata/check/test
transcripts, forbidden dependency scans, and a repo-local preflight transcript
before marking evidence tasks complete. If a representative consumer fails for a
pre-existing compatibility issue outside the branch/DAG leaf graph, the failure
MUST be preserved as evidence and the family MUST remain `workspace-internal`
until a follow-up fixes that path.
