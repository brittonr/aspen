## Why

Molten now produces many local, immutable artifacts and receipts: artifact registry entries, schema identities, typed-storage refs, evaluation-cache entries, executable transcripts, structured rewrites, upgrade plans, chunk manifests, chain evidence, and harness reports. These are evidence-rich, but inspection is scattered across subsystem-specific CLI commands.

A local artifact catalog gives Molten a unified, visibility-filtered inspection core before exposing any read-only MCP tools or remote catalog views. Catalog queries must resolve immutable artifact ids and receipt refs, not mutable names, paths, mtimes, or display aliases.

## What Changes

- Add a local catalog query core over the artifact registry and evidence ledger.
- Define canonical catalog summary, view, search query, search result, short-id resolution, and catalog receipt records.
- Render artifact, dependency, dependent, schema, effect, policy/evidence, transcript, rewrite, upgrade, chunk, and receipt views from canonical refs.
- Implement semantic search by artifact kind, schema ref, structural fingerprint, effect/capability refs, dependency, evidence/receipt refs, transcript status, rewrite/upgrade status, and bounded text terms.
- Add visibility filtering and redaction hooks before returning catalog results.
- Add unambiguous short-id resolution for CLI/UI convenience only; operations expand to full refs before use.
- Add local CLI commands under `molten test catalog list|view|search|deps|dependents|short-id`.

## Impact

This makes Molten's local evidence surface inspectable without weakening identity, authority, or redaction boundaries. It sets up future read-only MCP tools over the same query core and gives rewrite/upgrade/transcript workflows a shared way to show exact refs and impact.
