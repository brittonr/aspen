## Why

Generation-fenced local head state rejects stale claims while that state remains intact. It cannot detect an attacker who restores both the branch head and its local generation store to an older valid image.

The workspace release-channel change is selecting a shared transparency or witness owner. Molten needs a consumer contract for independent currentness without creating a private log protocol.

## What Changes

- Add a strong rollback-resistance profile that requires admitted independent witness observations.
- Define Molten adapter DTOs for append, inclusion, consistency, signed checkpoint, quorum, unavailable, and fork observations.
- Stage head claims before external append, then finalize the local head only after admitted witness evidence returns.
- Persist the highest admitted witness checkpoint and branch generation with the local head transition.
- Reject stale, inconsistent, unavailable, forked, wrong-provider, wrong-branch, and insufficient-quorum observations.
- Keep the existing local-only profile with an explicit weaker currentness claim.
- Reconcile witnessed claims whose final local compare-and-swap outcome is uncertain.
- Emit detached receipts that separate head authentication, branch authorization, witness currentness, and local persistence.

## Dependencies

- `add-world-branch-head-protocol`.
- The ownership and provider contract selected by `workspace-lifecycle/.cairn/changes/establish-governed-release-channels`.
- Artifact Auth, Durable Authority State, Transactional Reconciliation Core, Basalt, and UCAN.

## Non-Goals

- A Molten-owned transparency log, witness network, consensus protocol, or trust-root discovery system.
- Proof that a provider is honest, globally available, fork-free, or correctly operated.
- Atomic commitment across the local store and an external provider.
- Automatic repair after fork, missing state, or witness disagreement.

## Impact

- **Core**: witness profiles, normalized observations, currentness decisions, quorum checks, finalize plans, and reconciliation classes.
- **Shell**: provider adapters, staged claims, append requests, durable checkpoint state, final head transactions, and recovery.
- **Schemas**: canonical witness observations, admission receipts, and reconciliation records.
- **Testing**: valid witnessed advances plus negative whole-store rollback, stale checkpoint, consistency failure, fork, split quorum, provider substitution, unavailable provider, uncertain finalization, and witness-as-authority cases.
