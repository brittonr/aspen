# Reviewing job effects and retry safety

Mode: Review checklist

Use this checklist before accepting a job-workflow change, replay claim, or proposal to rerun an interrupted worker. It asks for evidence at the actual effect boundary rather than inferring safety from a successful receipt. No execution or tests were performed for this page. Return to the [Handbook](../README.md); use the [effect-profile companion](../../technical/configuration/effect-manifest-and-handler-profiles.md) for theory and the [governing effect contract](../../effect-manifest-profiles.md) for required admission semantics.

## Establish exactly what is under review

- [ ] Does the review identify the path: local blob-ref echo, target DAG execution, recorded local-gossip worker, or local scheduled worker? Attach the request family and entry-point source. These are distinct compositions, not interchangeable names for a production remote executor.
- [ ] Are the canonical request, job, admission, and effect/profile identities recorded? A Rust type, pathname, operation label, or human job name is insufficient. Include the target closure and applicable initial state when claiming replay equivalence.
- [ ] Is each reported observation labeled source inspection, checked-in fixture expectation, or newly executed evidence? A test name in a document does not establish a current passing run.

Acceptance evidence should be a short inventory with references to the actual values and their producers. Resolve uncertain scope before reviewing performance or convenience changes.

## Separate declared effects from admitted effects

- [ ] Which component establishes current authority, policy, provenance, source-gate, and resource observations? Point to its decision and consuming callsite rather than only a vector of reference strings.
- [ ] Where are handler-profile support and individual effect requests admitted? The governing contract distinguishes profile admission from request-level capability checks; neither merely naming an effect nor having a supported handler grants invocation permission.
- [ ] Does the review acknowledge the local blob-ref preflight's narrower behavior? Its [implementation](../../../src/job/parts/dag/p013/body.rs) tests nonempty policy/provenance/effect collections and the supported profile/output mode. It does not itself dereference those values into comprehensive current admission.
- [ ] Is the executable behavior described honestly? The [echo handler](../../../src/job/parts/dag/p014/body.rs) concatenates input bytes; it does not launch the verified executable bytes. The fixture's `elf-executable` format label is not a native execution result.

A passing local fixture can support content-verification behavior without satisfying a broader remote-admission claim. Record that limit rather than silently assigning the fixture more authority.

## Inspect denied and interrupted paths

- [ ] Which storage or transport effects can occur before final denial? The blob-ref path can fetch and pin before deciding whether to run the handler. The local worker publishes and delivers before worker admission checks complete. Do not require or claim “deny means no effects whatsoever.”
- [ ] What happens after output production but before evidence writing finishes? The [worker shell](../../../src/cli/workflow/job/worker.rs) invokes execution before sequentially writing its output files. A file-writing failure can leave an uncertain evidence boundary even when execution occurred.
- [ ] Are pin and cleanup claims precise? Blob-ref cleanup records successful input/executable unpins, ignores individual unpin errors, and does not clean the output pin through that list. Ask for per-object evidence when complete release matters.
- [ ] Are unknown outcomes preserved without automatically invoking the job again? Supply the last observed boundary and the operation owner's resolution procedure. A missing result file is not proof of nonexecution.

## Review scheduling without exactly-once assumptions

- [ ] Is duplicate suppression scoped to the actual duplicated operation? The [scheduler](../../../src/cli/workflow/job/schedule/run.rs) submits the same enqueue request twice and checks replay of its receipt. That observation is not duplicate suppression for arbitrary external stage effects.
- [ ] Does the report identify the newly constructed local fixture coordination runtime? A successful run does not demonstrate a durable queue surviving process restart or coordinating independent hosts.
- [ ] Is fencing checked before worker invocation, and is a mismatch preserved as denial? The [phase implementation](../../../src/cli/workflow/job/schedule/phase.rs) compares the effective token with the acquired token and returns before execution on mismatch. Never recommend choosing another token simply to evade a denial.
- [ ] Does the review distinguish normal release from error propagation? The normal returned-worker path attempts release; an error propagated by worker execution can exit before that later statement. Do not claim unconditional cleanup or distributed lease recovery from this helper.

## Check result completeness and replay claims

- [ ] Does the worker result bind the intended execution receipt, delivery log, output refs, and ordered stage receipt pairs? Compare them with independently expected run inputs, not only with one another.
- [ ] Are stage counts independently checked? The [pairing helper](../../../src/job/parts/dag/p021/body.rs) zips admission stage order with receipt refs; it does not reject unequal lengths there. This is a source-review observation, not a reproduced bug. A claim of complete stage evidence needs another enforcing boundary or explicit comparison.
- [ ] Does recorded delivery actually exist for the envelope? A successful execution without that condition is `non-replayable`, not a recorded-worker pass.

## Worked interrupted-run decision

Suppose a scheduled worker has produced output, but writing `worker-receipt.preserves` fails. The queue's earlier duplicate-enqueue check still says only that enqueue replay worked inside its local runtime. It does not show that the stage never ran or that repeating it is safe. Preserve the output, transport state, available receipts, and error. Withhold retry approval until the effect owner resolves what occurred and supplies a separately authorized next action. Do not delete state or rerun merely to obtain a tidier directory.

Accept a review only with claims matched to evidence scope. Receipts remain evidence, not authority; recorded local execution does not establish exactly-once effects, distributed liveness, or production readiness.

## Sources

- [Handbook](../README.md)
- [Effect manifest and handler-profile companion](../../technical/configuration/effect-manifest-and-handler-profiles.md)
- [Governing effect admission contract](../../effect-manifest-profiles.md)
- [Deterministic playback companion](../../technical/foundations/deterministic-playback-contract.md)
- [Blob-ref preflight and cleanup](../../../src/job/parts/dag/p013/body.rs)
- [Worker execution and sequential output writing](../../../src/cli/workflow/job/worker.rs)
- [Local coordination sequence](../../../src/cli/workflow/job/schedule/run.rs)
- [Token, execution, and release boundaries](../../../src/cli/workflow/job/schedule/phase.rs)
- [Stage-receipt pairing](../../../src/job/parts/dag/p021/body.rs)
