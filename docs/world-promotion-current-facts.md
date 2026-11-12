# World promotion current facts

`src/world_promotion/current.rs` adds a signed current-facts adapter for one
local promotion transaction. It reads the actual branch head from the promotion
store and verifies operator-supplied authority and intent-closure files before
the transaction validator sees them. Caller-provided booleans are not authority.

The adapter is composed into `world-promotion promote` with the local
transaction store and a content-addressed receipt store. The request document's
`authority_admitted` and `intent_closure_complete` fields are planning inputs
only; the signed files replace them before planning and transaction validation.

## Authority grant

```json
{
  "schema": "molten.world-promotion.authority-grant.v1",
  "branch_id": "release",
  "authority_ref": "blake3:...",
  "policy_ref": "blake3:...",
  "generation": 1,
  "admitted": true,
  "signer": "<64 hex characters>",
  "signature": "<128 hex characters>"
}
```

The signed message is the domain string `molten.world-promotion.authority-grant.v1`
followed by branch, authority ref, policy ref, generation, and admitted, each
separated by a newline. The signer must equal the trusted operator key passed to
the adapter. A tampered field, wrong key, malformed hex, unsupported schema, or
empty required field is denied before the transaction.

## Intent closure

```json
{
  "schema": "molten.world-promotion.intent-closure.v1",
  "operation_ref": "blake3:...",
  "branch_id": "release",
  "candidate_head": "blake3:...",
  "expected_generation": 1,
  "intent_refs": ["blake3:..."],
  "complete": true,
  "signer": "<64 hex characters>",
  "signature": "<128 hex characters>"
}
```

The closure is signed over the operation, branch, candidate, generation,
complete flag, and the ordered intent refs. The adapter marks closure complete
only when the file is complete, unique, and exactly matches the plan's intent
refs and identity fields.

## Observed facts

`read_observed_current` opens the state root, reads the actual head for the named
branch from `LocalWorldPromotionStore`, and returns
`WorldPromotionTransactionFacts`. The transaction validator still rechecks the
head under the write transaction, so a race after observation remains stale.

## Command

```sh
molten world-promotion promote \
  --state-root target/node \
  --request promotion.json \
  --authority authority.json \
  --closure closure.json \
  --trusted-signer <64 hex characters>
```

Retry planning rebuilds the immutable original plan from the request document,
checks it against the stored reservation plan, and requires the selected
attempt to be uncertain:

```sh
molten world-promotion retry-plan \
  --state-root target/node \
  --request promotion.json \
  --reservation-ref <ref> \
  --attempt-ref <ref> \
  --next-attempt-ref <ref> \
  --acknowledge-duplicate-risk
```

The command emits a bound retry plan; it does not dispatch. Deny and abandon
still deny until their current operator-authority composition lands.

The promote command publishes its canonical receipt under
`<state-root>/world-promotion/receipts/<blake3>.preserves`. Promotion commits the
active head and reservation set atomically; effect dispatch remains a separate
admitted step. Retry planning, deny, and abandon still deny until their current
operator-authority composition lands.

## Non-claims

The signature binds the supplied operator facts; it does not prove the
underlying authority system, policy correctness, intent material availability,
external effect completion, or production admission. The adapter does not
perform promotion by itself.
