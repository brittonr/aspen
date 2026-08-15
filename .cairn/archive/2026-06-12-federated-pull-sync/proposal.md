## Why

Molten will eventually connect independent nodes or clusters that retain local sovereignty. Aspen federation is useful prior art: independent clusters discover peers, query resource state, fetch missing objects on demand, and verify signatures and content hashes. This should complement Molten's Syndicate/SAM actors, not replace local dataspace semantics.

## What Changes

- Define federated pull-sync for artifacts, catalogs, docs, receipts, chunk manifests, typed storage exports, and application-level resources.
- Make synchronization receiver-driven: peers advertise or answer queries, but receivers decide what to fetch and admit.
- Verify remote data through origin signatures, delegate/authority signatures where applicable, and content/chunk hashes.
- Keep each cluster/node sovereign; no global Raft or global dataspace is implied.
- Integrate with peer bootstrap, authority/revocation, remote artifact sync, content-addressed chunk store, catalog visibility, and deterministic record/replay.

## Impact

This gives Molten an eventual-consistency/federation path that respects policy and evidence. The first milestone can model signed announcements, remote resource inventory queries, pull fetch planning, and verification receipts in a local/loopback test.
