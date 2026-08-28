## Why

A candidate world can contain effect intents that replay and simulation must not release. Moving an active branch head without durable release state can lose intended effects. Releasing effects before head promotion can expose effects from a losing branch.

Arbitrary external effects cannot share one transaction with local branch storage. Molten can atomically make admitted intents eligible and durably reserve them. Dispatch and observation must remain separate, idempotent, and reconcilable.

## What Changes

- Add canonical world-promotion plans that bind expected active head, candidate commit, intent set, authority facts, policy identity, and operation identity.
- Keep effect intents in immutable world state. Keep mutable release reservations and promotion records outside the candidate commit hash.
- Atomically compare-and-swap the active head and publish durable release reservations in one local transaction.
- Derive stable release identities from domain-separated BLAKE3 framing over candidate, intent, and promotion identities.
- Dispatch only from committed reservations through current effect, capability, policy, and adapter admission.
- Record attempts, observations, acknowledgments, uncertain outcomes, and reconciliation as distinct states.
- Reuse Transactional Reconciliation Core classifications for uncertain publication and retry decisions.
- Produce follow-up world commits or detached evidence for later effect observations without mutating the promoted commit.

## Dependencies

- `introduce-world-commit-core` and `add-world-branch-head-protocol`.
- `adopt-artifact-binding-and-semantic-effects`.
- Transactional Reconciliation Core.
- Existing Molten effect handles, effect-log validation, world replay capsules, delivery idempotency, and durable-state transactions.

Weft revision `dee51eff9940bc53921bd8675b68c5abce8b05dd` withdraws the planned product-neutral effect runtime. Choregraph revision `b3e08e19750f53bdbcae970cdf58a47a791ed20b` owns branchable history without effect or dispatch authority. Molten therefore retains effect-log validation and records acknowledged outcomes through its own world-transition trace.

## Non-Goals

- Atomic completion of email, payment, device, network, or other external effects.
- A generic exactly-once claim.
- Releasing simulated, denied, stale, or losing-branch intents.
- Treating a promotion or reservation receipt as proof that an external effect occurred.

## Impact

- **Core**: promotion plans, release sets, reservation identities, lifecycle states, reconciliation inputs, and diagnostics.
- **Shell**: atomic head-and-outbox transaction, dispatcher, attempt store, observation ingestion, and reconciliation loop.
- **Schemas**: promotion, reservation, attempt, observation, and reconciliation Preserves records.
- **Testing**: successful release plus crash, duplicate, stale-head, denied-authority, uncertain-commit, post-dispatch loss, and replay isolation cases.
