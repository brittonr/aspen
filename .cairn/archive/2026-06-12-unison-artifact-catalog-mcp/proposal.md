## Why

A content-addressed runtime needs good discovery and inspection tools. Developers, operators, and agents need to find artifacts by schema, type, effect, capability, dependency, provenance, docs, and receipts. Without a catalog and tool API, the registry will be correct but hard to use.

Unison Share and UCM's MCP server are useful prior art: code is browsable, docs render with links, semantic search works, and agents can inspect/typecheck/search through a tool protocol. Molten should adopt a catalog and MCP-style introspection surface for artifacts and runtime evidence.

## What Changes

- Add an artifact catalog service over the Molten registry.
- Render docs, schemas, effect manifests, dependency graphs, receipts, upgrade sessions, and transcript outputs by artifact id.
- Provide semantic search by name, kind, schema, effect, capability, dependency, provenance, policy, receipt, and text metadata.
- Add an MCP-compatible introspection server for agents and tools.
- Expose read-only tools first: list/search/view artifacts, dependencies, dependents, schemas, effects, receipts, transcripts, and upgrade plans.
- Add policy-gated mutating tools later for dry-run install, transcript run, structured rewrite preview, and upgrade-session creation.
- Keep catalog metadata as views over canonical registry artifacts, not authoritative identity.

## Impact

Molten gets a developer and agent UX comparable to the power of its artifact model. The first milestone can be local-only and read-only: serve catalog queries and MCP tools from Redb registry indexes with policy-filtered visibility.
