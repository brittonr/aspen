## Why

`n0-computer/iroh-experiments` contains several useful Iroh patterns that match Molten's roadmap, but the repository is explicitly experimental and unpolished. Molten should adopt the ideas only after translating them into Molten's canonical Preserves, BLAKE3, capability, policy, resource, and replay evidence model.

The strongest fit is content discovery plus deterministic closure/DAG sync. Those patterns directly improve federated pull sync, remote artifact sync, chunk-store resumability, and job DAG worker fetches without giving transport, tracker, DHT, or naming records authority.

## What Changes

- Add hint-only content locator records inspired by Iroh content discovery: signed announcements, query results, partial/complete claims, verification probes, and optional pkarr-style mutable pointers.
- Add deterministic traversal descriptors for artifact closures, chunk manifests, and job DAG outputs, with receiver-driven missing-set calculation and inline-data policy.
- Add chunk-store sync strategies for stem/leaf or partitioned fetches while preserving manifest identity, resumability, and verification before import.
- Add optional readback adapter boundaries for HTTP/3-over-Iroh and remote byte-source/outboard patterns, but keep canonical Molten gateway requests and receipts as the normative boundary.
- Add positive and negative validation fixtures proving locator evidence remains non-authority, deterministic traversal is replayable, and remote bytes are verified before exposure or import.

## Impact

- **Files**: federated-pull-sync, remote-artifact-sync, content-addressed-chunk-store, node-runtime/readback, testing harness, docs, CLI fixtures, and operator diagnostics.
- **Testing**: positive fixtures for signed locator discovery, deterministic traversal sync, resumable chunk fetch, optional pkarr resolution, and read-only gateway readback; negative fixtures for locator-only import, stale signatures, wrong selectors, mismatched hashes, unsupported hash algorithms, non-deterministic traversal, malformed inline policies, unverified remote bytes, and HTTP/3 transport treated as authority.
- **Security**: trackers, pkarr records, HTTP/3 sessions, S3/HTTP byte locations, Iroh endpoint identity, and random probes remain discovery or transport evidence only. Authority, provenance, source-gate, policy, resource, retention, and execution gates remain separate and fail closed.
