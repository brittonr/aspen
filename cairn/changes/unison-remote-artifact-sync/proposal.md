## Why

Molten intends to execute work across actors, peers, and sandboxes, but remote execution should not require out-of-band container images, manually synchronized policy files, or ad hoc dependency bundles. Unison's remote computation model shows the useful pattern: send a reference to content-addressed computation, discover missing dependencies, sync them on demand, and cache them.

Molten should adopt that dependency-closure sync pattern over Iroh and the artifact registry, while keeping execution admitted, capability-scoped, and sandboxed. It should not support arbitrary mobile closures or bypass policy gates.

## What Changes

- Define a remote execution/install protocol based on artifact root ids and dependency closure manifests.
- Let receivers compute missing artifact ids, request them over Iroh blobs/docs, verify content hashes, and cache admitted artifacts.
- Require all synced artifacts to pass registry installation policy before use.
- Represent remote jobs as canonical envelopes carrying artifact id, entrypoint, arguments, declared effect profile, capabilities, and evidence refs.
- Bind remote execution to effect/capability manifests and handler profiles available on the target peer.
- Emit receipts for dependency discovery, fetch, verification, admission, execution start, effect use, result, and failure.
- Keep ordinary actor messages and choreography traffic separate from remote artifact sync, though they may reference synced artifacts.

## Impact

This makes Molten remote execution reproducible and cacheable without requiring pre-installed code on every peer. The first milestone can stay in-process or loopback: compute a dependency closure, remove one dependency from a target cache, fetch it by content id, verify it, admit it, and run a local handler-backed job.
