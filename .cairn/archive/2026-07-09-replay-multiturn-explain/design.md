# Design: replay-multiturn-explain

## Scope

This change covers deterministic replay comparison and debug evidence for already materialized canonical run artifacts. It does not change production execution, live transport, authority admission, provenance admission, policy admission, release promotion, or destructive-operation semantics.

## Proof checklist

- **Proof claim**: replay comparison passes only when the same replay identity and ordered semantic boundary refs reproduce the same output and final-state refs, and denies at the first semantic divergence.
- **Out of scope**: executing live effects during replay, deciding current authority, validating provenance, or treating explain/debug evidence as pass evidence.
- **Trusted assumptions**: canonical Preserves bytes and BLAKE3 refs are stable for compared artifacts and chunk manifests.
- **Positive evidence**: multi-turn fixtures replay to identical report/output/final-state refs and emit a pass replay receipt.
- **Negative evidence**: changed scheduler, input, effect request, effect response, policy decision, hostcall decision, actor output, receipt, output, and state refs deny with the expected first-divergence path.
- **Canonical refs**: replay verify receipt ref, first-divergence ref, run identity ref, turn journal refs, effect log refs, output refs, final-state refs, chunk manifest refs, and explain receipt refs.
- **Regeneration command**: focused replay tests plus the harness replay CLI suite.

## Functional core

The core accepts in-memory parsed replay summaries:

```text
ReplayIdentity
ReplayTraceSummary {
  turn_refs: ordered refs
  boundary_refs: ordered boundary refs per turn
  effect_log_refs: ordered effect request/response refs
  output_refs
  final_state_ref
}
```

It returns a deterministic comparison result:

```text
ReplayComparison {
  decision
  first_divergence: optional canonical DTO
  expected_summary_refs
  actual_summary_refs
}
```

The core does not read files, inspect clocks, perform network I/O, render text, or load chunks. Shell code loads reports/manifests, invokes the pure comparison, writes receipts, and renders summaries.

## CLI shell

`molten test replay compare` accepts two canonical replay artifacts or manifests and writes a `deterministic-replay-verify-v1` receipt. `molten test replay explain` accepts a deny receipt or report pair and writes a canonical explain receipt plus a human-readable summary.

## Large trace strategy

Large traces use a manifest-backed prefix/Merkle shape:

```text
trace manifest
  ├─ run identity ref
  ├─ turn chunk refs
  │   ├─ boundary ref vector root
  │   └─ output/state refs
  └─ effect-log chunk refs
```

The comparator first checks summary roots, then narrows to the first divergent turn and boundary. Partial debug fetches must be covered by chunk range receipts and remain evidence-only.

## Privacy

First-divergence records are safe by default: expected/actual refs and paths are recorded, while raw payloads require a separate trace privacy gate. Redacted explain output must carry a redaction receipt and cannot replace pass replay evidence.

## Non-goals

- No live replay execution or external effect fallback.
- No release evidence regeneration.
- No weakening of existing harness gate, policy, capability, resource, source-gate, or provenance checks.
