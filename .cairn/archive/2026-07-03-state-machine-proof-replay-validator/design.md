# Design: state-machine proof replay validator

## Scope

This change defines a cross-cutting proof trace validator for state-machine evidence. It should validate in-memory trace records that bind before-state refs, command or transition refs, after-state refs, predicate/check names, decisions, diagnostics, and receipt refs.

## Proof checklist

- **Proof claim**: a proof trace is accepted only when each step's receipt and state refs are internally consistent and adjacent steps chain correctly.
- **Out of scope**: proving external adapter durability, network consensus liveness, or cryptographic security beyond canonical content refs.
- **Trusted assumptions**: individual receipt validators for lifecycle, coordination, runtime predicates, and control-plane receipts are correct for their own schemas.
- **Positive evidence**: a valid trace with passing and denying steps replays and produces a deterministic validation summary.
- **Negative evidence**: missing receipts, tampered diagnostics, stale before-state refs, wrong after-state refs, and out-of-order steps fail closed.
- **Canonical refs**: trace validation binds BLAKE3 content refs for inputs, receipts, and state snapshots.
- **Regeneration command**: focused replay/harness tests plus `cargo test replay` or the smallest existing replay validation command.

## Functional core

The validator core should accept parsed in-memory records and return a deterministic validation result. File discovery, fixture loading, JSON output, CLI printing, and ledger import remain in the imperative shell.

## Non-goals

- No PR automation.
- No new state-machine semantics; this validates evidence produced by existing transition gates.
