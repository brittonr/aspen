# retention-gc-audit-catalog-search

## Summary
Make retention GC plan, apply, execute, and audit artifacts discoverable through the read-only catalog and MCP search surfaces.

## Motivation
Operators can emit audit evidence for a retention GC execution, but they also need to find those chains later by object ref, subsystem, or execution ref without granting any new deletion authority.

## Scope
- Classify retention GC plan/apply/execute/audit artifacts in catalog summaries.
- Add read-only MCP search affordances for retention GC artifacts.
- Ensure audit artifacts have a ledger kind and searchable chain refs.
- Cover object/subsystem/execution-ref search with tests and docs.

## Non-Goals
- Authorizing deletion, clearance, policy, authority, resource, provenance, transport, execution, source-gate, or remote-GC trust.
- Changing destructive retention admission, apply, or execution gates.
- Fetching remote audit artifacts outside the local catalog/ledger inputs.
