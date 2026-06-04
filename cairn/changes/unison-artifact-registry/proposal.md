## Why

Molten already treats runtime communication, receipts, and large payloads as canonical Preserves values plus Blake3 content references, but it does not yet define the same stable identity model for executable and declarative runtime artifacts. Without this, protocol manifests, Wasm modules, Steel predicates, Nickel contracts, schemas, migrations, docs, and test transcripts can drift back toward filename, package-version, or human-name identity.

Unison is useful prior art here: definitions are identified by content hash, while names are metadata. Molten should adopt that pattern for runtime artifacts without adopting Unison, UCM, or Unison source compatibility.

## What Changes

- Add a content-addressed artifact registry for Molten runtime artifacts.
- Treat artifact names, aliases, versions, tags, owners, and docs as mutable metadata over immutable artifact hashes.
- Hash canonical artifact representations, not raw Rust memory, debug formatting, local paths, or non-normalized source text.
- Track dependency edges between artifacts so tools can compute dependency closures, reverse dependencies, impact sets, and installation plans.
- Store artifact bytes and metadata through Redb locally and Iroh blobs/docs remotely, while keeping registry semantics transport-neutral.
- Include schemas, effect/capability manifests, policy references, receipts, and evidence refs in artifact metadata.
- Support semantic lookup by artifact kind, type/schema, required effects, capabilities, dependencies, provenance, and docs.
- Support executable docs and transcript artifacts whose examples reference concrete artifact hashes and run through admitted local handlers.
- Keep Unison as non-normative inspiration only; do not require UCM, Unison syntax, or Unison hash compatibility.

## Impact

This gives Molten a stable substrate for remote dependency sync, typed durable storage, effect manifests, structured upgrades, semantic documentation, and policy/evidence review. The first implementation can be minimal: define artifact DTOs, canonical hashing, a Redb-backed index, and tests showing names can move without changing artifact identity.
