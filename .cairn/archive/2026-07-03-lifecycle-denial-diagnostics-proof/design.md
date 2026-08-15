# Design: lifecycle denial diagnostics proof

## Scope

This change covers lifecycle denial evidence. It focuses on the pure diagnostic function, receipt decision law, malformed transition input handling, and stable diagnostic ordering.

## Proof checklist

- **Proof claim**: every lifecycle denial names the failed transition predicate deterministically, and any non-empty diagnostics vector results in a `deny` receipt.
- **Out of scope**: runtime shells that decide whether to attempt a transition, external policy engines, and adapter cleanup side effects.
- **Trusted assumptions**: canonical ref validation errors remain part of input validation rather than transition predicate diagnostics.
- **Positive evidence**: valid transitions have no diagnostics and pass.
- **Negative evidence**: invalid jumps, action-target mismatches, malformed refs, empty entity ids, and empty causes fail with stable errors or diagnostics.
- **Canonical refs**: denial receipts bind the canonical transition ref whenever input validation succeeds.
- **Regeneration command**: `cargo test lifecycle`.

## Diagnostic law

The diagnostic core should remain deterministic and bounded. If multiple transition predicates fail, diagnostics should appear in stable order so receipt refs remain stable across replay.

## Non-goals

- No localization or free-form diagnostic rewriting.
- No side-effect rollback implementation; lifecycle transition receipts remain pure evidence.
