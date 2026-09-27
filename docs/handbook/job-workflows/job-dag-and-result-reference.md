# Job DAG and result reference

Mode: Reference

Use this page to distinguish job identities, request families, and output evidence during a handoff. It describes inspected source, not a newly executed exchange. All command names below are lookup entries, not runnable recipes. Start with the [Handbook](../README.md); consult the [canonical identity companion](../../technical/envelopes/canonical-preserves-boundary.md) for why a Rust struct is not a wire identity.

## Record families and their owners

The [DAG module](../../../src/job/dag.rs) includes implementation parts; public data structures are not a replacement for the canonical constructors and parsers. Reference identity is computed from canonical Preserves using BLAKE3. An artifact's local pathname locates bytes but does not define its identity.

| Family | Important fields or bindings | Owner and review purpose |
| --- | --- | --- |
| `job-dag-v1` | Nodes, edges, output roots, schema/effect/policy/evidence refs | DAG constructor and parser; identifies graph inputs |
| `job-sync-request-v1` | Job, selected stages, target peer, policy/capability/evidence refs | Sync planner; names closure-transfer intent, not execution |
| `job-admission-request-v1` | Job, sync, stages, target peer, resource refs | Target admission; requests a receiver-side decision |
| `job-execution-request-v1` | Job, admission, target peer, stages, storage/cache/chunk profile refs | Execution boundary; binds target context without source-registry access |
| `job-worker-request-v1` | Job, target, stages, sync, admission, execution request, authority/resources, bootstrap/identity/evidence refs | Worker boundary; binds transport-delivered work to prior inputs |
| `job-ref-submission-v1` | Job ID, operation ID, executable/input content refs, handler, effects, authority, policy, provenance | Local ref worker; a separate submission family |

The DAG node structure contains `id`, `kind`, optional `stage_artifact_ref`, input/output ports, configuration, and effect/policy/evidence refs. Edges name both endpoints and ports and include optional schema, partitioning, and materialization fields. See [DAG types](../../../src/job/parts/dag/p000/body.rs) and [canonical construction](../../../src/job/parts/dag/p002/body.rs). Do not flatten an edge into an ordering constraint and discard its port semantics when reviewing inputs.

## Result and receipt fields

These are Rust field names from the [result structures](../../../src/job/parts/dag/p001/body.rs), intended for source navigation rather than a JSON schema. Canonical record labels are defined separately in [worker value construction](../../../src/job/parts/dag/p012/body.rs).

| Structure | Fields to preserve | What they answer |
| --- | --- | --- |
| `JobRun` | `job_ref`, `request_ref`, `stage_receipt_refs`, `output_refs`, `output_value`, `receipt_value` | Which local DAG run produced the outputs? |
| `JobWorkerResult` | `decision`, `job_ref`, `target_peer`, optional `execution_receipt_ref`, `output_refs`, stage/receipt pairs, resource receipt refs, optional delivery-log ref, diagnostics | What execution and transport evidence supports the result? |
| `JobWorkerReceipt` | Optional job/request refs, assignment ref, status refs, result ref, optional execution/delivery-log refs | How do the worker's observations join together? |
| `JobWorkerScheduleReceipt` | Queue and lease keys, worker session, coordination report, optional token/worker/result refs | Which local scheduling observations surround the worker attempt? |
| `BlobRefJobExecution` | Submission, decision, statuses, optional output manifest, receipt, diagnostics | Did the local ref handler produce a stored object? |

Worker decisions are `pass`, `deny`, and `non-replayable`; worker states are `received`, `running`, `completed`, `denied`, and `non-replayable`. The blob-ref path instead emits states including `queued`, `fetching`, `running`, `result-ready`, `complete`, and `failed`. These vocabularies are not interchangeable.

## CLI lookup

The spelling is source-checked against [root routing](../../../src/main/root/parts/command/p000/body.rs), [job subcommands](../../../src/cli/workflow/job/command.rs), and [worker](../../../src/cli/workflow/job/command/worker.rs)/[ref](../../../src/cli/workflow/job/command/refs.rs) arguments. No commands were executed.

| Under `molten test job` | Role | Evidence owner |
| --- | --- | --- |
| `worker-request` | Build a request from admission and execution files | Request builder |
| `worker-run-local` | Record local-gossip delivery and invoke target execution | Worker shell |
| `worker-schedule-local` | Wrap the worker in local queue/lease observations | Scheduling shell |
| `ref-submit` / `ref-execute` | Construct / process a content-ref-only submission | Ref worker shell |
| `status` | Read job status evidence from a ledger | Ref CLI reader |
| `receipt-show` | Read a ledger receipt by reference | Ref CLI reader |

## On-disk output inventory

The [worker writer](../../../src/cli/workflow/job/worker.rs) owns these names beneath its output directory:

| Artifact | Meaning and availability |
| --- | --- |
| `request.preserves`, `envelope.preserves` | Input and transport wrapping |
| `publish-receipt.preserves`, `delivery-receipt.preserves`, `delivery-log.preserves` | Recorded local transport observations |
| `assignment.preserves`, indexed status files | Assignment and progress evidence |
| `result.preserves`, `worker-receipt.preserves` | Worker result and evidence join |
| `execution-receipt.preserves` | Written only when an execution object exists |
| `output.preserves` | Written only when that execution contains a run |

Writing is sequential, not an atomic directory commit. A partial directory does not establish which external effects did or did not occur.

## Bounds and worked interpretation

The implementation declares 256 nodes, 4,096 edges, 4,096 references, 64 ports, 256 checks, and 4,096 stage values as relevant local bounds. These are representation limits, not throughput or durable queue-capacity promises. The echo handler's concatenated-byte bound is the product of the stage-value limit with itself.

Consider an exchange with a passing execution receipt but `non-replayable` worker result. The [decision helper](../../../src/job/parts/dag/p015/body.rs) explicitly permits that combination when recorded delivery is absent. Preserve both observations: computation passed at the inspected execution boundary, while the recorded-worker evidence requirement did not. Neither an output file nor an execution receipt upgrades this to replayable transport evidence or current execution authority.

## Sources

- [Handbook](../README.md)
- [Canonical Preserves boundary](../../technical/envelopes/canonical-preserves-boundary.md)
- [Reference-execution governing contract](../../unison-reference-execution.md)
- [DAG fields and bounds](../../../src/job/parts/dag/p000/body.rs)
- [Request and result types](../../../src/job/parts/dag/p001/body.rs)
- [Worker result construction](../../../src/job/parts/dag/p012/body.rs)
- [Worker output filenames](../../../src/cli/workflow/job/worker.rs)
