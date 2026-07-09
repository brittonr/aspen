## Why

Unison Share and UCM-style inspection show how valuable a browsable semantic graph can be. Molten needs a read-only catalog that helps humans and agents inspect artifact refs, dependencies, schemas, effects, docs, transcripts, receipts, upgrades, and release evidence without receiving ambient authority.

The adaptation should prioritize linked views, exact refs, redaction, and MCP-style read-only tools. Mutation remains behind explicit subsystem commands and gates.

## What Changes

- Add Share-like linked catalog views over artifacts, dependency edges, dependents, schemas, effects, transcripts, docs, receipts, upgrade sessions, and release snapshots.
- Add read-only MCP tools for search, show, dependency/dependent queries, receipt lookup, transcript lookup, impact queries, and evidence explanation.
- Bind authorization and redaction decisions into catalog receipts.
- Deny catalog/MCP attempts to mutate artifacts, names, policies, capabilities, storage, retention, release, or execution state unless routed through explicit mutating gates outside the read-only catalog profile.

## Impact

- **Files**: catalog, MCP facade, redaction/confidentiality, artifact registry, upgrade sessions, release evidence, docs.
- **Testing**: positive fixtures for linked views and read-only queries; negative fixtures for redaction leaks, mutation attempts, private artifact contents, and Unison API compatibility claims.
- **Security**: catalog output is discovery/explanation evidence only. It does not grant authority, provenance, policy trust, retention, transport, release, or execution rights.