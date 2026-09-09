# Proposal: Fence runtime-local work across restarts

## Why

Static review of `crates/molten-core/src/system_extension/lifecycle.rs` and `dispatch.rs` at `6c7158db6` found that an ordinary `BeginRestart` increments `restart_attempts` but preserves `generation`, while `CallbackEvent` and `TypedEffectRequest` carry only `generation` and validate it against the active generation. Across a same-generation restart that check is a no-op: a delayed timer result, callback completion, or failure notification produced by the previous execution instance still validates against the replacement instance.

The addressable-actor layer partially mitigates this: `addressable_actor/transition.rs` rejects requests whose `expected_lifecycle_sequence` does not match the actor state, and the sequence advances per planned turn. But no equivalent binding exists at the system-extension callback and typed-effect boundary, and no conformance test currently proves that restart paths always advance the fencing token before old runtime-local work can act.

This mirrors the same identity root cause already tracked for the reference scheduler (`bind-scheduler-completions-to-lease-epoch`) and for promotion (`bind-promotion-logical-identity-across-restarts`); it closes the gap at the lifecycle and dispatch boundary.

## What Changes

- Establish three explicit identities at the system-extension boundary: the service key, the implementation generation, and a new runtime incarnation that changes on every restart, replacing the overloaded use of generation for that purpose.
- Add a conformance test, before any counter is added, that starts an instance, creates pending runtime-local work, restarts without changing implementation, and proves that the old instance's timer, callback completion, and failure notification cannot act on the replacement.
- If the test shows existing lifecycle-sequence propagation already provides the guarantee, document and enforce that propagation; otherwise introduce an explicit incarnation token bound to runtime-local messages, timers, continuations, and effect completions.
- Require that durable logical work addressed to the service survives restart only through explicit readmission under the replacement instance and a current delivery claim, never as a still-valid callback from the old instance.
- Keep placement fencing and delivery-attempt tokens separate; do not collapse them into the incarnation token.

## Impact

- **Files**: `molten-core` system-extension lifecycle and dispatch models, addressable-actor transition fixtures that exercise restart, and related tests.
- **Testing**: restart-fencing conformance trace, stale-incarnation rejection, durable-work readmission, and negative tests for delayed callback, timer, and failure-notification delivery across restart.
- **Non-goals**: no change to delivery-claim or placement-fencing semantics, no upgrade-path change (generation transitions already re-fence), and no claim about production live adapters beyond what conformance tests exercise.

## Dependencies

- `bind-scheduler-completions-to-lease-epoch` shares the identity root cause; the incarnation token design should reuse its domain-separated token discipline.
- `addressable-actor-runtime` lifecycle-sequence propagation is the candidate existing mechanism this change evaluates.
