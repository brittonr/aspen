# Following a delivery diagnostic

Mode: Walkthrough

This walkthrough follows the checked-in `bounded_generated_delivery_trace_preserves_idempotency_invariants` fixture, from its input bindings to persisted diagnostic decisions. It is a source walkthrough, not a live transport tutorial or an executed test report. Return to the [Handbook](../README.md); use the [claims and retry companion](../../technical/replication/delivery-claims-acks-and-retry.md) for coordination theory.

The diagnostic in the [README](../../../README.md#delivery-idempotency-diagnostics) is separate from the coordination-delivery claim, lease, and dead-letter state machine. Its central question is whether a scoped operation is first, duplicate, conflicting, stale, or ahead of its sequence window. It does not send the operation to a worker.

## 1. Fix the fixture boundary

Open the [generated trace fixture](../../../src/delivery/parts/idempotency/tests/m000/p000/body.rs). It creates an isolated temporary delivery root, derives the remote-topic scope for topic `services` and consumer `peer:b`, and builds seven trace steps. Producer `peer:a/producer` supplies most steps; the final stale step uses `peer:c/producer`.

The fixture's `fake_ref` inputs are test material, not current payload provenance, policy authority, or evidence for deployment. Do not copy them into an operational request. Its side-effect counters count returned decisions; the fixture does not perform the external business effect those decisions might precede.

Observable boundary: each trace step contains a sequence, payload label, evidence label, gap policy, expected decision, and expected side-effect Boolean. Those are the inputs and assertions to follow, not invented terminal output.

## 2. Follow identity before storage

The [identity constructor](../../../src/delivery/parts/idempotency/p000/body.rs) builds `operation-id-v1` from scope, producer, consumer, sequence, intent, payload reference, and policy references. Canonical Preserves and its BLAKE3 identity define the operation, not Rust struct layout or the temporary directory name.

The deduplication key in the [store helpers](../../../src/delivery/parts/idempotency/p002/body.rs) contains scope, producer, consumer, sequence, and intent. Payload is deliberately absent from that lookup key: changing payload for the same lookup coordinates must discover the previous entry and produce a conflict, rather than silently becoming another first operation.

Observable boundary: distinguish the operation reference from the dedup key and scope reference. They identify different records and are not interchangeable lookup tokens.

## 3. Track the seven decisions

The [decision law](../../../src/delivery/parts/idempotency/p001/body.rs) checks an existing entry before sequence-window comparisons. The fixture therefore expects:

| Stage | Supplied change | Expected decision meaning |
|---|---|---|
| 1 | Sequence 1, payload A, evidence A | `first`; advance the window |
| 2 | Exact same operation and evidence | `duplicate`; suppress another side effect |
| 3 | Sequence 1 with changed payload | `conflict`; suppress |
| 4 | Sequence 4 while next is 2 | `gap` under deny policy |
| 5 | Same gap with retry policy | `retry`; still suppress |
| 6 | Sequence 2, payload B, evidence B | `first`; advance again |
| 7 | Another producer at sequence 1 | `stale`; no matching retained entry |

The fixture asserts two permitted and five suppressed decisions. That is a source-checked assertion, not a result obtained while writing this page. Notice that the sequence window is stored by scope, not independently by producer. The final row is not a duplicate merely because its sequence appeared earlier.

## 4. Identify what becomes durable

On a first decision, the [first-decision helper](../../../src/delivery/parts/idempotency/p006/body.rs) advances `next_sequence`, constructs the receipt and dedup entry, and calls the storage transaction. The transaction writes the window, entry, receipt, and retention pins in `delivery-idempotency.redb` before returning `should_commit_side_effect: true`.

There is no external-effect callback in that transaction. A crash after this record is stored but before a caller's effect is completed is not resolved by calling the diagnostic again: the record can now classify the operation as duplicate. The caller needs its own effect and semantic-result evidence. Conversely, a `retry` decision is a sequence-gap classification, not proof that an uncertain external effect is safe to repeat.

## 5. Inspect an existing exported artifact safely

If a diagnostic receipt file already exists, prefer file inspection over issuing another stateful `check`. This optional command is source-checked, **not executed**. It requires an available `molten` binary and an actual idempotency artifact path; it neither creates that artifact nor inspects coordination-delivery receipts.

Command provenance: [root `test delivery` declaration](../../../src/main/root/parts/command/p000/body.rs), [delivery arguments](../../../src/cli/workflow/delivery/command.rs), and [file-only show handler](../../../src/cli/workflow/delivery/ops.rs).

```sh
molten test delivery show "${DELIVERY_ARTIFACT:?Set an existing idempotency artifact path}"
```

The handler parses the file and prints a summary. Preserve the full artifact because the summary omits bindings. In particular, the `check` summary's `prior=` value comes from `prior_semantic_result_ref`; it is not the receipt's separate `prior` receipt reference. Neither a printed `commit` nor a stored semantic-result reference grants authority or proves external completion.

## Sources

- [Handbook](../README.md)
- [Coordination delivery contract](../../coordination-delivery.md)
- [Claims and retry companion](../../technical/replication/delivery-claims-acks-and-retry.md)
- [Seven-step diagnostic fixture](../../../src/delivery/parts/idempotency/tests/m000/p000/body.rs)
- [Diagnostic decision law and receipt parser](../../../src/delivery/parts/idempotency/p001/body.rs)
- [First-decision persistence boundary](../../../src/delivery/parts/idempotency/p006/body.rs)
