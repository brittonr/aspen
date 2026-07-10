## Why

Unison's structured refactor/update workflow is useful because changes are planned over semantic definitions rather than blind text edits. Molten needs the same planning discipline for artifact aliases, schemas, protocols, policies, handler profiles, storage records, transcripts, and cleanup work.

Molten should adapt this as receipt-backed upgrade sessions. Sessions plan changes over exact refs and dependency indexes, track task state as evidence, and require gates before cutover or cleanup side effects.

## What Changes

- Define structured upgrade session artifacts for alias moves, artifact replacements, schema migrations, protocol drains, policy updates, handler-profile changes, transcript rewrites, and cleanup.
- Store mutable task progress as receipt-backed metadata, not by changing the canonical plan artifact.
- Require impact query, compatibility, protocol-session, migration, replay, and policy evidence before cutover.
- Document that sessions do not replace Git, Cargo, Nix, Cairn, or repo-local review workflows.

## Impact

- **Files**: upgrade sessions, artifact registry, dependency impact index, schema identity, typed storage, protocol gates, transcript/replay, docs.
- **Testing**: positive fixtures for gated cutover; negative fixtures for stale impact evidence, missing protocol drain, incomplete migration, failed replay, and name-only updates.
- **Security**: upgrade sessions coordinate changes but do not grant authority, policy trust, provenance, source-gate trust, transport, retention, or execution rights.