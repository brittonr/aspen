## Context

Live node-control workflows already bind tickets, peer admissions, authority grants, send/receive receipts, listener receipts, and workflow bundles. The missing layer is a reviewed declaration of the intended live topology and transport profile so operators can avoid restating the same peer/topic/endpoint assumptions for every command.

## Decisions

### Topology profiles constrain expected live identities

**Choice:** A topology profile declares node ids, peer ids, topics, endpoint expectations, admitted ticket refs, peer-admission refs, allowed ALPN surfaces, and optional role names.

**Rationale:** Live workflows should fail closed when the command is pointed at the wrong peer, topic, endpoint, or protocol surface.

### Transport profiles own live retry and timeout policy

**Choice:** A transport profile declares retry attempts, join/publish timeout bounds, relay/direct preferences, and diagnostic expectations, admitted through runtime limit-profile hard caps.

**Rationale:** Operators need environment-specific live behavior without hard-coding timeouts in command scripts.

### Profiles feed preflight, not authority

**Choice:** Topology/transport profiles are checked before live operations and their refs are recorded in receipts, but authority, policy, resource, provenance, source-gate, retention, and capability gates remain independent.

**Rationale:** Knowing the intended peer or transport does not authorize the operation.

### CLI profile use is optional and incremental

**Choice:** Add profile inputs first to live node-control surfaces where repeated peer/topic/ticket guards already exist. Existing explicit flags remain supported.

**Rationale:** This avoids a compatibility break while giving operators a safer reviewed path.

## Validation strategy

- Pure tests for topology guard matching and denial diagnostics.
- Positive live-loopback/profile tests for matching peer/topic/endpoint.
- Negative tests for wrong peer, wrong topic, unsupported ALPN, stale ticket/admission, and transport-as-authority misuse.
- Receipt tests that bind topology and transport profile refs.

## Non-claims

A live topology or transport profile does not prove network availability, delivery success, peer authority, policy admission, or source correctness. It constrains intended live operation inputs only.
