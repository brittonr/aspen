## Why

Review metrics show repeated task-stage omissions where checked tasks lack durable evidence, cite placeholder files, or leave verification references disconnected from requirement coverage. This lets OpenSpec changes appear done while reviewers cannot reproduce the proof.

## What Changes

- Add a deterministic task-evidence coverage rail for active and archived changes.
- Require every checked task to cite tracked, existing, change-local evidence or currently changed source/doc files.
- Reject placeholder or empty evidence artifacts.
- Emit actionable failure messages that name the task, missing evidence, and expected remediation.

## Capabilities

### New Capabilities
- `openspec-governance.task-evidence-coverage`: Checked OpenSpec tasks have durable, reviewable evidence.

## Impact

- **Files**: `scripts/openspec-preflight.sh`, OpenSpec gate/template docs, tests/fixtures for preflight behavior.
- **APIs**: No runtime API changes.
- **Dependencies**: None expected.
- **Testing**: Positive fixture with complete evidence; negative fixtures for missing, untracked, empty, and placeholder evidence.

## Verification

Run the preflight fixture suite, run `scripts/openspec-preflight.sh` against a known-good change, and save positive/negative transcripts.
