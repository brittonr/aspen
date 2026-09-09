# Preserve under-replicated status without targets

## Why

F07 demonstrates false completion at source revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.

The executed counterexample is `audit_insufficient_peers_remain_under_replicated`. Desired and minimum verified replicas equal two. Minimum fault domains equals one. Generation is 1, membership epoch is 2, and placement epoch is 3. The protected content has one present verified replica on `peer-a` in `zone-a`. The peer list contains only A, so no eligible target exists.

Expected: status retains the content as under-replicated and the execution receipt is partial. Actual core result: `desired=2`, `verified=1`, `decision=Ready`, `actions=[]`, and an empty under-replicated list. The plan contains `InsufficientPeers`.

The counterexample executed planner and status projection. Static review of the receipt classifier shows that this empty status selects `Complete`. No live service receipt or network execution ran in the audit.

## Proposed scope

Represent unmet demand independently from executable actions. Preserve per-content deficits through planning, execution, status, and receipt classification. An empty action list does not prove convergence.

Current consumers are the replication service and operator status. The Molten content-replication maintainers own the change. The durable capability is truthful bounded convergence reporting with repository-owned regression tests.

## Evidence and limits

The audit reports 359 passing core baseline tests. Its separate harness reports 41 passing controls and eight failing regression assertions. F07 is executed core evidence. Six other audit findings are static only.

This package records the complete trigger without ignored scratch dependencies. No commands or gates ran during planning. This proposal grants no implementation permission. It does not establish global availability, permanent durability, or release eligibility.
