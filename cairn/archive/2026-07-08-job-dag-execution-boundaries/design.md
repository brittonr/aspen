## Context

The job subsystem coordinates artifact refs, DAG dependencies, admission receipts, worker requests, local execution, and scheduling through coordination services. These are different trust boundaries and should not require one shared implementation namespace.

## Design

### Proposed job modules

- `model`: job ids, task refs, target refs, dependency graph inputs.
- `plan`: pure DAG planning and dependency closure checks.
- `admission`: authority, provenance, resource, policy, and effect checks.
- `schedule`: queue/lease/fencing-token decisions as pure plans.
- `worker`: worker request and execution result models.
- `blob_io`: manifest fetch/verify shell and chunk-store adapter use.
- `coordination`: coordination-service adapter boundary.
- `receipts`: canonical job receipt constructors and parsers.
- `cli`: command parsing and file/output shell.

### Execution law

Worker execution must require admitted job, executable, input, policy, provenance, resource, and effect evidence. Blob availability, queue delivery, or lease acquisition alone does not grant execution trust.

### Test strategy

Positive tests should cover admitted DAG planning, admitted scheduling, and admitted local execution intents. Negative tests should cover missing provenance, stale admission, dependency cycle, stale fencing token, missing blob manifest, and unsupported executor profile.

## Non-goals

- Do not redesign the job CLI.
- Do not replace coordination services.
- Do not turn queue delivery or blob presence into authority.
