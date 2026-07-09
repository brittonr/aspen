## Context

Molten has local catalog and MCP read-only capabilities. This change expands the discovery surface in the direction suggested by Unison Share while preserving Molten's stricter non-authority boundaries.

## Design

### Catalog views

Read-only catalog views should link:

- artifact refs, names, aliases, tags, and channels;
- direct dependencies and reverse dependents;
- schemas, unique/structural identity, and compatibility receipts;
- effect manifests and handler profiles;
- docs and executable transcripts;
- provenance, source-gate, policy, and evidence receipts;
- upgrade sessions and impact query receipts;
- release/package snapshots and caveats.

Views render exact refs first and names as metadata.

### MCP tools

MCP-style tools stay read-only under the default profile:

- `search_artifacts`;
- `show_artifact`;
- `dependencies` and `dependents`;
- `search_receipts`;
- `show_receipt`;
- `search_transcripts`;
- `impact_query`;
- `explain_evidence`;
- `show_release_snapshot`.

Tools return structured Preserves/JSON-friendly records plus redacted rendered summaries.

### Authorization and redaction

Every catalog query computes a visibility decision. Private artifact contents, sensitive policy outcomes, secret refs, private capabilities, retention-sensitive tombstones, and denied evidence details are redacted unless the caller has admitted read authority. Redaction decisions are recorded in catalog query receipts.

### Functional core and shell

Pure cores evaluate query filters, join in-memory indexes, apply redaction decisions, and produce stable result sets. Shells read ledgers/indexes, perform MCP I/O, enforce capability policy, and render summaries.

### Non-goals

- Do not claim compatibility with Unison Share, UCM APIs, or UCM MCP tool names.
- Do not add mutating MCP tools to the read-only catalog profile.
- Do not expose private artifact contents or sensitive decisions through search snippets.
- Do not let catalog query results grant execution, install, release, retention, or policy authority.