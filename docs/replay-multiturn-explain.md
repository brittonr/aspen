# Multi-turn replay compare and explain

Multi-turn replay comparison is a pure boundary over already materialized canonical replay summaries. It compares run identity, ordered semantic boundary refs, effect-log refs, output refs, final-state refs, and aggregate turn refs, then emits a canonical comparison receipt before any human summary is rendered.

First-divergence evidence records path metadata only: turn index, event index, boundary kind, optional actor/session/vat ids, field path, expected ref, actual ref, handler profile ref, and refs-only redaction status. It does not include raw payloads and remains evidence-only.

Large traces can use manifest-backed prefix comparison. The prefix receipt binds manifest refs, summary roots, turn chunk refs, effect-log chunk refs, and the requirement that partial debug fetches be covered by range receipts.

CLI shells under `molten test replay-fixture compare` and `molten test replay-fixture explain` read canonical fixtures/receipts, invoke the pure core, write canonical receipts when requested, and only then print rendered summaries. These receipts do not grant authority, policy, provenance, source-gate, transport, retention, release, or execution trust.
