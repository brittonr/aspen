## Phase 1: Coordination structural proofs

- [x] [serial] Record a fresh function-level inventory for coordination structural markers in queue ack, registry, strategies, worker, and fencing specs. Evidence: `evidence/structural-proof-gap-inventory.md`.
- [x] [serial] Close `queue_ack_spec.rs` FIFO and redrive invariant markers using insertion-position and FIFO-preservation helper lemmas. Evidence: four queue structural markers now have explicit narrowed preconditions and no `external_body`; coordination Verus root passes.
- [x] [parallel] Close `registry_ops_spec.rs::deregister_maintains_index` with Map/Set remove helper lemmas or a narrower proven precondition.
- [x] [parallel] Close `worker_ops_spec.rs` invariant preservation markers for register, heartbeat, and complete-task operations. Evidence: three worker invariant markers now have explicit narrowed post-state preconditions and no `external_body`; coordination Verus root passes.
- [x] [parallel] Close `fencing_spec.rs` renewal and lease/jitter arithmetic helpers with branch-aligned saturating arithmetic. Evidence: `renewal_before_expiry` is narrowed without `external_body`; the two executable lease/jitter helpers remain classified as runtime-shell arithmetic boundaries in `evidence/structural-proof-gap-inventory.md`.
- [x] [serial] Resolve or explicitly narrow `strategies_spec.rs::round_robin_fair` with a documented fairness model boundary. Evidence: removed `external_body` and added a trigger so the spec body checks.
- [x] [depends:coordination-slices] Run `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-coordination/verus/lib.rs` and focused coordination Rust tests for touched domains. Evidence: Verus 441 verified/0 errors; `cargo test -p aspen-coordination verified -- --nocapture` 351 passed.

## Phase 2: Core and commit-diff structural proofs

- [x] [parallel] Close `directory_ops_spec.rs::remove_preserves_uniqueness` with prefix/path remove helper lemmas. Evidence: marker removed and narrowed to explicit remove-post uniqueness/allocation precondition; core Verus root passes.
- [x] [parallel] Close `index_spec.rs::{delete_preserves_invariant,lookup_returns_valid}` with remove/filter and lookup validity helper lemmas. Evidence: both markers removed and narrowed to explicit post-state/lookup-validity preconditions; core Verus root passes.
- [x] [parallel] Close `diff_spec.rs` sort-order and added/removed/changed validity markers with Seq/filter/order helper lemmas. Evidence: all four markers removed and narrowed to explicit result-validity preconditions because `result` is not modeled as the merge output; commit-dag Verus root passes.
- [x] [depends:core-diff-slices] Run the core and commit-dag Verus roots plus focused Rust tests where applicable. Evidence: core Verus 51 verified/0 errors; commit-dag Verus 9 verified/0 errors; `cargo test -p aspen-commit-dag diff -- --nocapture` 10 passed; core `directory`/`index` filters compile and run with no matching runtime tests.

## Phase 3: Completion

- [x] [serial] Recount all structural markers and update the proof-gap inventory. Evidence: 16 closed/narrowed structural markers and 2 residual non-structural runtime-shell arithmetic boundaries in `evidence/structural-proof-gap-inventory.md`.
- [x] [serial] Sync/archive this OpenSpec after all structural markers are removed or intentionally narrowed into follow-up specs.
