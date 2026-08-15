## Why

A replay receipt can pass for the artifact and environment it was recorded against but still be stale for a later release or dogfood output. Replay indexes and release readback should bind freshness to the full deterministic run identity instead of only recording that replay evidence exists.

## What Changes

- Bind replay receipts, rollups, indexes, and release readback to artifact ref, dependency closure ref, initial state ref, schema refs, policy refs, capability refs, handler profile ref, seed or effect-log ref, runtime/tool refs, and replay profile.
- Deny stale replay indexes when any required identity component differs from the release or subsystem evidence they claim to cover.
- Add catalog/readback classifications for replay identity and freshness diagnostics.
- Add positive and negative tests for matching identities, changed artifact refs, changed policy refs, changed dependency closure refs, changed handler profile refs, changed seed/effect-log refs, and stale release-bound indexes.

## Impact

- **Files**: deterministic replay identity DTOs, replay index parsing/validation, release dogfood readback, catalog classifications, docs, and tests.
- **Testing**: identity-bound replay pass tests, stale identity denial matrix, release readback tamper tests, and catalog search tests.
- **Boundaries**: freshness binding proves replay evidence applies to a named deterministic identity; it does not grant authority, policy, provenance, source-gate, release, resource, transport, retention, or execution trust.
