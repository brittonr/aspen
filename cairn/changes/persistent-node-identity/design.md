## Context

Aspen persists the Iroh secret key to keep EndpointId stable across restarts. Molten needs the same operational property, but represented in its own authority/evidence model. Node identity is foundational for peer bootstrap, catalog visibility, remote artifact sync, receipts, and deterministic replay.

## Goals

- Preserve node endpoint identity across restarts.
- Make key source and identity resolution auditable.
- Support explicit deployment-provided keys and generated persisted keys.
- Detect endpoint drift and require policy for replacement.
- Avoid treating node identity as ambient authority.

## Non-Goals

- Do not make all nodes globally trusted just because they have stable keys.
- Do not store secret keys in the artifact registry or public catalog.
- Do not require one global PKI for every deployment.
- Do not copy Aspen's exact config/API; use Molten DTOs and receipts.

## Resolution order

Node endpoint key resolution should be deterministic and explicit:

1. Explicit key reference or secret backend configured by policy.
2. Existing key file in the configured node data directory.
3. Generate a new key and persist it if policy admits first boot.
4. Deny startup if persistence is required but unavailable.

The selected source is recorded in a startup receipt without exposing secret material.

## Identity record

A node identity record should include:

- node id and display name metadata,
- endpoint public key / Iroh endpoint id,
- key source class, not secret bytes,
- data directory or secret backend reference,
- creation or first-seen logical time where available,
- policy refs and authority context,
- receipt refs.

## Drift handling

If a persisted node has a prior endpoint id and resolves to a different endpoint id, Molten should either deny startup or require an explicit key-rotation/recovery policy. Drift is a trust-boundary event and emits receipts.

## Security

Local key files should be created with owner-only permissions where the platform supports it. Production deployments should prefer encrypted storage or a secrets backend. Key backup and recovery policy belongs to operators and must be visible in receipts.

## Integration

Peer bootstrap uses persistent node identity in handshakes. Authority/revocation handles key rotation and revocation. Deterministic replay references identity records but does not require access to private keys unless replaying signing effects under an admitted secret fixture.

## Open Questions

- Which secret backend should be supported first besides filesystem?
- Should key rotation be denied by default until authority/revocation lands?
- How should node id, endpoint id, and principal id be related in minimal deployments?
