## Context

`verification_run_diagnostics` marks a negative run with exit status zero `negative-run-did-not-deny` and an artifact-free run `missing-produced-artifact-ref`; both receipts carry decision `deny`. The parser checked only that the decision matched the supplied diagnostics, and conversion dropped the diagnostics.

## Decisions

### Decision: Diagnostics are recomputed, not trusted

**Choice:** One `run_diagnostics(coverage_kind, exit_status, artifact_refs)` function builds receipts and validates parsed ones; a parsed receipt must carry exactly those diagnostics.

**Rationale:** The diagnostics are a function of fields already in the receipt, so recomputing them rejects a manually assembled receipt that drops one without trusting its author.

### Decision: Invalid runs are stale evidence

**Choice:** `VerificationEvidence` gains `receipt_diagnostics`. Admission reports each as a `stale-…-invalid-run` diagnostic, which the existing classification already treats as a stale reference. Conversion no longer invents artifact presence.

**Rationale:** This reuses the established stale path and keeps the reason in the canonical entry diagnostics and rendered readback. Invalid receipts stay parseable for audit.

## Risks / Trade-offs

- The check is structural: it shows the run matched its coverage purpose and produced an artifact, not why a negative command exited nonzero.
- An entry holding both a valid and an invalid negative run is reported stale rather than covered, so the contradictory evidence is reviewed.
