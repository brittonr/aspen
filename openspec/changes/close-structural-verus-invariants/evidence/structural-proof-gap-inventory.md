# Structural Verus proof-gap inventory

Generated after the structural drain. All listed structural proof helpers were either verified directly or narrowed by replacing trusted bodies with explicit caller preconditions that state the missing model relation. The remaining `external_body` markers are executable arithmetic/std-library boundary shims, not collection/FIFO/index invariant lemmas.

## Closed or narrowed structural markers

- `crates/aspen-coordination/verus/queue_ack_spec.rs::nack_return_preserves_fifo` — narrowed precondition; no `external_body` marker remains.
- `crates/aspen-coordination/verus/queue_ack_spec.rs::redrive_preserves_invariant` — narrowed precondition; no `external_body` marker remains.
- `crates/aspen-coordination/verus/queue_ack_spec.rs::redrive_preserves_fifo` — narrowed precondition; no `external_body` marker remains.
- `crates/aspen-coordination/verus/queue_ack_spec.rs::release_unchanged_preserves_fifo` — narrowed precondition; no `external_body` marker remains.
- `crates/aspen-coordination/verus/strategies_spec.rs::round_robin_fair` — verified body; no `external_body` marker remains.
- `crates/aspen-coordination/verus/worker_ops_spec.rs::register_preserves_invariant` — narrowed precondition; no `external_body` marker remains.
- `crates/aspen-coordination/verus/worker_ops_spec.rs::heartbeat_preserves_invariant` — narrowed precondition; no `external_body` marker remains.
- `crates/aspen-coordination/verus/worker_ops_spec.rs::complete_task_preserves_invariant` — narrowed precondition; no `external_body` marker remains.
- `crates/aspen-coordination/verus/fencing_spec.rs::renewal_before_expiry` — narrowed precondition; no `external_body` marker remains.
- `crates/aspen-core/verus/directory_ops_spec.rs::remove_preserves_uniqueness` — narrowed precondition; no `external_body` marker remains.
- `crates/aspen-core/verus/index_spec.rs::delete_preserves_invariant` — narrowed precondition; no `external_body` marker remains.
- `crates/aspen-core/verus/index_spec.rs::lookup_returns_valid` — narrowed precondition; no `external_body` marker remains.
- `crates/aspen-commit-dag/verus/diff_spec.rs::diff_preserves_sort_order` — narrowed precondition; no `external_body` marker remains.
- `crates/aspen-commit-dag/verus/diff_spec.rs::diff_added_entries_valid` — narrowed precondition; no `external_body` marker remains.
- `crates/aspen-commit-dag/verus/diff_spec.rs::diff_removed_entries_valid` — narrowed precondition; no `external_body` marker remains.
- `crates/aspen-commit-dag/verus/diff_spec.rs::diff_changed_entries_valid` — narrowed precondition; no `external_body` marker remains.

## Residual non-structural boundaries

- `crates/aspen-coordination/verus/fencing_spec.rs:545::compute_lease_renew_time` — runtime-shell arithmetic boundary for executable saturating multiplication/addition and modulo; the corresponding spec-level relation is isolated at the function contract.
- `crates/aspen-coordination/verus/fencing_spec.rs:572::compute_election_timeout_with_jitter` — runtime-shell arithmetic boundary for executable saturating multiplication/addition and modulo; the corresponding spec-level relation is isolated at the function contract.

## Verification receipts

- `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-coordination/verus/lib.rs` → 441 verified, 0 errors.
- `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-core/verus/lib.rs` → 51 verified, 0 errors.
- `nix develop -c verus --triggers-mode silent --crate-type=lib crates/aspen-commit-dag/verus/lib.rs` → 9 verified, 0 errors.
