## Context

Molten already expects Basalt/UCAN capabilities, Nickel and Steel contracts, evidence refs, gatekeeper resolution, and dataspace cleanup. This change makes the identity and authority rules explicit so every adapter and policy gate can speak the same language.

## Goals

- Provide canonical identity records for principals, nodes, actors, services, sessions, artifacts, and execution contexts.
- Keep identity separate from authority: an id alone grants nothing.
- Represent capabilities, delegations, attenuation, expiry, revocation, and key rotation as canonical artifacts/records with evidence refs.
- Ensure authority loss retracts dependent assertions, subscriptions, live refs, and handler bindings.
- Make authorization decisions replayable and auditable.

## Non-Goals

- Do not invent a global PKI or require one trust root for every deployment.
- Do not treat human-readable names as security identity.
- Do not allow actor ids, node ids, or artifact ids to imply access by themselves.
- Do not bypass Basalt/UCAN, policy contracts, or receipts.

## Identity model

Identity records should include:

- `principal_id`: human/operator/service/root authority subject.
- `node_id`: runtime node identity and key refs.
- `actor_id`: scoped actor identity under a node/runtime/session.
- `service_id`: long-lived service identity and current live actor/session refs.
- `session_id`: protocol, job, replay, or transcript session scope.
- `artifact_id`: content-addressed artifact identity.
- `execution_id`: one admitted run of an artifact with a handler profile.

All ids have canonical Preserves representations. Human names are metadata.

## Authority context

Every trust-boundary request carries an authority context:

- presented UCAN/Basalt capabilities,
- delegation chain and caveats,
- attenuation patterns or rewrite/filter rules,
- expiry and not-before bounds,
- revocation list/checkpoint refs,
- key ids and signature/proof refs,
- policy contract refs,
- prior receipt/evidence refs.

The authority context is hashed and referenced by admission receipts.

## Revocation and expiry

Revocation may target keys, principals, delegations, capabilities, live refs, handler bindings, sessions, or artifacts. Expiry is checked at admission time using the admitted clock source. When authority is lost, Molten retracts or disables dependent runtime state:

- dataspace assertions and Observe subscriptions,
- live gatekeeper refs,
- effect handler bindings,
- remote sync/execution permissions,
- storage access leases,
- catalog visibility,
- protocol/session participation.

Cleanup emits retraction and revocation receipts.

## Key rotation

Key rotation creates new key refs and links them to a principal/node under policy. Old keys can be deprecated, revoked, or retained for verifying historical receipts. Historical verification must not require old keys to retain current authority.

## Gatekeeper resolution

Gatekeepers resolve long-lived credentials into live scoped refs. Resolution is policy-gated and returns refs with scope, attenuation, expiry, revocation hooks, and evidence refs. Gatekeepers must not mint authority beyond the credential and policy.

## Replay

Recorded authority decisions must include enough context to replay deterministically. Replay can verify that the recorded decision followed the recorded policy/capability state, but it must not reuse expired/revoked authority for new side effects outside the replay scope.

## Open Questions

- What key formats and signature suites should be first-class in the first milestone?
- Should revocation state be local metadata first or Raft-backed control-plane state from the start?
- How should offline peers receive revocation updates without leaking private capability graphs?
