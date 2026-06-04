## Context

The artifact registry, transcripts, evaluation cache, and upgrade sessions create a rich graph of runtime information. Humans and agents need a safe way to browse it. Unison Share demonstrates how code hosting can render linked docs and definitions. UCM's MCP server demonstrates that agents benefit from structured tools such as search-by-type, dependencies, dependents, and typecheck.

Molten should provide a catalog service and MCP-compatible tool surface over artifact metadata, while preserving Molten's policy/evidence boundaries and not making names authoritative.

## Goals

- Make artifacts discoverable by semantic metadata and dependency graph position.
- Render docs and transcript outputs with links to exact artifact ids and receipts.
- Provide read-only MCP tools for agents to inspect Molten registries and runtime state.
- Support policy-filtered visibility for private artifacts, capabilities, and receipts.
- Support local-only catalog first, with remote/shared catalog later via Iroh docs/blobs.
- Make catalog views reproducible from registry data where possible.

## Non-Goals

- Do not make a centralized public service mandatory.
- Do not make catalog names authoritative identity.
- Do not expose private artifact contents, capabilities, or policy decisions without authorization.
- Do not give MCP mutating tools ambient runtime authority.
- Do not claim compatibility with Unison Share APIs or UCM MCP tool names.

## Catalog views

The catalog should render:

- artifact summary: id, short id, kind, names/aliases, owner, docs, status.
- dependency graph: direct dependencies, closure summary, reverse dependents.
- schema view: identity mode, structural fingerprint, fields/variants, compatible schemas.
- effect view: required effects, handler profiles, capability scopes.
- policy/evidence view: install/use receipts, provenance refs, review records.
- transcript view: markdown docs, canonical output, failures/bugs, run receipts.
- upgrade view: active sessions, impacted artifacts, task status, cutover state.

Short ids are UI conveniences and must expand to full artifact ids before any operation.

## Search

Search dimensions should include:

- text metadata and docs,
- artifact kind,
- name/alias/tag/channel,
- schema input/output/ref,
- structural schema fingerprint,
- required effect/capability,
- dependency or dependent of artifact id,
- policy or receipt kind,
- provenance/reviewer,
- transcript status,
- upgrade session status.

Results should include why each match was returned and what visibility filters were applied.

## MCP tools

Initial read-only tools:

- `list_artifacts`
- `view_artifact`
- `search_artifacts`
- `search_by_schema`
- `search_by_effect`
- `list_dependencies`
- `list_dependents`
- `view_receipts`
- `view_transcript`
- `list_upgrade_sessions`
- `short_id_resolve`

Later policy-gated tools:

- `dry_run_install`
- `run_transcript`
- `rewrite_preview`
- `create_upgrade_session`
- `remote_sync_plan`

Every tool call should be traceable and should respect the caller's capability and visibility policy.

## Remote catalog

A remote catalog can be built from Iroh docs for mutable metadata and Iroh blobs for immutable artifacts. Public catalogs may publish only selected metadata. Private catalogs require capability-gated access and should avoid leaking names or dependency structure when unauthorized.

## Policy and evidence

Catalog reads can leak sensitive information, so visibility is a policy decision. Mutating MCP tools require explicit capabilities and should emit receipts. Agent actions should include caller identity, tool name, parameters hash, visible result hash, policy decision, and trace refs.

## Open Questions

- Which MCP SDK/protocol crate should Molten use, or should the first server be protocol-minimal?
- Should local catalog be a CLI subcommand, HTTP service, MCP server, or all three behind one query core?
- How should short id ambiguity be displayed to humans and agents?
- Which receipt fields are safe to show in public catalogs?
