## Why

Per-remote destructive retention clearance now fails closed unless operators provide current `retention-remote-gc-clearance-v1` evidence. That evidence is currently produced as a local fixture. Molten needs a transport-neutral request/response/import workflow so a peer can evaluate its own retained refs and return peer-produced clearance evidence before local deletion, tombstoning, redaction, or compaction.

## What Changes

- Add canonical remote-clearance request, response, and import receipt records.
- Let a requester build a clearance request that binds peer, remote, object, class, action, policy, authority, and supporting evidence refs.
- Let the addressed peer answer with a clearance response that embeds the exact clearance receipt value it produced, including retained/revoked/stale diagnostics.
- Let the requester import only passing, scope-matching peer clearance responses into its local retention store before destructive admission can use them.
- Add CLI coverage for the request/respond/import workflow and summaries.

## Impact

- **Files**: `src/retention.rs`, `src/main.rs`, `src/preserves_rail.rs`, `src/ledger.rs`, README/docs, Cairn runtime-spine specs.
- **Testing**: Unit and CLI tests for pass import, retained/stale peer denial, tampered/wrong request response denial, and destructive admission using imported peer clearance.
