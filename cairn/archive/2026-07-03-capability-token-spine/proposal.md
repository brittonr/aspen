## Why

The new peering, subscriber/read-only, handoff, promotion, and Raft-membership boundaries all rely on capability-like evidence, but the current roadmap still spreads that idea across authority contexts, capability refs, live tickets, peer admissions, UCAN/Basalt seams, and subsystem grants. Molten needs one explicit capability-token spine so implementers do not treat arbitrary refs, imported artifacts, handoff bundles, transport observations, or peer sessions as authority.

## What Changes

- Define canonical capability token and capability proofset records that can wrap local grants, Basalt/UCAN proofs, peer subscription grants, promotion grants, authority grants, and future subsystem capability artifacts.
- Require privileged peer operations to resolve capability tokens through an admission receipt at use time, not by possession or import.
- Bind every token to holder/principal, peer/session or actor context, resource, ability/operation, scope, attenuation, caveats, expiry, revocation, policy refs, resource refs, issuer/delegation chain, and evidence refs.
- Add a clear taxonomy separating identity refs, evidence refs, transport receipts, handoff bundles, sessions, bootstrap tickets, authority tokens, read tokens, promotion tokens, and membership admission evidence.
- Preserve the Basalt/UCAN replacement path while supporting local deterministic fixtures for tests.

## Impact

- **Files**: authority/capability specs, peer/session specs, runtime spine docs, token admission core, diagnostics, and positive/negative tests.
- **Testing**: positive token admission fixtures and negative tests for bearer-only use, wrong holder, wrong session, wrong operation, over-broad scope, expired token, revoked issuer/delegation, caveat failure, missing policy/resource, token import as authority, and handoff/session/transport-as-token attempts.
