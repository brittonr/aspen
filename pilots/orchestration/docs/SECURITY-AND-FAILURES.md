# Threat and failure scope

The initial administrative trust model is a trusted local OS/root owner, one
explicitly authorized controller, allowlisted operator keys and explicitly enrolled
workers. Transport authenticates keys; separate local policy grants execution of
an exact import-free component profile. These are not Basalt capabilities or
hardware attestations. Compromise of an authorized worker can forge its result
observations; there is no Byzantine result validation or redundant computation.

No private key is transmitted in RPC, copied by enrollment, or included in
operator status. The CLI generates private keys under private roots. Use a secure
channel to provision public enrollments and the shared policy. Protect root
ownership, backups and revocation files. Do not publish a root or its key file in
a repository or artifact bundle.

## Failure meanings

| Boundary | Outcome |
|---|---|
| Invalid job / stale fence / denied policy | No admitted execution. |
| Materialization failure before reservation | Task stays pending; verified content writes may already exist. |
| Conflicting assignment CAS | This caller does not dispatch. |
| Unknown assignment CAS | This caller does not dispatch; no success inference from readback. |
| Unknown start RPC | Keep the attempt/reservation; reconcile with the exact worker request. |
| Worker cannot find attempt | Persist a cancellation tombstone; do not blindly start again. |
| Cancellation while running | Signal the in-process runtime; hold resources until actual stopped result. |
| Worker acceptance committed, callback lost | Duplicate start does not execute. Reconcile/cancel or restart. |
| Worker Begin commit unknown | No execution by that caller; Running marker may remain until cancellation/restart reconciliation. |
| Component output publication fails | Record stopped failure; no successful dependency publication. |
| Terminal controller commit unknown | Reconcile the retained worker result; never rerun on that basis. |
| Worker process exits | Exclusive-journal reopen permits interruption classification only for this no-import in-process profile. |
| Snapshot/profile corruption | Refuse startup/read/commit rather than reset state. |
| Bound exhausted | Refuse further work; never delete dedupe history automatically. |

The application distinguishes compare-and-commit conflicts from uncertain outcomes.
The initial allocation core is reused for authoritative controller attempt phase
changes. The pure batch state machine and each restore validator enforce resource
accounting and exact task/attempt relationships.

## Limits of the runtime boundary

A Wasm trap, fuel budget, guest memory limit, epoch interrupt, or cleared hostcall
surface does not prove the Wasmtime compiler/runtime free of security bugs. There
is no production sandbox attestation. Compiler/native allocations and compile
time are not controlled by per-guest limits. OS process/cgroup limits remain
necessary. CPU reservation is scheduler accounting, not a per-task OS quota.

The worker process has normal host privileges of its service account; guest
components receive no imports. Do not extend this adapter to WASI or native
processes without changing the survival matrix, authority model and recovery
rules. Do not apply stopped-on-process-reopen semantics to detached workloads.

Observability counters are process-local, saturating and diagnostic. They are not
durable evidence or authority. No automatic timer/health event grants deletion,
failover or resource-release authority.
