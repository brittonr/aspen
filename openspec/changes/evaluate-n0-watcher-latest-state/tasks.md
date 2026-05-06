## Phase 1: Spec foundation

- [x] Create proposal, design, delta spec, and task rail for `evaluate-n0-watcher-latest-state`.

## Phase 2: Candidate selection

- [ ] Inspect existing Aspen watcher/broadcast usages and select one seam where latest-state semantics are correct.
- [ ] Record rejected seams where every transition, event, or log item must remain durable/ordered.

## Phase 3: Prototype and evidence

- [ ] Add `n0-watcher` only to the selected crate or document why `tokio::sync::watch` remains better.
- [ ] Implement the prototype or no-adoption comparison note without changing durable/network protocols.
- [ ] Add tests for initialization, latest-value convergence, slow-observer skipped values, and disconnect behavior.
- [ ] Capture dependency-tree evidence that `n0-watcher` does not leak into alloc-only/core dependency paths.

## Phase 4: Closeout

- [ ] Update docs/comments describing allowed latest-state observer use and forbidden durable-stream use.
- [ ] Run targeted tests, strict OpenSpec validation, helper verification, and `git diff --check`.
- [ ] Sync/archive only after the prototype/adoption decision and all evidence tasks are complete.
