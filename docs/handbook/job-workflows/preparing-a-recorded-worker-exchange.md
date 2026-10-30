# Preparing a recorded worker exchange

Mode: How-to

## Goal and prerequisites

Prepare a local-gossip job DAG exchange whose request, delivery log, target execution, and result can be reviewed together. This is the recorded local CLI path, not deployment of a live Iroh worker service. The procedure is source-checked and was not executed for this documentation batch. See the [Handbook](../README.md) and [deterministic playback companion](../../technical/foundations/deterministic-playback-contract.md) for context.

You need an installed target-side job closure, a canonical target admission receipt, its matching execution request, genuine peer-bootstrap and node-identity evidence references, and isolated writable storage, cache, transport, and output locations. You also need capabilities to the already initialized sender and receiver node roots and a receiver-committed local session generation. Have the existing sync evidence and authority/resource evidence available for review. Missing prerequisites are a stop condition: a sender's source registry or invented digest is not a substitute.

The [receiver contract](../../unison-reference-execution.md) keeps fetch selection and admission at the receiver. This how-to deliberately begins after those stages; it does not manufacture their evidence.

## 1. Decide whether these are the right inputs

Choose the DAG worker path only if you have `job-admission` and `job-execution-request` artifacts. A `job-ref-submission-v1` used by the local echo worker is a different input, as shown in [the ref-backed walkthrough](following-a-ref-backed-job.md).

Before creating the worker request, compare the execution request's admission reference with the canonical hash of the admission receipt, and compare their job references. The [request builder](../../../src/cli/workflow/job/worker.rs) performs these comparisons. It inherits the sync reference from admission unless explicitly supplied; absent stage arguments, it copies execution-request stages. Empty authority arguments fall back to admission authority-receipt references, and empty resource arguments fall back to execution-request resources.

These defaults preserve supplied evidence; they do not establish its present validity. Choose the target peer explicitly, because the CLI default is `peer:loopback`, which may not match your execution request.

## 2. Construct one request in a fresh destination

Set each environment variable below from your actual evidence inventory. `WORKER_REQUEST_OUT` must be a new output file in your isolated workspace; `TARGET_PEER` must match the admission/execution context. The guards detect missing values, not evidence validity. Repeated evidence flags are available when your context needs several refs.

Source-checked, not executed. Command spelling comes from the [root command declaration](../../../src/main/root/parts/command/p000/body.rs), [job subcommands](../../../src/cli/workflow/job/command.rs), and [worker arguments](../../../src/cli/workflow/job/command/worker.rs); [the implementation](../../../src/cli/workflow/job/worker.rs) defines defaults and binding checks.

```sh
molten test job worker-request \
  --admission-receipt "${ADMISSION_RECEIPT:?Set the existing admission file}" \
  --execution-request "${EXECUTION_REQUEST:?Set its execution request file}" \
  --target-peer "${TARGET_PEER:?Set the admitted target peer}" \
  --peer-bootstrap-ref "${PEER_BOOTSTRAP_REF:?Set actual bootstrap evidence}" \
  --node-identity-ref "${NODE_IDENTITY_REF:?Set actual identity evidence}" \
  --out "${WORKER_REQUEST_OUT:?Set a fresh request output path}"
```

Review the resulting value before continuing. Its evidence list includes sync, admission, execution-request, bootstrap, and identity references. Those bindings explain what the request names; they are not an authority grant.

## 3. Establish receiver knowledge and choose execution scope

The [reference introduction boundary](../../syndicate-reference-harness.md) requires more than a request that carries
bootstrap or authority metadata. `SENDER_STATE` and `RECEIVER_STATE` identify the actual initialized node roots for
this exchange; their node IDs must match `SOURCE_PEER` and `TARGET_PEER`. Review identity bindings against the existing
bootstrap inventory, not only display labels. Do not create a different identity with the same name to make a mismatch
disappear. This recorded profile authenticates local root possession, not a network receiver acknowledgment.

```sh
molten test remote session admit \
  --sender-state-root "${SENDER_STATE:?Actual initialized sender root}" \
  --receiver-state-root "${RECEIVER_STATE:?Actual initialized receiver root}" \
  --topic molten.job.worker --receipt-out "${SESSION_ADMISSION_OUT:?Fresh source admission output}"
```

Read the generation from that canonical commit receipt and set `SESSION_GENERATION` to it. Copy `JOB_REF` and
`EXECUTION_REQUEST_REF` from the corresponding workload operand fields of the reviewed worker request, not from
rendered-file hashes or proof metadata. Then commit an actual introducing assertion at the receiver:

