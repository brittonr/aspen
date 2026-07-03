# Design: peer session lifecycle

## Scope

This change turns scattered peering evidence into a first-class lifecycle and read model. It does not weaken existing admission gates. The canonical receipts remain the source of truth for authority, policy, resources, provenance, source gates, replay/idempotency, retention, and execution.

## Peer records

`peer-profile-v1` is an operator-reviewed declaration for a peer or peer class. It names node/peer ids, expected endpoint ids or ticket issuers, allowed transports, admitted join kinds, topics/docs/job pools, resource bounds, expiry policy, revocation policy, and policy refs.

`peer-session-v1` is the live/readback state derived from receipts. It binds the profile ref, local node identity ref, remote identity/admission refs, negotiated agreement/session refs, live ticket refs, admitted scopes, optional authority grant refs, resource refs, freshness, lifecycle state, revocation/quarantine markers, and diagnostics.

## Lifecycle

The pure lifecycle reducer accepts prior session state plus canonical events and emits the next session state plus a transition receipt:

```text
discovered -> invited -> handshaking -> negotiated -> admitted -> connected
           -> expired
           -> revoked
           -> quarantined
```

Transitions are fail-closed. A node may record observations such as neighbor events, last-seen endpoints, and failed sends, but observations alone never advance a peer into `admitted` or grant authority.

## Node read model

The node state root keeps a bounded peer table for operator lookup. It indexes by peer id, node id, profile ref, ticket ref, admission ref, and active scopes. The read model can summarize current state and next missing evidence, but every pass decision still names canonical receipt refs.

## CLI UX

`molten peer invite create` emits a profile-bound invite artifact. `molten peer invite accept` converts an invite plus local identity into handshake material. `molten peer connect` resolves tickets/admissions and records a session. `molten peer status` renders the read model. `molten peer diagnose` recomputes diagnostics over canonical receipts and prints missing next steps. `molten peer revoke` writes revocation/quarantine evidence and marks affected sessions unusable for future sends.

## Nickel config

Static peers use typed Nickel contracts for profile shape, allowed transports/scopes, resource bounds, expiry/revocation policy, and evidence refs. Runtime code consumes exported, checked config and must not evaluate Nickel during live node operations.

## Functional core

The core validates records, computes transitions, derives read-model updates, and produces diagnostics from in-memory canonical values. The CLI and node shell read/write files, import ledger artifacts, and render operator text.

## Non-goals

- No automatic trust from Iroh endpoint identity, neighbor observations, or topic membership.
- No implicit authority grant from a connected peer session.
- No Raft/control-plane membership join from ordinary peer admission.
- No global peer directory or fleet-wide discovery service in this slice.
