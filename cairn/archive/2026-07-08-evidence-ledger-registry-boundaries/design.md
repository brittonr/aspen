## Context

Molten uses evidence chains, artifact ledgers, catalogs, and registries to make review surfaces discoverable and tamper-evident. These concepts are adjacent but have different claims: evidence says what was checked, ledger says what was stored, registry says what can be found.

## Design

### Evidence boundary

Evidence modules own chain scopes, links, checkpoints, predicate receipts, proof summaries, non-claim text, and canonical constructors/parsers. They should not require local ledger storage to validate in-memory evidence values.

### Ledger boundary

Ledger modules own content-addressed persistence, pins, tombstone-safe deletion hooks, import/export, and local store indexing. Ledger presence is storage evidence only.

### Registry/catalog boundary

Registry/catalog modules own classification, search, MCP read-only views, and operator discovery. Discovery must not grant authority, provenance, policy, retention, source-gate, or execution trust.

### Test strategy

Positive tests should validate a stored evidence artifact can be discovered and parsed through the appropriate layers. Negative tests should show stored-but-malformed artifacts, registry-only discovery, stale chain links, and missing predicate receipts do not promote trust.

## Non-goals

- Do not change chain hash semantics.
- Do not remove ledger or catalog commands.
- Do not make artifact presence equivalent to evidence validity.
