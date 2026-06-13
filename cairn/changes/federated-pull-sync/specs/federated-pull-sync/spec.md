# federated pull sync Delta Spec

## ADDED Requirements

### Requirement: System MUST Define signed announcement and inventory records for federated resources
r[molten.federation.announcement_model] The system MUST Define signed announcement and inventory records for federated resources.

### Requirement: System MUST Define initial federated resource types for artifacts, chunk manifests/chunks, docs/catalog metadata, receipts, provenance, transcripts, protocols, and schemas
r[molten.federation.resource_types] The system MUST Define initial federated resource types for artifacts, chunk manifests/chunks, docs/catalog metadata, receipts, provenance, transcripts, protocols, and schemas.

### Requirement: System MUST Document that federation does not imply global Raft, global dataspace, or pushed state
r[molten.federation.no_global_consistency] The system MUST Document that federation does not imply global Raft, global dataspace, or pushed state.

### Requirement: System MUST Emit receipts for discovery, query, fetch, verification, admission, import, denial, and merge
r[molten.federation.receipts] The system MUST Emit receipts for discovery, query, fetch, verification, admission, import, denial, and merge.

### Requirement: System MUST Implement receiver-driven query, missing-set calculation, fetch plan, verification, and local admission flow
r[molten.federation.pull_workflow] The system MUST Implement receiver-driven query, missing-set calculation, fetch plan, verification, and local admission flow.

### Requirement: System MUST Verify origin signatures, delegate/capability signatures, content/chunk hashes, and local policy before import
r[molten.federation.verification_layers] The system MUST Verify origin signatures, delegate/capability signatures, content/chunk hashes, and local policy before import.

### Requirement: System MUST Support static/configured peers as the first discovery mode
r[molten.federation.static_discovery] The system MUST Support static/configured peers as the first discovery mode.

### Requirement: System MUST Apply resource and rate-limit policy to announcements, inventory queries, and fetches
r[molten.federation.rate_limits] The system MUST Apply resource and rate-limit policy to announcements, inventory queries, and fetches.

### Requirement: System MUST Represent sync status, imported resources, denials, and peer availability as local dataspace assertions
r[molten.federation.dataspace_status] The system MUST Represent sync status, imported resources, denials, and peer availability as local dataspace assertions.

### Requirement: System MUST Integrate with remote artifact sync and content-addressed chunk-store fetch/verification
r[molten.federation.remote_sync_chunk_store] The system MUST Integrate with remote artifact sync and content-addressed chunk-store fetch/verification.

### Requirement: System MUST Add loopback tests for signed announcement, pull fetch, verification, import, and denial
r[molten.federation.loopback_tests] The system MUST Add loopback tests for signed announcement, pull fetch, verification, import, and denial.

### Requirement: System MUST Add Hegel property tests for receiver-driven sync, no-push-import, and verification-before-import invariants
r[molten.federation.property_tests] The system MUST Add Hegel property tests for receiver-driven sync, no-push-import, and verification-before-import invariants.

