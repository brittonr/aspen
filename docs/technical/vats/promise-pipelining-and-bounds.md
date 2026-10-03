# Promise pipelining and bounds

Promise pipelining describes dependent work without pretending that an unresolved far-call result is already available. This article examines Molten's finite promise-state, pipeline, and promise-use predicates. It assumes familiarity with asynchronous far references and canonical references. The [architecture](../../architecture.md#vatobject-layer-goblins-inspired) supplies the governing model; the [Technical companion](../README.md) connects adjacent explanations.

## Three questions, three representations

A promise review has three separate questions. Is the promise state well formed and legally changing? Is the queue of dependent operations bounded and ordered? Is a particular use backed by the required resolution or pipeline evidence? Combining these into “the promise is valid” would hide distinct failure modes.

The inspected implementation keeps these concerns separate. `evaluate_promise_state_transition` compares before and after promise states. `evaluate_promise_pipeline` validates a source promise together with a queue. `evaluate_promise_use` validates a dependent use against a resolution reference or pipeline reference. Their canonical receipts identify the supplied representations and local checks; a receipt does not itself execute or authorize remote work.

## State shape and terminal immutability

The [promise-shape validator](../../../src/runtime/predicates/parts/mod/p010/body.rs) distinguishes pending, resolved, broken, cancelled, and timed-out states. Pending states cannot carry terminal value, reason, or causal data. Resolved states require a canonical value reference and exclude failure data. Broken states require a nonempty reason, exclude a resolved value, and validate the causal reference collection. Cancellation and timeout require a reason and exclude both resolved values and causal-resolution data.

The [transition evaluator](../../../src/runtime/predicates/parts/mod/p008/body.rs) requires the same promise identifier before and after. Once terminal, the entire state is immutable, not merely its status. An identical terminal description is allowed by this local check; changing its reason, value, or other fields is not. A pending-to-pending change is likewise rejected when it changes the represented state.

These are deterministic in-memory transition checks. A timed-out state contains a reason; this evaluator does not observe a clock or prove that a deadline elapsed. A cancelled state does not prove that a remote operation was physically interrupted. Those observations and effects belong to their admitted runtime boundaries.

## The queue law

The [pipeline validator](../../../src/runtime/predicates/parts/mod/p005/body.rs) checks the source state before examining `max_queue` and `entries`. A nonempty queue with a zero bound is denied, as is any queue longer than its declared bound. Every terminal source requires an empty queue.

Each entry carries `sequence`, `target_ref`, and `operation`. Sequence values must be unique and strictly increasing in the supplied order. Targets must be canonical content references and operations nonempty. The check does not require contiguous numbering or a particular first sequence number. Thus entries numbered 10 and 30 can satisfy ordering just as entries numbered 1 and 2 can. Sorting a malformed queue before recording its evidence would conceal the submitted ordering error rather than explain it.

The vat fixture uses a queue bound of four, visible in the [fixture constants](../../../src/runtime/vat/parts/mod/p000/body.rs). Four is not a universal product capacity or a measured throughput limit. The validator accepts an explicit bound; queue length is not a bound on payload bytes, object execution cost, total application promises, or wall-clock latency.

## Admitting a dependent use

For `ResolvedValue` use, the source must be resolved and its value reference must equal `admitted_resolution_ref`. Pipeline evidence cannot substitute for that resolution. For `PipelineForward`, the source must still be pending, resolution evidence is excluded, and an admitted pipeline reference must be supplied. These rules are implemented in the [promise-use validators](../../../src/runtime/predicates/parts/mod/p005/body.rs).

An important limit is visible in the code: pipeline-use validation checks the presence and canonical shape of the supplied pipeline reference; it does not load an artifact and independently recompute its admission. The surrounding integration remains responsible for giving meaning to “admitted.” Canonical reference syntax is necessary for evidence identity, not sufficient for provenance or authority.

## Worked example: dependent catalog operations

Consider an **illustrative** far call that will yield a catalog object. The caller queues `lookup` and then `read-metadata` against the future reference, with sequence numbers 10 and 20 and a declared bound of two. The local queue law admits this shape if both target references and operation fields are valid.

Adding a third entry exceeds the bound. Renumbering the second entry to 10 creates a duplicate and breaks strict order. Neither failure can be repaired by asserting that the remote service is fast enough: these are finite admission properties, independent of observed latency.

Now the source promise becomes broken because the target turn aborted. Keeping either dependent entry produces `terminal-promise-pipeline-not-cleaned`. Removing both satisfies the pipeline cleanup condition, but does not retroactively prove that no already released remote request ran. The [addressable actor profile](../../addressable-actor-runtime.md#unknown-effects) treats uncertain external outcomes separately and does not infer safe retry from a local terminal state.

## Verification and limits

The existing [vat property tests](../../../src/runtime/vat/parts/mod/tests/m000/p001/body.rs) cover queues within the fixture bound, overflow, and terminal cleanup. The [promise fixture](../../../src/runtime/vat/parts/mod/p001/body.rs) constructs resolution, breakage, cancellation, timeout, terminal mutation denial, and unresolved-use denial cases. Suggested review compares each denial against the specific input boundary rather than merely checking for a receipt. These checks were inspected, not executed for this article.

The predicates provide no delivery guarantee, fairness proof, remote cancellation guarantee, or exactly-once execution guarantee. Bounded queuing prevents one represented queue from exceeding its declared entry count; production resource admission and transport behavior remain separate concerns.

## Sources

- [Technical companion](../README.md)
- [Architecture: vat/object layer](../../architecture.md#vatobject-layer-goblins-inspired)
- [Addressable actor unknown effects](../../addressable-actor-runtime.md#unknown-effects)
- [Promise-state shape](../../../src/runtime/predicates/parts/mod/p010/body.rs)
- [Promise transitions](../../../src/runtime/predicates/parts/mod/p008/body.rs)
- [Pipeline and promise-use checks](../../../src/runtime/predicates/parts/mod/p005/body.rs)
- [Vat fixture constants](../../../src/runtime/vat/parts/mod/p000/body.rs)
- [Promise fixture](../../../src/runtime/vat/parts/mod/p001/body.rs)
- [Vat property tests](../../../src/runtime/vat/parts/mod/tests/m000/p001/body.rs)
