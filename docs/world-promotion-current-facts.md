# World promotion current facts

`src/world_promotion/material/mod.rs` adds a signed current-facts adapter for one
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

## Intent material

An intent reference is the BLAKE3 content reference of the intent's canonical
material record (`molten.world-effect-intent-material.v1`, arity 5): semantic
ref, handler ref, adapter ref, and release class. The schema artifact is
`schemas/preserves-boundaries/molten-world-effect-intent-material-v1.preserves`.
`LocalIntentMaterial` stores the bytes in the node storage namespace at
`world-promotion/intent-material/<blake3>.preserves` and reads them with a
64 KiB bound. An operator publishes material with:

```sh
molten world-promotion publish-intent \
  --state-root target/node \
  --intent intent.json
```

where `intent.json` holds `semantic_ref`, `handler_ref`, `adapter_ref`, and
`release_class`. The command prints `intent_ref=<blake3 ref>` and
`authority_granted=false`: publication only makes material available.
Republishing identical material is idempotent; a stored file whose bytes differ
from its address is reported and never overwritten.

## Observed facts

`read_observed_current` opens the state root, reads the actual head for the named
branch from `LocalWorldPromotionStore`, and returns
`WorldPromotionTransactionFacts`. The transaction validator still rechecks the
head under the write transaction, so a race after observation remains stale.

When the transaction is applied, the adapter reads the material of every plan
intent. Absent material, non-canonical bytes, bytes whose content reference
differs from the intent ref, or material whose semantic, handler, adapter, or
release class differs from the request's intent all make
`intent_material_available` false, and the transaction validator denies
`IntentMaterialUnavailable` before the head or any reservation changes.

## Command

```sh
molten world-promotion promote \
  --state-root target/node \
  --request promotion.json \
  --authority authority.json \
  --closure closure.json \
  --trusted-signer <64 hex characters>
```

Resubmitting the same request replays the stored operation (`replayed=true`)
without moving the head again; a different request under the same operation ref
is denied as an identity conflict.

## Operator authority

Deny, retry planning, and abandon each require the original request document
(`--request`) and a signed authority grant (`--authority`, `--trusted-signer`)
in the format above. The pure `validate_operator_authority` decision requires
the grant to be admitted, its generation to equal the reservation's generation,
and its branch, authority ref, and policy ref to match the immutable original
plan. A missing, stale, revoked, or foreign grant is denied before any record changes.

All three operator commands rebuild the original plan from the request document.
The document's `authority_admitted` and `intent_closure_complete` fields are not
authority and are ignored here: the plan is rebuilt with the facts its committed
transaction validated and accepted only when the store holds that exact committed
promotion identity for the operation. A request that changes any identity field,
or names an operation that never committed, is denied, so no replacement logical
identity can be planned.

Retry planning requires the selected attempt to be uncertain and explicit
duplicate-risk acknowledgement:

```sh
molten world-promotion retry-plan \
  --state-root target/node \
  --request promotion.json \
  --authority authority.json \
  --trusted-signer <64 hex characters> \
  --reservation-ref <ref> \
  --attempt-ref <ref> \
  --next-attempt-ref <ref> \
  --acknowledge-duplicate-risk
```

The command emits a bound retry plan for the same logical release; it does not
dispatch.

An operator can deny a committed or blocked reservation:

```sh
molten world-promotion deny \
  --state-root target/node \
  --request promotion.json \
  --reservation-ref <ref> \
  --authority authority.json \
  --trusted-signer <64 hex characters>
```

Deny applies only before an effect attempt: committed or blocked reservations
can be denied, and repeated denial is idempotent. In-flight and uncertain
releases require reconciliation or explicit unknown-outcome acknowledgement;
deny cannot relabel them as if no effect had occurred.

An uncertain attempt can be abandoned only with the original request document,
a current grant, and explicit unknown-outcome acknowledgement:

```sh
molten world-promotion abandon \
  --state-root target/node \
  --request promotion.json \
  --authority authority.json \
  --trusted-signer <64 hex characters> \
  --attempt-ref <ref> \
  --acknowledge-unknown-outcome
```

Deny and abandon publish canonical receipts; neither proves external effect
completion. Effect dispatch remains a separate admitted step.

The promote command publishes its canonical receipt under
`<state-root>/world-promotion/receipts/<blake3>.preserves`. Promotion commits the
active head and reservation set atomically; effect dispatch remains a separate
admitted step.

## Non-claims

The signature binds the supplied operator facts; it does not prove the
underlying authority system or policy correctness, external effect completion,
or production admission. Intent material availability establishes that the
bytes named by the signed closure exist locally and match the plan; it does not
prove the effect handler or adapter will accept them. The adapter does not
perform promotion by itself.
