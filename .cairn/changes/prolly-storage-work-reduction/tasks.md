# Tasks

## Phase 1: Measurements and baseline

- [ ] [serial] Add pure measurement counters (`nodes_read`, `nodes_decoded`, `subtrees_not_traversed`, `transactions_opened`, `bytes_copied`) to the Prolly service observations and split `skipped_equal_nodes` into `shared_node_count` plus avoided-traversal counts. r[molten.prolly.metrics]
- [ ] [serial] Capture the rebuild-first baseline for edit, diff, and snapshot-load operations within the standard profile and record it as repo-owned evidence. r[molten.prolly.metrics]

## Phase 2: Bounded read sessions

- [ ] [serial] Add the bounded read-session operation to `ProllyBlockStorePort` and implement the single-transaction Redb adapter; update all adapters and tests. r[molten.prolly.readsession]
- [ ] [serial] Add positive tests (one session per snapshot load, measured transaction reduction) and negative tests (use-after-close, bound exhaustion). r[molten.prolly.readsession]

## Phase 3: Genuine subtree-skipping diff

- [ ] [serial] Implement the demand-driven diff planner that skips matching validated subtrees and requests blocks only for differing regions, keeping validation contracts unchanged. r[molten.prolly.diff]
- [ ] [serial] Add differential tests proving exact diff-record equality with the reference implementation plus demonstrated traversal reductions, including the untrusted-matching-root negative case. r[molten.prolly.diff]

## Phase 4: Incremental edits

- [ ] [serial] Implement the bounded incremental edit planner with rebuild fallback and boundary-resynchronization continuation. r[molten.prolly.incremental]
- [ ] [serial] Add oracle comparisons against `full_rebuild(apply(existing_entries, edits))` at tiny capacities, including adversarial boundary shifts and work-bound fallback. r[molten.prolly.incremental]

## Phase 5: Decoded-node cache

- [ ] [serial] Extend the dataspace-access cache principles to a decoded immutable-node cache with (node reference, profile, codec version) identity and advisory retention. r[molten.prolly.cache]
- [ ] [serial] Add tests for hot-set survival under scan, retention-invariance of canonical outputs, and reachability of eviction paths at tiny capacities. r[molten.prolly.cache]

## Phase 6: Validation

- [ ] [serial] Re-measure after each optimization and record the deltas against the Phase 1 baseline; run workspace tests, strict Clippy, and `cairn validate --root .`. r[molten.prolly.metrics] r[molten.prolly.incremental]
