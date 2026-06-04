## Phase 1: Federation records

- [x] [serial] r[molten.federation.announcement_model] Define signed announcement and inventory records for federated resources.
- [x] [serial] r[molten.federation.resource_types] Define initial federated resource types for artifacts, chunk manifests/chunks, docs/catalog metadata, receipts, provenance, transcripts, protocols, and schemas.
- [x] [parallel] r[molten.federation.no_global_consistency] Document that federation does not imply global Raft, global dataspace, or pushed state.
- [x] [parallel] r[molten.federation.receipts] Emit receipts for discovery, query, fetch, verification, admission, import, denial, and merge.

## Phase 2: Pull workflow and verification

- [x] [serial] r[molten.federation.pull_workflow] Implement receiver-driven query, missing-set calculation, fetch plan, verification, and local admission flow.
- [x] [serial] r[molten.federation.verification_layers] Verify origin signatures, delegate/capability signatures, content/chunk hashes, and local policy before import.
- [x] [parallel] r[molten.federation.static_discovery] Support static/configured peers as the first discovery mode.
- [x] [parallel] r[molten.federation.rate_limits] Apply resource and rate-limit policy to announcements, inventory queries, and fetches.

## Phase 3: Integration and tests

- [x] [serial] r[molten.federation.dataspace_status] Represent sync status, imported resources, denials, and peer availability as local dataspace assertions.
- [x] [parallel] r[molten.federation.remote_sync_chunk_store] Integrate with remote artifact sync and content-addressed chunk-store fetch/verification.
- [x] [serial] r[molten.federation.loopback_tests] Add loopback tests for signed announcement, pull fetch, verification, import, and denial.
- [x] [parallel] r[molten.federation.property_tests] Add Hegel property tests for receiver-driven sync, no-push-import, and verification-before-import invariants.
