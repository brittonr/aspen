## Why

Evidence chains are append-only state machines over heads, links, payload refs, predicates, and checkpoints. They must prove continuity: idempotent append preserves an existing head, a valid append advances exactly one head, stale observed heads deny, and verification catches gaps or forks.

## What Changes

- Add proof requirements for chain head transitions and append receipts.
- Require generated chain segments covering genesis, append, idempotent append, stale head denial, gap denial, fork denial, and checkpoint preservation.
- Require retention/GC proof that anchored chain evidence remains reachable.

## Impact

- **Files**: evidence chain append/verify logic, ledger chain tests, retention interactions if needed.
- **Testing**: positive linear chains, negative stale/gap/fork/tamper cases, checkpoint verification, and generated chain segments.
