## Why

Unison's `Remote` ability frames distributed execution as `fork` a computation, possibly elsewhere, then `await` the result. Aspen has JobManager, WorkerPool, Iroh, blob transfer, Raft scheduling, runtime-hosts, and madsim, but callers still integrate with executor-specific orchestration.

Aspen should define a `Remote`-style fork/await abstraction that can run through local, simulated, receipt-recording, and real cluster handlers while preserving Aspen's explicit capability and receipt contracts.

## What Changes

- Define a portable `Remote` abstraction for submitting content-addressed execution closures and awaiting typed output handles.
- Provide at least a local/deterministic handler and one real Aspen JobManager/WorkerPool-backed handler.
- Require receipt correlation between remote handles, closure hash, job ID, worker/runtime target, and output handle.
- Require cancellation/timeout/error behavior to be typed and bounded.

## In Scope

- API/spec for fork, await, cancel/timeout, and receipt correlation.
- Local handler for tests and one product-backed handler.
- Negative tests for missing result, timeout, cancellation, and capability denial.

## Out of Scope

- Transparent language-level function mobility.
- Public HTTP RPC.
- Replacing every CI/job API immediately.

## Verification

- `openspec validate add-remote-fork-await-abstraction --strict`
- Local-handler deterministic tests.
- Product-path fork/await test through JobManager/WorkerPool.
- Timeout/cancel/denied-capability negative tests.
- `openspec validate --all --strict --json`
- `git diff --check`
