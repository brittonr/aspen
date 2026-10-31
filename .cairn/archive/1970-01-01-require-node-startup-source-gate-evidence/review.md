# Implementation review: F02 real startup evidence

## Disposition

Normal startup consumes independently authenticated exact-candidate evidence and no longer
invents a passing Octet receipt. Executed commands and results are in
`evidence/2026-10-31-verification.md`; the producer contract is reviewed in
`produce-exact-node-startup-binding/review.md`.

## Reviewed invariants

- `run_local_with_root` and the read-only source gate both call `source_binding::load` before
  restart preparation, lifecycle writes or activation.
- Rejected startup preserves the active lock, shutdown receipt, startup receipt and prior
  evidence, and prepares no receipts or validation artifact.
- Missing, stale, tampered, wrong-candidate, missing-closure, synthetic, read-error, unsigned and
  wrong-key inputs all deny through source admission rather than a later effect boundary.
- Synthetic composition exists only behind `#[cfg(test)]` constructors with a separate fixture
  validation schema; historical synthetic receipts remain diagnostics.
- Receipt and binding schemas are versioned; an incompatible producer or cohort requires a new
  version.

## Non-claims

Source-gate acceptance does not prove compiler correctness, whole-node correctness, adapter
readiness, deployment success or release eligibility, and does not flip production admission.
The positive control used an explicitly experimental signer.
