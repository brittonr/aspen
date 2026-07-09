## Why

Main's Iroh identity persistence guidance is worth adapting for Molten. Molten already has persistent node identity requirements, but the Iroh endpoint secret-key path needs a concrete reviewed boundary so restarts keep stable endpoint ids without leaking private key material or treating transport identity as authority.

Molten should define a pure identity-resolution core and shell-owned secret backend/file behavior that binds endpoint public identity into startup and peer evidence while keeping private key material out of receipts, logs, and replay bundles.

## What Changes

- Define the Iroh endpoint identity backend model: explicit key, managed secret backend, persisted node-state file, generate-and-persist, and fail-closed denial.
- Require owner-only file permissions where the platform supports them and secret-safe diagnostics where it does not.
- Detect endpoint-id drift and require admitted rotation or recovery evidence before accepting a changed identity for an existing node scope.
- Bind endpoint public identity refs into startup, replay, and peer-bootstrap evidence while preserving the rule that endpoint identity is transport evidence only.
- Add positive and negative fixtures for restart stability, malformed keys, permission failures, drift, redaction, rotation, and transport-as-authority denial.

## Impact

- **Files**: persistent-node-identity specs, peer-bootstrap docs where identity refs are consumed, node startup shells, production profile contracts, tests, and operator diagnostics.
- **Testing**: positive restart/reload fixtures; negative malformed secret, missing secret backend, wrong file permissions, identity drift, stale rotation, receipt secret leakage, and endpoint-id-as-authority fixtures.
- **Security**: private endpoint keys never appear in receipts or logs. Endpoint public identity supports transport continuity and peer binding, but does not grant capabilities, authority, policy, resource, provenance, source-gate, retention, or execution trust.