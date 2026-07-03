## Why

Authority grants, delegation imports, peer admissions, and live tickets form trust-bearing state machines. Transport identity, ticket possession, imported grants, or historical receipts must never become current authority unless every scope, epoch, revocation, freshness, and policy check passes.

## What Changes

- Add authority-state proof requirements for grants, delegation imports, revocation, expiry, and attenuation.
- Add peer-admission proof requirements for live tickets, peer admissions, and transport-neutral bootstrap evidence.
- Require negative traces for revoked, expired, wrong-scope, wrong-peer, stale-epoch, and transport-only evidence.

## Impact

- **Files**: authority admission, peer bootstrap, node live peer/ticket import, authority grant import, and tests.
- **Testing**: valid scoped admission, wrong-scope denial, revoked/expired denial, ticket mismatch denial, and proof that transport observations do not grant authority.
