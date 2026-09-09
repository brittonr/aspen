# F07 design

## Boundary and source

Source revision: `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.

`crates/molten-core/src/content_replication/planner.rs::plan_missing` records `InsufficientPeers` without an action when the domain minimum is already met. `action.rs::status` retains deficits only through unsuccessful or deferred actions. `src/content_replication/service/execution.rs::execution_receipt` then classifies an empty status as complete.

The accepted content-replication contract requires explicit unresolved causes after bounded reconciliation. This change makes demand, not action presence, the status authority.

## Proposed decision

The pure core tracks per-content desired counts and current verified counts through reconciliation. It clears a deficit only after current admitted evidence satisfies that content. A partially successful plan cannot clear demand that exceeds its selected targets.

No eligible target produces an explicit unresolved cause and a non-complete plan decision. An empty action list remains valid for no-work outcomes. It is not a success signal. A defer action alone cannot substitute for correct deficit accounting.

The shell gathers observations, executes admitted actions, and publishes the core status. Receipt classification requires all demand to be satisfied, in addition to existing active-operation and failure checks. Rejected plans and adapter failures preserve prior durable state and cannot emit completion.

## Compatibility and receipts

Prefer current canonical status fields. If per-content counts require a record change, parent review must select explicit schema and replay compatibility before implementation. Do not infer per-content success from aggregate counts across different content refs.

Old receipts retain their historical bytes and identities. Corrected runs can produce partial decisions and different status references. Replay fixtures must distinguish old recorded outcomes from corrected current evaluation. No existing receipt becomes current convergence evidence without evaluation under the corrected rules.

## Coordination and order

F04 owns historical success versus fresh availability. F07 owns deficit preservation without targets. F05 owns policy-safe cleanup. Recommended integration order is F04, F07, then F05, with no circular or mandatory blocking dependency.

Shared tests must cover fresh loss with no targets, partial transfer success, and cleanup rejection. F07 must not count an F04 historical reuse as a resolved deficit.

## Validation and ownership

The content-replication maintainers own core projection, shell receipts, adapters, and docs. Run focused existing status tests before edits. Move `audit_insufficient_peers_remain_under_replicated` into normal tests.

Add enough-target success, no-target failure, fewer-target partial success, multiple content refs, denied plans, unavailable transport, and invalid verification. Controlled adapters must demonstrate partial receipts without fabricated transfer effects. Canonical status round trips and malformed inputs need positive and negative coverage.

Run focused Octet and Clippy, workspace tests, relevant Nix checks, and all required Cairn gates without weaker checks. This design claims no executed acceptance gate.
