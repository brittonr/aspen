# Persistent Node Identity Specification

## Purpose

Defines the `persistent-node-identity` capability.

## Requirements

### Requirement: System MUST Define canonical node identity records with node id, endpoint public key, key source class, data-dir/secret-backend ref, policy refs, and receipt refs
r[molten.node_identity.record_model] The system MUST Define canonical node identity records with node id, endpoint public key, key source class, data-dir/secret-backend ref, policy refs, and receipt refs.

### Requirement: System MUST Implement explicit-key, persisted-file, generate-and-persist, and deny-if-unavailable resolution order
r[molten.node_identity.resolution_order] The system MUST Implement explicit-key, persisted-file, generate-and-persist, and deny-if-unavailable resolution order.

### Requirement: System MUST Document and enforce that stable node identity grants no capabilities by itself
r[molten.node_identity.no_identity_authority] The system MUST Document and enforce that stable node identity grants no capabilities by itself.

### Requirement: System MUST Ensure startup receipts never expose private key material
r[molten.node_identity.secret_redaction] The system MUST Ensure startup receipts never expose private key material.

### Requirement: System MUST Persist generated endpoint secret material to configured node data directory with restricted permissions where supported
r[molten.node_identity.file_backend] The system MUST Persist generated endpoint secret material to configured node data directory with restricted permissions where supported.

### Requirement: System MUST Detect endpoint id drift and deny or require admitted recovery/key-rotation policy
r[molten.node_identity.drift_detection] The system MUST Detect endpoint id drift and deny or require admitted recovery/key-rotation policy.

### Requirement: System MUST Emit receipts for identity resolution, first boot generation, load, drift, denial, and rotation
r[molten.node_identity.startup_receipts] The system MUST Emit receipts for identity resolution, first boot generation, load, drift, denial, and rotation.

### Requirement: System MUST Validate node identity config through Nickel/static policy before startup side effects
r[molten.node_identity.config_contract] The system MUST Validate node identity config through Nickel/static policy before startup side effects.

### Requirement: System MUST Include node identity refs in peer bootstrap handshakes and join admission
r[molten.node_identity.peer_bootstrap] The system MUST Include node identity refs in peer bootstrap handshakes and join admission.

### Requirement: System MUST Include node identity refs in replay/startup evidence without requiring private key access
r[molten.node_identity.replay_refs] The system MUST Include node identity refs in replay/startup evidence without requiring private key access.

### Requirement: System MUST Add tests proving restart with the same data dir preserves endpoint id
r[molten.node_identity.restart_tests] The system MUST Add tests proving restart with the same data dir preserves endpoint id.

### Requirement: System MUST Add Hegel property tests for resolution determinism and no-secret-in-receipt invariants
r[molten.node_identity.property_tests] The system MUST Add Hegel property tests for resolution determinism and no-secret-in-receipt invariants.
