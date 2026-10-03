# Diagnosing worker admission and result failures

Mode: Troubleshooting

Start with preserved artifacts, not a second execution attempt. This guide separates malformed inputs, worker binding denials, execution denials, and incomplete evidence. It is based on source inspection; no failure was reproduced or command executed for this page. The [Handbook](../README.md) links adjacent workflows, while the [evidence/authority companion](../../technical/foundations/evidence-and-authority-separation.md) explains why a receipt is not permission.

## Triage the boundary first

Preserve the submitted files, target-root locations, output directory, stderr, and process status. Record whether a result exists and whether it contains an execution-receipt reference. In the [worker CLI](../../../src/cli/workflow/job/worker.rs), a returned worker execution is written before the command rejects a nonpassing result. Parsing, transport, execution, or file-writing errors can instead return earlier. A missing final file therefore does not prove that nothing happened.

Use [the artifact reference](job-dag-and-result-reference.md) to identify file roles. Read copies of evidence if operational policy requires immutable incident originals. Do not remove a ledger, clear a cache, or delete transport state to make the symptom disappear.

## Symptom: request cannot be constructed or parsed

**Discriminating evidence:** Determine whether the error identifies record shape, schema, reference syntax, or a prohibited mobile/ambient token. The [worker parser](../../../src/job/parts/dag/p004/body.rs) expects `job-worker-request-v1`; a blob-ref submission is not an alternative encoding. The CLI request builder separately rejects execution requests whose admission reference or job reference disagrees with the supplied admission artifact.

**Safe next action:** Locate the correct producer artifact and compare canonical identities, not filenames or readable job labels. Rebuild a request only from the intended matching inputs into a fresh file.

**Stop condition:** If you cannot identify which admission actually belongs to the execution request, stop before transport. Do not replace reference strings manually or remove structural checks.

## Symptom: delivered message, denied worker

**Discriminating evidence:** Inspect result diagnostics for the failed relation. [Delivery and input checks](../../../src/job/parts/dag/p015/body.rs) compare the envelope target with the request target, recompute admission and execution-request references, compare job and sync identities, and require selected stages to agree with admission order. A successfully published message answers none of those questions by itself.

**Safe next action:** Make a small comparison sheet: worker request → execution request → admission receipt. For each, record job identity, target, admission identity, sync identity where present, and ordered stage list. A stage sequence is not an unordered set for this check.

**Stop condition:** Do not resend while one row belongs to a different run. Preserve the mismatch as evidence of incorrect composition, not as a transport outage.

## Symptom: authority or resource binding denied

**Discriminating evidence:** The [binding helpers](../../../src/job/parts/dag/p016/body.rs) require nonempty authority refs, nonempty admission authority-receipt refs, and membership of requested authority refs in admission refs. Resource refs must also be nonempty and bound in admission; the admission resource verdict must pass.

**Safe next action:** Ask the owning admission process for the appropriate evidence, then review that evidence's subject and scope. Peer identity, bootstrap evidence, queue claims, and successful delivery are not substitute execution authority.

**Stop condition:** If only reference presence is known, do not claim present-time validity. These helpers compare supplied values; they do not independently query every authority or revocation source.

## Symptom: execution receipt denies or output is absent

**Discriminating evidence:** Examine target closure, topology, executable gate, capability context, resource profile, sync binding, and strict source-gate observations. The [execution admission readiness helper](../../../src/job/parts/dag/p030/body.rs) requires the named checkset, authority receipt refs, and passing resource verdict. Its checkset membership test should not be inflated into independent verification of every referenced artifact.

**Safe next action:** Confirm the target registry is the intended populated registry. Never substitute the sender's registry to bypass the target-only boundary. If chunks are involved, confirm which chunk root was selected; the CLI defaults to `job-chunks` beneath the target registry when no explicit root is given.

**Stop condition:** Missing target artifacts require the supported sync/admission process before a new execution, not a change to the receipt's decision.

## Symptom: computation passed but result is non-replayable

**Discriminating evidence:** Check whether the supplied delivery log is replayable and contains the delivered envelope reference. The worker [final-decision function](../../../src/job/parts/dag/p015/body.rs) distinguishes execution success from recorded-delivery success. The [live-unrecorded fixture](../../../src/job/parts/dag/tests/m000/p004/body.rs) expects a passing execution with a `non-replayable` result.

**Safe next action:** Classify the run as diagnostic-only for the missing recording boundary. A newly recorded run, if separately authorized, is new evidence; it does not retroactively record the previous delivery.

**Stop condition:** Never relabel the old log or use matching output bytes as proof of recorded transport.

## Worked failure and escalation packet

The checked-in [missing-authority case](../../../src/job/parts/dag/tests/m000/p003/body.rs) constructs a valid worker request with an empty authority list, delivers it with a recorded log, and expects `deny` with no execution object. This isolates admission from transport: adding more network attempts would not address the discriminating evidence.

For escalation, provide the request, admission, execution request, transport artifacts, result and diagnostics if present, and the exact last known completed boundary. State separately whether effects are known not started, observed completed, or unresolved. A process error with partial evidence belongs in the unresolved category until the effect owner establishes more; it is not automatic retry permission.

## Sources

- [Handbook](../README.md)
- [Evidence and authority separation](../../technical/foundations/evidence-and-authority-separation.md)
- [Reference-execution contract](../../unison-reference-execution.md)
- [Worker shell and error boundaries](../../../src/cli/workflow/job/worker.rs)
- [Binding and recorded-delivery checks](../../../src/job/parts/dag/p015/body.rs)
- [Authority and target-only checks](../../../src/job/parts/dag/p016/body.rs)
- [Negative worker fixtures](../../../src/job/parts/dag/tests/m000/p003/body.rs)
