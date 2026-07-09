# Replay identity freshness

Replay freshness compares the expected subject identity for a release, dogfood output, subsystem, or coverage row against the identity carried by replay evidence. The pure validator binds artifact, dependency closure, initial state, schema, policy, capability, revocation, handler profile, seed-or-effect-log, runtime, tool, and replay profile fields before accepting replay evidence as fresh.

Freshness receipts are canonical evidence. They emit the expected and supplied identity refs, a pass/deny decision, stale-component diagnostics, and evidence-only checks. A pass means the replay evidence applies to the named deterministic identity; it does not grant authority, policy, provenance, source-gate, release, resource, transport, retention, or execution trust.

Replay rollups and indexes preserve member identity refs so catalog search and release readback can find stale or matching replay evidence without inspecting payload contents. Catalog classifications include `replay-identity-ref:*`, `replay-rollup-identity:*`, and `replay-index-identity:*` terms, plus freshness receipt terms from the pure catalog helper.
