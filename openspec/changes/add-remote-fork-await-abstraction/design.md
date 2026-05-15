## Context

Aspen's distributed execution code is powerful but fragmented. A common fork/await abstraction would let product code, tests, simulations, and future service orchestration share one mental model while still using Aspen's explicit closure/capability/receipt machinery.

## Goals / Non-Goals

**Goals:**
- Define a small remote execution API over execution closures.
- Support local deterministic and real cluster handlers.
- Make receipts and cancellation/timeouts first-class.

**Non-Goals:**
- Making arbitrary Rust closures serializable.
- Hiding distributed failure modes.
- Adding HTTP endpoints.

## Decisions

### 1. Fork returns a durable remote handle

**Choice:** `fork` returns a handle that records closure hash, input handle, submission identity, and handler/backend identity.

**Rationale:** Await/cancel/diagnose need stable handles across process boundaries.

### 2. Await returns output handle plus receipt

**Choice:** `await` resolves to a typed output handle and receipt summary, not necessarily raw output bytes.

**Rationale:** Large outputs belong in blob/KV storage and receipts must stay bounded.

### 3. Multiple handlers share the same core contract

**Choice:** Local, madsim/chaos, receipt-recording, and real JobManager-backed handlers implement the same contract.

**Rationale:** This matches Aspen's simulation strengths and Unison's local interpreter lesson.

## Risks / Trade-offs

**API hides hard distributed problems** → Preserve explicit timeout, cancellation, capability, and failure variants.

**Handle leaks** → Define cancellation and retention/GC behavior.

**Duplicating JobManager** → The real handler must wrap existing job orchestration rather than fork a new scheduler.
