## Why

Replay diagnostics are strongest when the verifier can compare real multi-turn evidence and identify the first semantic divergence with enough path metadata to debug without exposing sensitive payloads. Current fixture replay and generic harness replay provide the right foundation, but the next step is to make the comparison core reusable across reports, vats, node workflows, job workers, coordination, and remote dataspace traces.

## What Changes

- Add a generic multi-turn replay comparison core over ordered run identity, turn journal, effect log, output, and final-state refs.
- Extend first-divergence evidence with canonical path metadata: turn index, event index, boundary kind, actor/session/vat identifiers when present, field path, expected ref, actual ref, handler profile ref, and redaction status.
- Add replay explain and compare CLI surfaces that emit canonical receipts before rendering human summaries.
- Add manifest/Merkle-backed prefix comparison so large traces can find divergent turns without materializing entire reports.

## Impact

- **Files**: deterministic replay core, harness replay comparator, replay CLI, catalog classification hooks, docs, and tests.
- **Testing**: positive multi-turn replay stability, negative divergence matrix, explain CLI receipt tests, large-trace prefix comparison tests, and privacy/redaction denial tests.
- **Boundaries**: evidence-only replay diagnostics; no new authority, policy, provenance, source-gate, transport, retention, or release trust.
