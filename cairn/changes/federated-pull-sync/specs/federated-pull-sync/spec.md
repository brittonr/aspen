# federated pull sync Delta Spec

## ADDED Requirements

### Requirement: Define signed announcement and inventory records for federated resources
r[molten.federation.announcement_model] Define signed announcement and inventory records for federated resources.

### Requirement: Define initial federated resource types for artifacts, chunk manifests/chunks, docs/catalog metadata, receipts, provenance, transcripts, protocols, and schemas
r[molten.federation.resource_types] Define initial federated resource types for artifacts, chunk manifests/chunks, docs/catalog metadata, receipts, provenance, transcripts, protocols, and schemas.

### Requirement: Document that federation does not imply global Raft, global dataspace, or pushed state
r[molten.federation.no_global_consistency] Document that federation does not imply global Raft, global dataspace, or pushed state.

### Requirement: Emit receipts for discovery, query, fetch, verification, admission, import, denial, and merge
r[molten.federation.receipts] Emit receipts for discovery, query, fetch, verification, admission, import, denial, and merge.

### Requirement: Implement receiver-driven query, missing-set calculation, fetch plan, verification, and local admission flow
r[molten.federation.pull_workflow] Implement receiver-driven query, missing-set calculation, fetch plan, verification, and local admission flow.

### Requirement: Verify origin signatures, delegate/capability signatures, content/chunk hashes, and local policy before import
r[molten.federation.verification_layers] Verify origin signatures, delegate/capability signatures, content/chunk hashes, and local policy before import.

### Requirement: Support static/configured peers as the first discovery mode
r[molten.federation.static_discovery] Support static/configured peers as the first discovery mode.

### Requirement: Apply resource and rate-limit policy to announcements, inventory queries, and fetches
r[molten.federation.rate_limits] Apply resource and rate-limit policy to announcements, inventory queries, and fetches.

### Requirement: Represent sync status, imported resources, denials, and peer availability as local dataspace assertions
r[molten.federation.dataspace_status] Represent sync status, imported resources, denials, and peer availability as local dataspace assertions.

### Requirement: Integrate with remote artifact sync and content-addressed chunk-store fetch/verification
r[molten.federation.remote_sync_chunk_store] Integrate with remote artifact sync and content-addressed chunk-store fetch/verification.

### Requirement: Add loopback tests for signed announcement, pull fetch, verification, import, and denial
r[molten.federation.loopback_tests] Add loopback tests for signed announcement, pull fetch, verification, import, and denial.

### Requirement: Add Hegel property tests for receiver-driven sync, no-push-import, and verification-before-import invariants
r[molten.federation.property_tests] Add Hegel property tests for receiver-driven sync, no-push-import, and verification-before-import invariants.

