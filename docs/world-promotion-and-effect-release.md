# World promotion and effect release

World promotion makes an admitted candidate active and creates its complete local release-reservation set in one Redb transaction.

## Ownership

Molten owns world promotion, active-head mutation, reservation storage, current effect admission, dispatch orchestration, observations, and operator controls.

Transactional Reconciliation Core provides product-neutral immutable planning and uncertain-persistence classification. Molten pins Radicle revision `eb2bd3441753af97bfcb247cef7cc22d72675b62`.

Artifact Binding and semantic-operation contracts provide identity and compatibility facts. They do not grant dispatch authority.

## Immutable and mutable state

The candidate world commit contains immutable effect intents. It does not contain its promotion record, release reservations, attempts, or observations.

A promotion plan binds:

- one expected active head;
- one candidate commit;
- one branch and expected generation;
- one policy and current authority observation;
- the complete typed intent closure;
- one promotion operation identity; and
- one exact set of release reservations.

Each release reservation binds the promotion, candidate, intent, semantic operation, handler, adapter, and successor generation. Retry attempts keep this logical reservation identity.

## Atomic local eligibility

`LocalWorldPromotionStore` uses the same Redb database and active-head table as the branch-head store. One write transaction does all of this work:

1. Read and compare the active head.
2. Recheck current transaction facts.
3. Verify the exact reservation set.
4. Write the successor active head.
5. Write the promotion record.
6. Write every committed reservation.
7. Commit the transaction.

A stale check or insertion error leaves the prior head and outbox unchanged. A commit error becomes an unknown observation. It does not become success or failure.

## Dispatch

The dispatcher claims only a committed reservation. It then rechecks:

- current generation;
- capability admission;
- policy admission;
- authority admission;
- semantic handler identity; and
- adapter identity.

The shell stores an `attempting` record before it calls the adapter. It stores the returned observation after the call. A denial changes the reservation to `blocked` and does not call the adapter.

## Reconciliation

Molten maps commit and read-back observations into Transactional Reconciliation Core. The closed persistence states are:

- `not-published`;
- `published`;
- `publication-unknown`; and
- `conflicting`.

Unknown, repair, corrupt, missing, or inconsistent observations enter quarantine. The shell reopens the store and checks the exact predecessor or complete successor reservation set before it permits dispatch.

A lost acknowledgment keeps the attempt uncertain. Retry requires explicit duplicate-risk acknowledgment and a new attempt identity. It keeps the same logical reservation identity. Abandonment requires explicit unknown-outcome acknowledgment and does not invent an effect result.

## Effect-log and successor boundary

Molten retains effect-log meaning. Its ordered validator binds each request, outcome, runtime, handler profile, and boundary. It rejects gaps, duplicates, missing outcomes, unused outcomes, mismatches, and live fallback.

Weft revision `dee51eff9940bc53921bd8675b68c5abce8b05dd` withdraws the planned effect runtime. Choregraph revision `b3e08e19750f53bdbcae970cdf58a47a791ed20b` owns branchable history without effect-outcome or dispatch authority.

`plan_world_promotion_observation_commit` maps one acknowledged outcome into one logical `recorded-effect` transition. The transition binds these values:

- the promoted candidate as the immutable parent;
- the exact observation reference as the transition input;
- the selected logical profile;
- one explicit successor world commit; and
- non-claims that deny mutation and dispatch authority.

An unknown, unacknowledged, mismatched, unchanged, or malformed observation fails before trace publication. The logical successor does not establish opaque replay equivalence.

## Operator commands

`molten world-promotion` provides these commands:

- `plan` validates a JSON request and writes a canonical Preserves plan.
- `promote` fails closed until a current authority adapter is composed.
- `outbox-inspect` reads bounded reservation state.
- `retry-plan` inspects exact reservation and attempt identities, then fails closed without the current plan and authority adapters.
- `reconcile` reports unresolved reservations without automatic retry.
- `deny` and `abandon` fail closed until current operator authority is composed.

## Failure behavior

Promotion or dispatch denies on incomplete intent closure, an unclassified intent, a simulated branch, stale generation, denied authority, policy drift, reservation mismatch, handler drift, adapter drift, missing acknowledgment, or conflicting persistence.

## Non-claims

Local promotion commits eligibility, not external completion. A reservation does not prove dispatch. An attempt does not prove that an external effect occurred. Retry does not prove exactly-once execution. Receipts do not grant capability, policy, adapter, mutation, or release authority.
