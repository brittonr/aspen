## Why

Molten has the right security boundary for peering: transport identity, live tickets, peer admission, authority grants, policy, resources, and replay evidence are separate. The operator experience is still scattered across ticket export/import, peer admission, authority import, bundle verification, live send, and ad hoc diagnostics. Operators need one durable peer/session lifecycle that explains what is known, what is admitted, what is missing, and what remains evidence-only without turning a connected transport peer into authority.

## What Changes

- Define canonical `peer-profile-v1` and `peer-session-v1` records that carry peer identity, endpoint scope, negotiated joins, admitted capabilities, resource bounds, freshness, revocation state, and evidence refs.
- Add an explicit peer lifecycle: discovered, invited, handshaking, negotiated, admitted, connected, expired, revoked, and quarantined.
- Store a node-local peer read model under the state root so `peer status` and `peer diagnose` can report current evidence without replacing canonical receipts.
- Add typed Nickel contracts for static peer configuration and operator-reviewed peer profiles.
- Introduce `molten peer invite|connect|status|revoke|diagnose` UX over the existing live ticket, peer admission, and authority import boundaries.

## Impact

- **Files**: peer bootstrap/session core, node state read model, CLI peer commands, Nickel peer config contracts, docs/runbooks, and positive/negative tests.
- **Testing**: lifecycle transition tests, missing/stale/mismatched evidence denials, typed peer config fixture tests, and CLI diagnostics coverage.
- **Security**: preserves the existing rule that transport identity, peer sessions, and status readbacks do not grant authority, policy, provenance, resource, source-gate, retention, or execution trust.