```sh
printf '<job-worker-operands-v1 "%s" "%s">\n' \
  "${JOB_REF:?Actual job operand}" "${EXECUTION_REQUEST_REF:?Actual execution request operand}" \
  > "${WORKER_INTRODUCTION_OUT:?Fresh introducing assertion file}"
molten test remote session commit \
  --sender-state-root "$SENDER_STATE" --receiver-state-root "$RECEIVER_STATE" \
  --topic molten.job.worker --generation "${SESSION_GENERATION:?Actual admitted generation}" \
  --operation assert --payload "$WORKER_INTRODUCTION_OUT" \
  --receipt-out "${INTRODUCTION_RECEIPT_OUT:?Fresh receiver commit output}"
```

Merely constructing the assertion or carrying these refs in evidence/authority lists does not introduce them.
Retraction, close, identity replacement, or a changed generation invalidates the old sender view. These local source
receipts do not replace the existing job authority/resource/admission artifacts.


Use direct local execution when your question concerns request delivery and target execution. Choose the separate `worker-schedule-local` surface only when queue/lease evidence is part of the question. Do not run both merely to improve confidence: both paths can reach execution.

The scheduling shell creates a local fixture coordination runtime, performs enqueue and duplicate-enqueue replay, dequeues, acquires a fencing token, and then calls the worker path. It is not a durable multi-worker scheduling service. See [scheduling implementation](../../../src/cli/workflow/job/schedule/run.rs). This procedure uses the direct path to avoid conflating transport evidence with coordination evidence.

## 4. Execute only after reviewing effect scope

The command below writes transport state, execution state, and evidence. All writable paths should belong to a fresh isolated run. `TARGET_REGISTRY` must already contain the admitted closure. `TARGET_CHUNKS` must identify the required target chunk store, not an empty replacement for existing dependencies. Omit the optional ledger here to keep the example's evidence destination explicit.

Source-checked, not executed. Both the exact options and optional chunk-root behavior are defined by [worker arguments](../../../src/cli/workflow/job/command/worker.rs) and [worker shell](../../../src/cli/workflow/job/worker.rs), under the [job command declaration](../../../src/cli/workflow/job/command.rs).

```sh
molten test job worker-run-local "${WORKER_REQUEST_OUT:?Set the reviewed request}" \
  --target-registry "${TARGET_REGISTRY:?Set the admitted target registry}" \
  --storage "${WORKER_STORAGE:?Set isolated storage}" \
  --cache "${WORKER_CACHE:?Set isolated cache}" \
  --chunks "${TARGET_CHUNKS:?Set the target chunk store}" \
  --admission-receipt "${ADMISSION_RECEIPT:?Set the same admission file}" \
  --execution-request "${EXECUTION_REQUEST:?Set the same execution request}" \
  --transport-root "${WORKER_TRANSPORT:?Set isolated transport state}" \
  --from-peer "${SOURCE_PEER:?Actual initialized sender node ID}" --topic molten.job.worker \
  --sender-state-root "$SENDER_STATE" --receiver-state-root "$RECEIVER_STATE" \
  --session-generation "$SESSION_GENERATION" \
  --out "${WORKER_OUT:?Set a fresh output directory}"
```

## 5. Preserve and interpret the exchange

Review `request.preserves`, `envelope.preserves`, publish/delivery receipts, `delivery-log.preserves`, assignment, statuses, result, and worker receipt together. Execution receipt and output appear only when their corresponding stages produced them. A nonzero exit may still leave useful denial evidence; an earlier error may leave only partial artifacts.

Retain the source admission and introduction receipts with those artifacts. A reference/currentness refusal before
transport is recorded in `sender-denial.preserves`; its canonical denial names unknown workload refs. Do not substitute
stdout or edit a source receipt to make a stale generation appear current. The actual source is reread on build and
publication, and receiver observation remains separate from workload execution authority.


For a worked failure, suppose the request names peer B while the execution request names peer A. Request construction does not compare those target fields; the worker's target-state check does. Stop at that mismatch and reconstruct the intended request from authoritative inputs. Do not edit the receipt to make identities agree or retry the effect path blindly.

## Sources

- [Handbook](../README.md)
- [Deterministic playback companion](../../technical/foundations/deterministic-playback-contract.md)
- [Reference-execution contract](../../unison-reference-execution.md)
- [Worker CLI declarations](../../../src/cli/workflow/job/command/worker.rs)
- [Worker request and artifact-writing shell](../../../src/cli/workflow/job/worker.rs)
- [Target and authority binding checks](../../../src/job/parts/dag/p016/body.rs)
- [Local scheduling sequence](../../../src/cli/workflow/job/schedule/run.rs)
- [Recorded session source owner](../../../src/remote/parts/dataspace/p010/recorded.rs)
- [Reference lifetime and verification boundary](../../syndicate-reference-harness.md)
