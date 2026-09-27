# Inspecting node control evidence

Mode: How-to

## Goal and prerequisites

The goal is to determine what one node-control request actually reached: enqueue, admission, operation result, archive, or a bounded loop observation. You need the selected node root, the original canonical request or its reviewed ref, and access to non-secret artifacts. Prefer an existing evidence export or an operator-approved preserved copy when investigating an incident. Do not run dispatch merely to obtain a more convenient receipt.

This procedure is source-checked and not executed for this document. The [operator runbooks](../../production-operator-runbooks.md) govern review; the [lifecycle companion](../../technical/operations/node-lifecycle-and-recovery.md) explains why local observations are narrower than recovery guarantees. Canonical Preserves plus BLAKE3 identify artifacts. A filename, log line, or successfully parsed receipt does not grant authority.

## 1. Decide whether you need observation or a new effect

For already captured artifacts, use `node show` to obtain a supported summary. It reads the supplied Preserves file and calls the daemon summary dispatcher. It does not enqueue or execute the request. A summary is a navigation aid, not a substitute for examining the canonical fields and linked artifacts.

This guarded example accepts the path to an actual non-secret control or lifecycle artifact. Spelling is supported by the [Show declaration](../../../src/cli/ops/node/command/base.rs), [top-level routing](../../../src/main.rs), and [show implementation](../../../src/cli/ops/node/parts/lifecycle/p001/body.rs). Source-checked; not executed. Do not point it at endpoint secret files or raw tickets.

```sh
: "${CONTROL_ARTIFACT:?Set a reviewed non-secret Preserves artifact path}"
molten node show "$CONTROL_ARTIFACT"
```

By contrast, `node status` writes health and control receipts and imports them into the ledger. `control-dispatch`, `run-loop`, and `serve` can process pending work. If preserving the current evidence state is the goal, stop before invoking those commands. An operator asking for “status” may mean inspection, not permission to write another observation.

## 2. Anchor the investigation to one request

Locate the original `node-control-request` and its canonical request ref. The concrete [status-dispatch regression](../../../src/node/parts/daemon/tests/m000/p000/body.rs) checks that the parsed control receipt's `request_ref` equals the request it submitted. Apply that relationship rather than joining evidence by timestamp or a familiar-looking basename.

Check operation and any target or payload binding before interpreting a result. The dispatcher has distinct branches for `status`, `shutdown`, `install`, `run`, and `gate`; a successful status request does not validate a neighboring job or installation. Treat authority, policy, resource, and evidence references as relationships requiring their own review, not as permission merely because strings are present.

## 3. Separate enqueue evidence from execution evidence

Submission parses and imports the request, writes it to `control/inbox`, then writes a queue receipt. That queue receipt records phase `enqueue`. It proves neither later dispatch nor completion of the operation.

The path helpers distinguish several outbox artifacts: archived request, dispatch receipt, control receipt, and operation receipt. Some operation-specific paths produce additional subreceipts. Ask which artifact exists and what its decision means; do not label every `.preserves` file “the result.” A queue receipt left behind in the inbox is not itself a pending request.

Before dispatch, the implementation requires an active lock bound to the current startup. It re-reads the capability-bound pending entry and rejects bytes changed since discovery. After operation handling it archives through the outbox and removes the inbox entry through its originating namespace. These are separate effects, so archive presence and inbox absence are observations to reconcile, not an atomicity theorem.

## 4. Work a status-request example

Use `assert_status_dispatch` as a source-only example. It constructs the local status request, submits it, obtains the returned inbox entry, dispatches that entry, and parses the resulting control receipt. Its assertions require operation `status`, decision `pass`, and exact request-ref equality. The ledger check expects request, queue, health, and control artifact kinds.

In a real evidence bundle, follow the same chain: request → enqueue receipt → control receipt → health result. Then compare the health startup binding with the startup under review. If the bundle contains only the enqueue receipt, report “submission observed; completion not established.” If it contains a passing control receipt for another request, stop: that is not evidence for this request.

## 5. Handle duplicate and bounded-loop evidence carefully

The dispatch path checks for a prior dispatch before performing a new operation and can emit phase `duplicate-dispatch`. Retain the original operation result alongside that later observation. This local mechanism does not justify exactly-once claims across arbitrary interruption points or remote effects.

A bounded loop receipt can deny because its request limit was reached while work remained. Earlier requests may already have completed. Review processed request refs and their individual receipts before deciding whether anything is safe to retry. A heartbeat is bound to startup and lock evidence; it is not universal proof that all adapters or peers are healthy.

Stop when request identity, startup binding, admission evidence, or effect completion cannot be reconciled. Preserve uncertainty explicitly and escalate through the authorized operational process instead of resubmitting uncertain work.

## Sources

- [Handbook](../README.md)
- [Production operator runbooks](../../production-operator-runbooks.md)
- [Lifecycle technical companion](../../technical/operations/node-lifecycle-and-recovery.md)
- [Control submission implementation](../../../src/node/parts/daemon/p018/body.rs)
- [Dispatch and bounded loop implementation](../../../src/node/parts/daemon/p046/body.rs)
- [Lock and archive path helpers](../../../src/node/parts/daemon/p034/body.rs)
- [Control regression example](../../../src/node/parts/daemon/tests/m000/p000/body.rs)
