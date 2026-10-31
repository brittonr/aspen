# Implementation review: exact node startup binding producer

## Disposition

The producer, independent authentication, pure admission, startup integration and operator
documentation are implemented and were executed against real producer outputs. This review is
not production trust approval, compiler-correctness evidence or release authority. Executed
commands and results are in `evidence/2026-10-31-verification.md`.

## Reviewed invariants

- One immutable source tree feeds both the compiled candidate and the real primary/library Octet
  cohort; the emitted binding is detached and never embeds an executable self-hash.
- Authentication selects one full public key supplied outside the evidence and verifies the
  containing NAR with an explicit signature threshold; content-addressed status, local-build
  trust and evidence-carried keys are refused.
- The loader re-measures `/proc/self/exe`, the retained cohort artifacts and all 21,043 build
  closure paths before bounded pure admission, and runs before any restart preparation or
  lifecycle write.
- Fixture constructors and fixture validation schemas are `#[cfg(test)]` only; normal wrappers
  cannot select them, and historical receipts never gain authenticity during replay.

## Defects found and fixed

- The private release-channel Cargo source was fetched with `pkgs.fetchgit` inside the sandbox,
  which has no ssh, so the producer only built while a prefetched store path survived GC. The
  exact pinned revision now resolves through the client-fetched flake input (`567be00a0`).
- `MAX_CLUSTER_CHILD_TIMEOUT_MS` (300 s) did not admit a signed child start that re-measures the
  build closure on a loaded host. It is now 30 minutes with a regression test.

## Non-claims

The producer does not establish compiler correctness, whole-node correctness, deployment
permission or production operator approval. The experimental signer used here is not production
trust.
