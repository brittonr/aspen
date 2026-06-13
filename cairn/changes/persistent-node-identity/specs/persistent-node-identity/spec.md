# persistent node identity Delta Spec

## ADDED Requirements

### Requirement: Define canonical node identity records with node id, endpoint public key, key source class, data-dir/secret-backend ref, policy refs, and receipt refs
r[molten.node_identity.record_model] Define canonical node identity records with node id, endpoint public key, key source class, data-dir/secret-backend ref, policy refs, and receipt refs.

### Requirement: Implement explicit-key, persisted-file, generate-and-persist, and deny-if-unavailable resolution order
r[molten.node_identity.resolution_order] Implement explicit-key, persisted-file, generate-and-persist, and deny-if-unavailable resolution order.

### Requirement: Document and enforce that stable node identity grants no capabilities by itself
r[molten.node_identity.no_identity_authority] Document and enforce that stable node identity grants no capabilities by itself.

### Requirement: Ensure startup receipts never expose private key material
r[molten.node_identity.secret_redaction] Ensure startup receipts never expose private key material.

### Requirement: Persist generated endpoint secret material to configured node data directory with restricted permissions where supported
r[molten.node_identity.file_backend] Persist generated endpoint secret material to configured node data directory with restricted permissions where supported.

### Requirement: Detect endpoint id drift and deny or require admitted recovery/key-rotation policy
r[molten.node_identity.drift_detection] Detect endpoint id drift and deny or require admitted recovery/key-rotation policy.

### Requirement: Emit receipts for identity resolution, first boot generation, load, drift, denial, and rotation
r[molten.node_identity.startup_receipts] Emit receipts for identity resolution, first boot generation, load, drift, denial, and rotation.

### Requirement: Validate node identity config through Nickel/static policy before startup side effects
r[molten.node_identity.config_contract] Validate node identity config through Nickel/static policy before startup side effects.

### Requirement: Include node identity refs in peer bootstrap handshakes and join admission
r[molten.node_identity.peer_bootstrap] Include node identity refs in peer bootstrap handshakes and join admission.

### Requirement: Include node identity refs in replay/startup evidence without requiring private key access
r[molten.node_identity.replay_refs] Include node identity refs in replay/startup evidence without requiring private key access.

### Requirement: Add tests proving restart with the same data dir preserves endpoint id
r[molten.node_identity.restart_tests] Add tests proving restart with the same data dir preserves endpoint id.

### Requirement: Add Hegel property tests for resolution determinism and no-secret-in-receipt invariants
r[molten.node_identity.property_tests] Add Hegel property tests for resolution determinism and no-secret-in-receipt invariants.

