# World promotion current facts

`src/world_promotion/current.rs` adds a signed current-facts adapter for one
local promotion transaction. It reads the actual branch head from the promotion
store and verifies operator-supplied authority and intent-closure files before
the transaction validator sees them. Caller-provided booleans are not authority.

This is an adapter, not the public `world-promotion promote` composition. The
operator command still denies until the adapter, transaction store, receipt
port, and effect admission are composed.

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

## Non-claims

The signature binds the supplied operator facts; it does not prove the
underlying authority system, policy correctness, intent material availability,
external effect completion, or production admission. The adapter does not
perform promotion by itself.
