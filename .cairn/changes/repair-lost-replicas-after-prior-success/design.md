# F04 design

## Boundary and source

Source revision: `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.

`crates/molten-core/src/content_replication/planner.rs` selects an old verified attempt in `next_attempt`. `reuse_or_conflict` converts it into `Reuse`. `action.rs::status` counts that result. `src/content_replication/service/execution.rs::execute_actions` skips external work for reuse.

The accepted content-replication contract requires deterministic planning, exact-operation idempotency, current-epoch facts, and explicit unresolved causes. Same-epoch loss exposes a gap between historical outcome and current observation.

## Proposed decision

The pure core owns evidence classification, bounded attempt selection, action identity, and current replica accounting. Current inventory remains an explicit input. Historical success preserves its original semantic meaning only.

A fresh missing or unverified target needs a distinct repair attempt under existing limits, or an explicit unresolved decision. The shell gathers observations and executes admitted pin, fetch, verification, and durable publication steps. No new external port is needed for an internal comparison.

Exact repeats of a historical operation remain idempotent. A new reconciliation after observed loss is not that historical operation. Conflicting history, exhausted attempts, invalid verification, and stale epochs cannot inflate current replica counts. Rejection preserves durable history and current inventory.

## Compatibility and receipts

Prefer existing canonical record shapes and bounded attempt fields. Parent review must settle the distinction between an exact replay and a new repair cycle. If existing identity fields cannot express it, require an explicit versioned compatibility decision before implementation.

Update the old `corrupt_replica_repairs_and_exact_terminal_operation_reuses` expectation where it equates unverified inventory with current success. Retain a separate positive exact-replay control. Replay fixtures must retain old outcome evidence without rewriting old receipts as current availability.

Receipts bind observed verification and report unresolved repair. They cannot turn historical success into current availability or permanent durability.

## Coordination and order

Keep F04, F05, and F07 separate. Recommended order is F04 accounting, F07 deficit projection, then F05 cleanup integration. Each package can validate its own invariant without another package as a blocking dependency.

F07 owns action-free deficits and partial receipts. F05 owns residual fault-domain checks. Shared status tests must prevent double counts and cleanup from inferred historical replicas.

## Validation and ownership

The content-replication maintainers own core, shell, adapters, and docs. Start with focused existing core tests before edits. Move the named audit reproduction into normal tests without absolute includes. Add successful fresh repair, exact replay, lost target, exhausted attempts, stale history, and failed verification cases.

Controlled adapters must show fresh fetch and verification after loss, no protected effects after rejection, and live/simulation semantic parity. Run focused Octet and Clippy, workspace tests, relevant Nix checks, and all required Cairn gates. Preserve existing gate strength. This document records a plan, not completed validation.
