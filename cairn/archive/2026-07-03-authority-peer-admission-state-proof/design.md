# Design: authority and peer admission state proof

## Scope

This change proves authority and live peer admission state machines. It covers authority context evaluation, delegation/attenuation, expiry, revocation, key currentness, authority grant import, live ticket export/import, peer admission, and transport-neutral bootstrap checks.

## Proof checklist

- **Proof claim**: current authority is admitted only when scope, capability, attenuation, epoch, expiry, revocation, key, peer, node, topic, and policy checks pass; identity, ticket possession, transport observation, and imported artifacts remain non-authorizing by themselves.
- **Out of scope**: cryptographic algorithm proofs and real-world identity vetting outside canonical receipt inputs.
- **Trusted assumptions**: signature/keyring verification and canonical hash routines are stable.
- **Positive evidence**: scoped authority and peer admission traces pass with matching principal/node/peer/topic/operation/scope refs and current revocation/key evidence.
- **Negative evidence**: wrong scope, wrong peer, wrong node, stale epoch, expired grant, revoked key/delegation, missing admission, mismatched ticket, and transport-only evidence deny.
- **Canonical refs**: authority context refs, grant refs, import refs, key/revocation refs, peer ticket refs, peer admission refs, node/topic/endpoint refs, policy/resource refs, and diagnostics.
- **Regeneration command**: `cargo test authority peer node`.

## Functional core

Keep admission decisions as pure predicates over authority context records, peer ticket/admission records, revocation/key state, and requested operation scope. Import and transport adapters only supply candidate evidence.

## Non-goals

- No new authority semantics beyond existing grant, delegation, ticket, and admission models.
- No trust promotion from live transport or neighbor observations.
