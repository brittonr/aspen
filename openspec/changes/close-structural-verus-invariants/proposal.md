## Why

Aspen's broad proof-gap drain reduced trusted `#[verifier(external_body)]` markers to the remaining non-trivial structural invariants. These are not crypto assumptions; they are collection, FIFO, index, and operation-preservation proofs that should be discharged with helper lemmas and spec alignment instead of left trusted.

## What Changes

- **Coordination proofs**: Close remaining queue FIFO/invariant, registry index, worker invariant, strategies fairness, and fencing arithmetic helpers.
- **Core proofs**: Close remaining directory remove uniqueness and index delete/lookup proofs.
- **Commit DAG diff proofs**: Close sort/order and added/removed/changed entry validity proofs.

## Capabilities

### Modified Capabilities
- `verus-proof-trust`: Remaining structural `external_body` markers are converted into verified proof bodies or explicitly documented blockers with a narrower follow-up.
- `coordination`: Queue/registry/worker/fencing model invariants remain aligned with executable behavior.
- `core`: Directory/index state-model invariants remain aligned with modeled Map/Set/Seq updates.
- `commit-diff`: Diff output validity and order facts become inspectable verification evidence.

## Impact

- **Files**: `crates/aspen-coordination/verus/{queue_ack_spec.rs,registry_ops_spec.rs,strategies_spec.rs,worker_ops_spec.rs,fencing_spec.rs}`, `crates/aspen-core/verus/{directory_ops_spec.rs,index_spec.rs}`, `crates/aspen-commit-dag/verus/diff_spec.rs`.
- **APIs**: No public runtime API changes expected.
- **Dependencies**: None expected.
- **Testing**: Run affected Verus roots and focused Rust tests for coordination, core, and commit-dag diff behavior.
