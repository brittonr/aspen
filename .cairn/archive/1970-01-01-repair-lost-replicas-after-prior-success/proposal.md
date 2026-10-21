# Repair lost replicas after prior success

## Why

F04 demonstrates that historical transfer success can replace fresh availability evidence. The reviewed source revision is `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.

The executed counterexample is `audit_old_transfer_success_does_not_override_current_absence`. The fixture requires two verified replicas across two fault domains. Generation is 1, membership epoch is 2, and placement epoch is 3. Inventory contains protected, verified content only on `peer-a` in `zone-a`. Eligible `peer-b` occupies `zone-b`.

The first plan selects a transfer to B. The counterexample saves that operation as `Verified`, then advances observed tick from 1 to 2. Inventory still contains only A. This models loss at B without an epoch change.

Expected: repair needs new verification, or status retains unresolved demand. Actual: the planner emits `Reuse`, status reports two verified replicas, and the under-replicated list is empty. Static shell review shows that `Reuse` skips fetch and verification. The audit did not execute a live transfer or data loss.

## Proposed scope

Separate historical operation results from current replica availability. Preserve exact-operation replay without treating it as proof that bytes remain present. Keep bounded repair, epoch fencing, protected content, and retention admission intact.

Current consumers are the content-replication service and its operator status. The Molten content-replication maintainers own this change. The durable capability is repeatable repair after same-epoch loss, with repository-owned regression tests.

## Evidence and limits

The audit reports 359 passing core baseline tests. Its separate harness reports 41 passing controls and eight failing regression assertions. F04 is one executed counterexample. Six other audit findings have static evidence only.

This package records the trigger without dependence on ignored scratch files. Implementation must move the reproduction into normal tests. This proposal records no new product execution evidence. No full-workspace, Octet, Clippy, Nix, or lifecycle acceptance claim follows from the audit.

This proposal grants no implementation permission. It does not prove permanent availability, remote durability, deletion authority, or release eligibility.
