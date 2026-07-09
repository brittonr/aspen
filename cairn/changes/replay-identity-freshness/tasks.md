# Tasks: replay-identity-freshness

## Phase A: Identity model and pure validation

- [ ] [serial] r[molten.determinism.replay_freshness.identity_binding] Define replay run identity DTOs with artifact, dependency closure, initial state, schema, policy, capability, handler profile, seed/effect-log, runtime/tool, and replay profile refs.
- [ ] [serial] r[molten.determinism.replay_freshness.identity_binding] Add pure freshness validation between expected subject identity and supplied replay evidence identity.
- [ ] [parallel] r[molten.determinism.replay_freshness.index_binding] Extend replay index summaries to preserve member identity refs and detect stale members.

## Phase B: Release and catalog integration

- [ ] [serial] r[molten.release.replay_freshness.readback] Bind release/dogfood replay readback to expected run identity refs and deny missing or stale identity evidence.
- [ ] [parallel] r[molten.catalog.replay_freshness.identity_search] Classify replay identity and freshness diagnostics in catalog search.
- [ ] [parallel] r[molten.catalog.replay_freshness.evidence_only] Preserve evidence-only caveats in freshness receipts, release readback, and catalog/MCP responses.

## Phase C: Tests and docs

- [ ] [serial] r[molten.determinism.replay_freshness.tests] Add positive matching-identity replay freshness tests.
- [ ] [serial] r[molten.determinism.replay_freshness.tests] Add stale-component denial matrix for artifact, dependency closure, initial state, schema, policy, capability, handler profile, seed/effect-log, runtime, tool, and replay profile refs.
- [ ] [serial] r[molten.determinism.replay_freshness.tests] Add release readback tamper tests and catalog identity search tests.
- [ ] [serial] r[molten.determinism.replay_freshness.tests] Document freshness semantics and evidence-only limits.
