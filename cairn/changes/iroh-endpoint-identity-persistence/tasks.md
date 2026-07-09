## Tasks

- [ ] [serial] r[molten.node_identity.iroh_secret_backend_model] Inventory current node startup identity inputs, state-root layout, profile fields, and Iroh endpoint key handling.
- [ ] [serial] r[molten.node_identity.iroh_identity_resolution_core] Define a pure resolution core for explicit key, secret backend, persisted file, generate-and-persist, rotation, and fail-closed denial decisions.
- [ ] [parallel] r[molten.node_identity.iroh_identity_permissions] Wire shell-owned secret file/backend handling with owner-only permissions where supported and deterministic diagnostics where unsupported.
- [ ] [parallel] r[molten.node_identity.iroh_identity_drift_recovery] Add drift detection and admitted rotation/recovery receipt requirements for changed endpoint public keys under the same node scope.
- [ ] [parallel] r[molten.node_identity.iroh_identity_receipt_redaction] Ensure startup, replay, peer-bootstrap, and rotation receipts bind public identity refs without exposing private key bytes, bearer tokens, raw tickets, or sensitive paths.
- [ ] [serial] r[molten.testing.iroh_identity_positive_negative_fixtures] Add positive fixtures for first boot, restart stability, explicit-key precedence, secret-backend load, and admitted rotation plus negative fixtures for malformed keys, unsafe permissions, drift without rotation, stale rotation, secret leakage, and endpoint-id-as-authority.
- [ ] [serial] r[molten.testing.iroh_identity_positive_negative_fixtures] Update operator docs and run focused identity/startup tests plus Cairn validation.