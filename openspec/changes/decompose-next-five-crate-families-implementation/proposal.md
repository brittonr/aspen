## Why

The parent change `decompose-next-five-crate-families` completed the next-wave selection, family baselines, roadmap, and full manifest setup. The remaining policy/checker work, crate moves, downstream fixtures, and compatibility rails are implementation-heavy and exceed the local drain budget for a single parent task.

## What Changes

- Carry forward the parent implementation tasks for policy/checker support and first-blocker slices across `foundational-types`, `auth-ticket`, `jobs-ci-core`, `trust-crypto-secrets`, and `testing-harness`.
- Add the required downstream fixtures, negative boundary proofs, positive/negative tests, and compatibility evidence before any selected family readiness state is raised.
- Preserve the parent constraints: no publication/repository split, no readiness beyond `workspace-internal` or `extraction-ready-in-workspace`, and out-of-order work must record bypassed prerequisites and compatibility guards.

## Parent Change

- Parent: `decompose-next-five-crate-families`
- Deferred tasks: parent I3 through I17 and V1 through V4.

## Impact

- **Docs/policy**: `docs/crate-extraction/policy.ncl`, `scripts/check-crate-extraction-readiness.rs`, family manifests.
- **Code**: selected crates only, in bounded first-blocker slices.
- **Testing**: downstream fixture metadata/checks, negative boundary mutations, compatibility compile/test rails, positive/negative behavior tests.
