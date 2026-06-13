## Why

The upgrade-session slice now has canonical plans, receipts, name pointers, and cleanup checks, but its impact analysis is deliberately temporary: it scans ledger artifact text for references. That is enough to keep the first workflow safe, but it is not the registry Molten needs for real upgrades, typed-storage migrations, provenance policy, catalog queries, or cleanup safety.

Molten needs a local artifact registry that treats artifact identity as immutable content and treats names, aliases, tags, and channels as mutable metadata with receipts. The registry should provide explicit dependency and reverse-dependency indexes so upgrade sessions can compute impact sets without string-scanning opaque artifacts.

## What Changes

- Add a canonical local artifact DTO with kind, payload ref/inline payload, schema refs, dependency refs, effect manifest ref, policy refs, evidence refs, and domain-separated artifact id.
- Add a local Redb-backed registry index for installed artifacts, name/alias/tag/channel metadata, dependency edges, reverse dependencies, and receipt refs.
- Store large immutable artifact payloads via content/chunk refs while preserving canonical artifact identity.
- Emit receipts for artifact installation, name/alias moves, dependency-closure admission, query/impact calculations, and denial cases.
- Provide query APIs and CLI commands for install, list, view, name set/show, dependencies, dependents, closure, and impact.
- Wire upgrade sessions to use registry impact queries when a registry root is provided, while keeping the current ledger scan as a compatibility fallback.
- Prepare later catalog/MCP, provenance, schema identity, retention/GC, evaluation-cache, and remote artifact sync work to reuse the same registry model.

## Impact

This turns the upgrade workflow from a prototype over ledger text into an explicit content-addressed artifact database. It improves cleanup safety, lets tools answer reverse-dependency questions, and provides the first local substrate for schema identity and provenance policies without adopting Unison/UCM or replacing Cargo/Git/Nix workflows.
