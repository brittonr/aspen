# Design: evidence-chain head transition proof

## Scope

This change proves the evidence-chain head transition state machine. It covers genesis links, append links, head-before/head-after refs, append receipts, predicate receipts, segment verification, checkpoint evidence, fork/gap detection, and anchor/retention preservation.

## Proof checklist

- **Proof claim**: chain append advances from a known head to exactly one new head when continuity checks pass; idempotent append preserves existing head evidence; stale heads, gaps, forks, tampered links, and missing predicate receipts deny.
- **Out of scope**: distributed consensus over chain heads and external signature trust beyond existing signed receipt verification.
- **Trusted assumptions**: canonical chain link hashing and payload refs are stable.
- **Positive evidence**: generated linear chains verify with no gaps, append receipts bind head-before/head-after, and checkpoints bind verified ranges.
- **Negative evidence**: stale observed heads, forked heads, missing intermediate links, tampered payload refs, duplicate sequence conflicts, and missing predicate receipts fail closed.
- **Canonical refs**: proof traces bind chain scope/id/epoch, prior head refs, appended link refs, payload refs, predicate receipt refs, append receipt refs, checkpoint refs, and segment verify refs.
- **Regeneration command**: `cargo test chain` or focused `cargo test ledger`/`cargo test evidence` command available in the repo.

## Functional core

Continuity checks should be pure over in-memory chain links and receipts. Ledger storage, import/export, and retention shell code should not decide continuity outside the proof core.

## Non-goals

- No claim that a remote peer has all links unless fetch/import evidence proves it.
- No replacement for signed receipt keyring verification.
