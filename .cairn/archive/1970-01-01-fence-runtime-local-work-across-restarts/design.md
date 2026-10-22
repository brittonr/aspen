# Design: Fence runtime-local work across restarts

## Context

`LifecycleState` carries `generation`, `phase`, `restart_attempts`, `health`, and `checkpoint_ref`. `BeginRestart` at `lifecycle.rs:284-298` clones the state, bumps `restart_attempts`, and preserves `generation`. `CallbackEvent` (`dispatch.rs:18-22`) and `TypedEffectRequest` carry only `generation`; `dispatch.rs:129` rejects stale generations, which cannot distinguish instances across a same-generation restart. `CallbackInvocation` adds a dispatch-assigned `sequence` but no instance identity of its own.

The addressable-actor profile (`addressable_actor/transition.rs:97`) binds each request to `expected_lifecycle_sequence`, which increments per planned turn in `transition/support.rs:65`. Whether every restart path is forced through a sequence-advancing turn before old runtime-local work can act is exactly what the conformance test must establish.

## Approach

1. **Conformance test first.** Add a pure deterministic trace: install and start instance A, create pending runtime-local work (timer, callback completion, failure notification) addressed to A, apply `BeginRestart` without changing implementation, then deliver each delayed item. The test asserts none can act on the replacement instance. Record the verdict:
   - If all deliveries are already rejected through lifecycle-sequence or equivalent propagation, the change documents that mechanism, adds the regression, and stops.
   - If any delivery is accepted, proceed with the explicit incarnation token.
2. **Incarnation token.** Add a monotonic `incarnation` counter to `LifecycleState`, incremented on every restart-entry transition (`BeginRestart`, and recovery restarts). `CallbackEvent` and `TypedEffectRequest` gain an `incarnation` field; dispatch validates generation and incarnation together. Tokens use the same domain-separated BLAKE3 encoding discipline as the scheduler assignment token in `bind-scheduler-completions-to-lease-epoch`.
3. **Durable readmission.** Durable logical work (mailbox records, durable semantic events) is not fenced out; it is re-admitted deliberately: after restart, the shell presents durable work with a fresh delivery claim bound to the new incarnation. The transition rejects any runtime-local completion whose incarnation predates the claim's incarnation, and the mixed-up cases (old callback presented as durable work, durable work treated as live callback) get dedicated negative fixtures.
4. **Separation of concerns.** Placement fencing, delivery-attempt tokens, and the incarnation stay distinct fields with distinct validation rules. No field doubles for another.

## Alternatives considered

- Reuse `generation` for restart fencing by bumping generation on restart. Rejected: generation is the implementation/configuration identity and upgrade/rollback semantics depend on it changing only there; overloading it would break upgrade compatibility checks and receipt identity.
- Rely only on the addressable-actor `expected_lifecycle_sequence`. Rejected as sufficient on current evidence: the system-extension callback boundary is reachable without that check, and the conformance test must pin the guarantee at every boundary, not one.

## Non-claims

- Passing the conformance test does not prove whole-runtime correctness or that every host adapter propagates the token.
- Durable-work survival guarantees remain scoped to the delivery and durability contracts; this change only fences runtime-local work.

## Risks

- Adding a field to `CallbackEvent` and `TypedEffectRequest` breaks constructors in `tests.rs` and fixtures; all call sites update in the same change.
- If the conformance test exposes that restarts skip sequence-advancing turns, the fix may touch the restart transition itself; that fix stays inside this change's scope.
