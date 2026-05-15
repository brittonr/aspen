## Phase 1: API and handler inventory

- [ ] [serial] Inventory JobManager, WorkerPool, runtime-host proof, CI, and test helper submission/await patterns.
- [ ] [depends:inventory] Define the remote handle, fork input, await result, cancellation, timeout, and receipt correlation types.

## Phase 2: Handlers

- [ ] [depends:api] Implement a local deterministic handler for tests and examples.
- [ ] [depends:local-handler] Implement or adapt one product JobManager/WorkerPool-backed handler without creating a parallel scheduler.
- [ ] [depends:product-handler] Wire timeout and cancellation behavior with bounded state transitions.

## Phase 3: Tests and docs

- [ ] [depends:handlers] Add positive local and product-path fork/await tests.
- [ ] [depends:positive-tests] Add negative tests for missing handle, timeout, cancellation denial, and capability denial.
- [ ] [depends:negative-tests] Update docs and run focused remote-execution tests, strict OpenSpec validation, and `git diff --check`.
