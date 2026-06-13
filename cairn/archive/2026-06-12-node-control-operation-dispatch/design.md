## Design

Control operations remain `node-control-request-v1` values submitted through the durable inbox. Dispatch interprets operation-specific refs as follows:

- `install`: `payload` MUST name a Preserves value already available in the node ledger. Dispatch installs that value into `<state-root>/registry` as a `node-control-artifact`, using request authority refs as install capabilities, request policy refs as artifact policy refs, and request resource refs plus payload/target refs as evidence. The artifact install receipt and operation receipt are subreceipts of the final node control receipt.
- `run`: `payload` MUST name a `job-execution-request-v1` value in the node ledger. `target` MUST name the matching `job-admission-receipt-v1` value. Dispatch executes through `job_dag::execution_loopback` using only node-local registry, storage, cache, and chunk roots.
- `gate`: `target` MUST name the subject artifact ref and `payload` MUST name a strict clean `octet-gate-receipt-v1` value in the node ledger. Dispatch validates it with the Octet source-gate validator for `node-control-gate`.

All three operations first check active lock, explicit request authority/policy/resource refs, and ledger-resolvable payload/target refs. Missing or invalid evidence emits deny receipts before operation side effects. Denied suboperations still emit canonical subreceipts and final deny control receipts so operators can inspect failure evidence through the node ledger.

## Boundaries

This change does not add live socket serving, remote operation submission, or new authority systems. It reuses existing local artifact/job/Octet semantics and keeps rendered CLI output non-normative.
