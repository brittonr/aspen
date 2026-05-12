## Phase 1: Coordination structural proofs

- [ ] [serial] Record a fresh function-level inventory for coordination structural markers in queue ack, registry, strategies, worker, and fencing specs.
- [ ] [serial] Close `queue_ack_spec.rs` FIFO and redrive invariant markers using insertion-position and FIFO-preservation helper lemmas.
- [ ] [parallel] Close `registry_ops_spec.rs::deregister_maintains_index` with Map/Set remove helper lemmas or a narrower proven precondition.
- [ ] [parallel] Close `worker_ops_spec.rs` invariant preservation markers for register, heartbeat, and complete-task operations.
- [ ] [parallel] Close `fencing_spec.rs` renewal and lease/jitter arithmetic helpers with branch-aligned saturating arithmetic.
- [ ] [serial] Resolve or explicitly narrow `strategies_spec.rs::round_robin_fair` with a documented fairness model boundary.
- [ ] [depends:coordination-slices] Run `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-coordination/verus/lib.rs` and focused coordination Rust tests for touched domains.

## Phase 2: Core and commit-diff structural proofs

- [ ] [parallel] Close `directory_ops_spec.rs::remove_preserves_uniqueness` with prefix/path remove helper lemmas.
- [ ] [parallel] Close `index_spec.rs::{delete_preserves_invariant,lookup_returns_valid}` with remove/filter and lookup validity helper lemmas.
- [ ] [parallel] Close `diff_spec.rs` sort-order and added/removed/changed validity markers with Seq/filter/order helper lemmas.
- [ ] [depends:core-diff-slices] Run the core and commit-dag Verus roots plus focused Rust tests where applicable.

## Phase 3: Completion

- [ ] [serial] Recount all structural markers and update the proof-gap inventory.
- [ ] [serial] Sync/archive this OpenSpec after all structural markers are removed or intentionally narrowed into follow-up specs.
