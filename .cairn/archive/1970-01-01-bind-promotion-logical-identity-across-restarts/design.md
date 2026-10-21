# Promotion identity design

## Boundary and source

`LocalWorldPromotionStore` persists the successor head, promotion record, and reservation set atomically. The dispatch path already distinguishes committed eligibility, attempted dispatch, and observed external completion. The gap is representational: the record does not carry a stable logical operation identity that survives restart, and nothing binds a canonical request to that identity.

## Proposed decision

The pure promotion core owns the four identity roles. The logical operation identity is minted once at the origin before the first submission and is persisted before any effect. The canonical request identity is a domain-separated BLAKE3 binding of the exact parameters. The attempt identity changes per transmission. The generation identity comes from the existing fencing mechanism.

Replay rules, all evaluated in the pure core:

- Same logical identity, same canonical request identity: replay of the original operation; return the retained outcome or retained uncertainty.
- Same logical identity, different canonical request identity: typed rejection; no state mutation.
- Different logical identity, same canonical request identity: two distinct operations; no merge.
- Unknown logical identity after restart: treated as a new operation only when the caller supplies a fresh logical identity; the shell never substitutes one.

`OutcomeUnknown` remains a first-class outcome. A lost response stores the logical and attempt identities with an unresolved outcome, so a post-restart retry can report the original uncertainty instead of inventing success or failure.

For effects executed inside an authoritative transactional state machine, deduplication prevents a second application. For external destinations, the contract preserves uncertainty and defers to the destination's reconciliation support; this package records no exactly-once claim.

## Compatibility and replay

Prefer unchanged Redb schema where possible; add fields to the promotion record with explicit unknown-tolerant reads for records written before this change. Old records without a canonical request identity are replayed by logical identity alone and keep their historical outcomes. Replay tests keep logical identity, canonical request identity, and outcomes stable across reopen; stored historical receipts are never rewritten.

## Coordination and order

`bind-delivery-duplicate-results-to-original-claims` owns delivery duplicate responses; no shared generic history abstraction is introduced here. `bind-scheduler-completions-to-lease-epoch` owns the scheduler fencing token; this change consumes generation identity from the existing fencing source. `add-world-head-rollback-witnessing` owns head authority; content identity is not authority here.

## Validation and ownership

Promotion-path maintainers own the core, store, adapter, and docs. Move the restart replay into normal repository tests with named identities and bounded storage. Cover both same-identity paths, both mismatch rejections, `OutcomeUnknown` retention across reopen, malformed records, and expired-generation rejection. Run focused Octet and Clippy checks, workspace tests, relevant Nix checks, and required Cairn gates without weakening checks. This plan grants no implementation permission and claims no exactly-once external semantics.
